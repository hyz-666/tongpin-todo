//! Local command dispatch into durable, signed operations.

use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use todo_domain::clock::{Hlc, LocalDate, LocalTime, UtcInstant};
use todo_domain::command::*;
use todo_domain::ids::{DeviceId, EntityId, LifecycleGeneration, OperationId};
use todo_domain::model::Priority;
use todo_domain::operation::{EntityKind, OperationPayload, VerifiedOperation};
use todo_domain::register::VersionStamp;
use todo_domain::validation::{validate_list_name, validate_tag_name, validate_title};
use todo_storage::Storage;
use todo_storage::config::StorageConfig;
use todo_storage::error::StorageError;
use todo_storage::health::{SpaceProvider, UnlimitedSpace};
use todo_storage::materialize;
use todo_storage::repository::Repository;

use crate::error::CoreError;
use crate::recovery::ReplicaState;

#[derive(Clone)]
pub struct SignatureBytes(pub Vec<u8>);

pub trait OperationSigner: Send + Sync {
    fn sign(&self, canonical: &[u8]) -> Result<SignatureBytes, CoreError>;
}

pub trait SignatureVerifier: Send + Sync {
    fn verify(
        &self,
        signer: &DeviceId,
        canonical: &[u8],
        signature: &[u8],
    ) -> Result<(), CoreError>;
}

pub struct MutationReceipt {
    pub operation_ids: Vec<OperationId>,
    pub projection_revision: u64,
    pub committed_at: UtcInstant,
    pub affected_entities: Vec<EntityId>,
}

pub struct Core {
    pub(crate) repo: Mutex<Repository>,
    pub(crate) device_id: DeviceId,
    signer: Box<dyn OperationSigner>,
    pub(crate) verifier: Box<dyn SignatureVerifier>,
    members: Mutex<HashSet<DeviceId>>,
    space: Box<dyn SpaceProvider>,
    reserve_bytes: u64,
    read_only: AtomicBool,
}

struct Spec {
    entity: EntityId,
    kind: EntityKind,
    parent: Option<EntityId>,
    new_entity: bool,
    payload: OperationPayload,
}

fn spec_set(entity: EntityId, kind: EntityKind, field: &str, value: Value) -> Spec {
    Spec {
        entity,
        kind,
        parent: None,
        new_entity: false,
        payload: OperationPayload::SetField {
            field: field.to_string(),
            value,
        },
    }
}

fn date_json(d: LocalDate) -> Value {
    Value::String(format!("{:04}-{:02}-{:02}", d.year, d.month, d.day))
}

fn time_json(t: LocalTime) -> Value {
    Value::String(format!("{:02}:{:02}", t.hour, t.minute))
}

fn priority_json(p: Priority) -> Value {
    Value::String(
        match p {
            Priority::None => "none",
            Priority::Low => "low",
            Priority::Medium => "medium",
            Priority::High => "high",
        }
        .to_string(),
    )
}

fn id_string(id: &EntityId) -> String {
    id.to_string()
}

fn command_to_specs(cmd: &Command) -> Result<Vec<Spec>, CoreError> {
    match cmd {
        Command::CreateTask(c) => {
            validate_title(&c.title)?;
            let id = EntityId::new_v7();
            let mut s = vec![
                Spec {
                    entity: id,
                    kind: EntityKind::Task,
                    parent: None,
                    new_entity: true,
                    payload: OperationPayload::SetField {
                        field: "title".into(),
                        value: Value::String(c.title.clone()),
                    },
                },
                spec_set(
                    id,
                    EntityKind::Task,
                    "description",
                    Value::String(c.description.clone()),
                ),
                spec_set(id, EntityKind::Task, "priority", priority_json(c.priority)),
                spec_set(id, EntityKind::Task, "completed", Value::Bool(false)),
            ];
            if let Some(d) = c.due_date {
                s.push(spec_set(id, EntityKind::Task, "due_date", date_json(d)));
            }
            if let Some(t) = c.due_time {
                s.push(spec_set(id, EntityKind::Task, "due_time", time_json(t)));
            }
            if let Some(l) = c.list_id {
                s.push(spec_set(
                    id,
                    EntityKind::Task,
                    "list_id",
                    Value::String(id_string(&l)),
                ));
            }
            if !c.tags.is_empty() {
                let tags: Vec<String> = c.tags.iter().map(id_string).collect();
                s.push(spec_set(
                    id,
                    EntityKind::Task,
                    "tags",
                    serde_json::json!(tags),
                ));
            }
            Ok(s)
        }
        Command::SetTaskField(c) => {
            let (field, value) = match &c.field {
                TaskField::Title(t) => {
                    validate_title(t)?;
                    ("title", Value::String(t.clone()))
                }
                TaskField::Description(d) => ("description", Value::String(d.clone())),
                TaskField::DueDate(d) => ("due_date", d.map(date_json).unwrap_or(Value::Null)),
                TaskField::DueTime(t) => ("due_time", t.map(time_json).unwrap_or(Value::Null)),
                TaskField::Priority(p) => ("priority", priority_json(*p)),
                TaskField::List(l) => (
                    "list_id",
                    l.map(|x| Value::String(id_string(&x)))
                        .unwrap_or(Value::Null),
                ),
            };
            Ok(vec![spec_set(c.task.id, EntityKind::Task, field, value)])
        }
        Command::SetTaskCompleted(c) => Ok(vec![spec_set(
            c.task.id,
            EntityKind::Task,
            "completed",
            Value::Bool(c.completed),
        )]),
        Command::DeleteTask(r) => Ok(vec![Spec {
            entity: r.id,
            kind: EntityKind::Task,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Delete,
        }]),
        Command::RestoreTask(r) => Ok(vec![Spec {
            entity: r.task.id,
            kind: EntityKind::Task,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Restore,
        }]),
        Command::PurgeTask(r) => Ok(vec![Spec {
            entity: r.id,
            kind: EntityKind::Task,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Delete,
        }]),
        Command::CreateSubtask(c) => {
            validate_title(&c.title)?;
            let id = EntityId::new_v7();
            Ok(vec![Spec {
                entity: id,
                kind: EntityKind::Subtask,
                parent: Some(c.parent.id),
                new_entity: true,
                payload: OperationPayload::SetField {
                    field: "title".into(),
                    value: Value::String(c.title.clone()),
                },
            }])
        }
        Command::SetSubtaskField(c) => {
            let (field, value) = match &c.field {
                SubtaskField::Title(t) => {
                    validate_title(t)?;
                    ("title", Value::String(t.clone()))
                }
                SubtaskField::Completed(b) => ("completed", Value::Bool(*b)),
            };
            Ok(vec![spec_set(
                c.subtask.id,
                EntityKind::Subtask,
                field,
                value,
            )])
        }
        Command::DeleteSubtask(r) => Ok(vec![Spec {
            entity: r.id,
            kind: EntityKind::Subtask,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Delete,
        }]),
        Command::RestoreSubtask(r) => Ok(vec![Spec {
            entity: r.subtask.id,
            kind: EntityKind::Subtask,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Restore,
        }]),
        Command::MoveSubtask(c) => Ok(vec![spec_set(
            c.subtask.id,
            EntityKind::Subtask,
            "parent",
            Value::String(id_string(&c.parent.id)),
        )]),
        Command::CreateList(c) => {
            validate_list_name(&c.name)?;
            let id = EntityId::new_v7();
            Ok(vec![Spec {
                entity: id,
                kind: EntityKind::List,
                parent: None,
                new_entity: true,
                payload: OperationPayload::SetField {
                    field: "name".into(),
                    value: Value::String(c.name.clone()),
                },
            }])
        }
        Command::SetListField(c) => {
            let (field, value) = match &c.field {
                ListField::Name(n) => {
                    validate_list_name(n)?;
                    ("name", Value::String(n.clone()))
                }
                ListField::Color(s) => ("color", Value::String(s.clone())),
                ListField::Icon(s) => ("icon", Value::String(s.clone())),
            };
            Ok(vec![spec_set(c.list.id, EntityKind::List, field, value)])
        }
        Command::DeleteList(r) => Ok(vec![Spec {
            entity: r.id,
            kind: EntityKind::List,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Delete,
        }]),
        Command::RestoreList(r) => Ok(vec![Spec {
            entity: r.list.id,
            kind: EntityKind::List,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Restore,
        }]),
        Command::MoveList(c) => Ok(vec![spec_set(
            c.list.id,
            EntityKind::List,
            "rank_before",
            c.before
                .map(|b| Value::String(id_string(&b.id)))
                .unwrap_or(Value::Null),
        )]),
        Command::CreateTag(c) => {
            validate_tag_name(&c.name)?;
            let id = EntityId::new_v7();
            Ok(vec![Spec {
                entity: id,
                kind: EntityKind::Tag,
                parent: None,
                new_entity: true,
                payload: OperationPayload::SetField {
                    field: "name".into(),
                    value: Value::String(c.name.clone()),
                },
            }])
        }
        Command::SetTagField(c) => {
            let TagField::Name(n) = &c.field;
            validate_tag_name(n)?;
            Ok(vec![spec_set(
                c.tag.id,
                EntityKind::Tag,
                "name",
                Value::String(n.clone()),
            )])
        }
        Command::DeleteTag(r) => Ok(vec![Spec {
            entity: r.id,
            kind: EntityKind::Tag,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Delete,
        }]),
        Command::RestoreTag(r) => Ok(vec![Spec {
            entity: r.tag.id,
            kind: EntityKind::Tag,
            parent: None,
            new_entity: false,
            payload: OperationPayload::Restore,
        }]),
        Command::MoveTask(c) => Ok(vec![spec_set(
            c.task.id,
            EntityKind::Task,
            "rank_before",
            c.before
                .map(|b| Value::String(id_string(&b.id)))
                .unwrap_or(Value::Null),
        )]),
        Command::SetTaskTag(c) => Ok(vec![spec_set(
            c.task.id,
            EntityKind::Task,
            &format!("tag:{}", c.tag.id),
            Value::Bool(c.attached),
        )]),
    }
}

impl Core {
    pub fn open(
        config: StorageConfig,
        device_id: DeviceId,
        signer: Box<dyn OperationSigner>,
        verifier: Box<dyn SignatureVerifier>,
    ) -> Result<Self, CoreError> {
        Self::open_with_space(
            config,
            device_id,
            signer,
            verifier,
            Box::new(UnlimitedSpace),
            0,
        )
    }

    pub fn open_with_space(
        config: StorageConfig,
        device_id: DeviceId,
        signer: Box<dyn OperationSigner>,
        verifier: Box<dyn SignatureVerifier>,
        space: Box<dyn SpaceProvider>,
        reserve_bytes: u64,
    ) -> Result<Self, CoreError> {
        let storage = Storage::open(config)?;
        let repo = Repository::new(storage.conn);
        let mut members = HashSet::new();
        members.insert(device_id);
        Ok(Self {
            repo: Mutex::new(repo),
            device_id,
            signer,
            verifier,
            members: Mutex::new(members),
            space,
            reserve_bytes,
            read_only: AtomicBool::new(false),
        })
    }

    pub fn add_member(&self, device: DeviceId) {
        self.members.lock().unwrap().insert(device);
    }

    fn check_space(&self) -> Result<(), CoreError> {
        if self.space.available_bytes() < self.reserve_bytes {
            self.read_only.store(true, Ordering::SeqCst);
            return Err(CoreError::ReadOnlyLowSpace);
        }
        Ok(())
    }

    pub fn replica_state(&self) -> ReplicaState {
        if self.read_only.load(Ordering::SeqCst) {
            ReplicaState::ReadOnlyLowSpace
        } else {
            ReplicaState::Ready
        }
    }

    pub fn note_space_recovered(&self) {
        self.read_only.store(false, Ordering::SeqCst);
    }

    pub fn dispatch(&self, command: Command) -> Result<MutationReceipt, CoreError> {
        self.check_space()?;
        let mut repo = self.repo.lock().unwrap();
        let specs = command_to_specs(&command)?;
        let sequence = Repository::read_frontier(&repo.conn, &self.device_id)?.unwrap_or(0);
        let last_hlc = read_hlc(&repo.conn)?;
        let now = now_millis();

        let mut sequence = sequence;
        let mut hlc = last_hlc;
        let mut signed = Vec::new();
        for spec in &specs {
            let generation = if spec.new_entity {
                LifecycleGeneration(1)
            } else {
                LifecycleGeneration(Repository::read_lifecycle(&repo.conn, &spec.entity)?.0)
            };
            let next_sequence = sequence
                .checked_add(1)
                .ok_or(CoreError::SequenceExhausted)?;
            let stamp = VersionStamp {
                generation,
                hlc: hlc.tick(now)?,
                device: self.device_id,
                operation: OperationId::new(self.device_id, next_sequence),
            };
            let op = VerifiedOperation {
                entity: spec.entity,
                kind: spec.kind,
                parent: spec.parent,
                stamp,
                payload: spec.payload.clone(),
            };
            let canonical = serde_json::to_vec(&op)
                .map_err(|e| CoreError::InvalidCommand(format!("serialize: {e}")))?;
            let signature = self.signer.sign(&canonical)?;
            let next_hlc = op.stamp.hlc;
            signed.push((op, canonical, signature));
            sequence = next_sequence;
            hlc = next_hlc;
        }

        let tx = repo.conn.transaction()?;
        let committed_at = now;
        let mut operation_ids = Vec::new();
        let mut affected = Vec::new();
        for (op, canonical, signature) in &signed {
            let inserted = Repository::insert_operation(
                &tx,
                &self.device_id,
                op.stamp.operation.sequence,
                canonical,
                Some(&signature.0),
                committed_at,
            )?;
            if inserted {
                materialize::apply(&tx, op)?;
            }
            operation_ids.push(op.stamp.operation);
            affected.push(op.entity);
        }
        Repository::upsert_frontier(&tx, &self.device_id, sequence)?;
        write_hlc(&tx, hlc)?;
        let revision = Repository::projection_revision(&tx)? + 1;
        Repository::bump_revision(&tx, revision)?;
        tx.commit()?;

        Ok(MutationReceipt {
            operation_ids,
            projection_revision: revision,
            committed_at: UtcInstant::from_millis(committed_at),
            affected_entities: affected,
        })
    }

    pub fn field(&self, entity: &EntityId, field: &str) -> Result<Option<Value>, CoreError> {
        let repo = self.repo.lock().unwrap();
        Ok(Repository::read_field(&repo.conn, entity, field)?)
    }

    pub fn is_deleted(&self, entity: &EntityId) -> bool {
        let repo = self.repo.lock().unwrap();
        Repository::is_deleted(&repo.conn, entity).unwrap_or(false)
    }

    pub fn parent_of(&self, entity: &EntityId) -> Result<Option<EntityId>, CoreError> {
        let repo = self.repo.lock().unwrap();
        Ok(Repository::read_parent(&repo.conn, entity)?)
    }

    pub fn count_entities(&self) -> usize {
        let repo = self.repo.lock().unwrap();
        Repository::count_entities(&repo.conn).unwrap_or(0)
    }

    pub fn verifier(&self) -> &dyn SignatureVerifier {
        self.verifier.as_ref()
    }

    pub fn is_member(&self, device: &DeviceId) -> bool {
        self.members.lock().unwrap().contains(device)
    }

    pub fn repo(&self) -> &Mutex<Repository> {
        &self.repo
    }

    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn read_hlc(conn: &rusqlite::Connection) -> Result<Hlc, StorageError> {
    let physical = read_meta(conn, "hlc_physical")?.unwrap_or(0);
    let logical = read_meta(conn, "hlc_logical")?.unwrap_or(0);
    Ok(Hlc::new(physical, logical as u32))
}

fn write_hlc(conn: &rusqlite::Connection, hlc: Hlc) -> Result<(), StorageError> {
    write_meta(conn, "hlc_physical", hlc.physical_millis)?;
    write_meta(conn, "hlc_logical", hlc.logical as i64)
}

fn read_meta(conn: &rusqlite::Connection, key: &str) -> Result<Option<i64>, StorageError> {
    match conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    }) {
        Ok(v) => Ok(v.parse().ok()),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

fn write_meta(conn: &rusqlite::Connection, key: &str, value: i64) -> Result<(), StorageError> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        rusqlite::params![key, value.to_string()],
    )?;
    Ok(())
}
