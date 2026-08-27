//! Command payloads. Commands are validated before they are dispatched.

use crate::clock::{LocalDate, LocalTime};
use crate::ids::EntityId;
use crate::model::Priority;

/// A reference to a single entity by id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EntityRef {
    pub id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateTask {
    pub title: String,
    pub description: String,
    pub due_date: Option<LocalDate>,
    pub due_time: Option<LocalTime>,
    pub priority: Priority,
    pub list_id: Option<EntityId>,
    pub tags: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskField {
    Title(String),
    Description(String),
    DueDate(Option<LocalDate>),
    DueTime(Option<LocalTime>),
    Priority(Priority),
    List(Option<EntityId>),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetTaskField {
    pub task: EntityRef,
    pub field: TaskField,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetTaskCompleted {
    pub task: EntityRef,
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestoreTask {
    pub task: EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateSubtask {
    pub parent: EntityRef,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SubtaskField {
    Title(String),
    Completed(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetSubtaskField {
    pub subtask: EntityRef,
    pub field: SubtaskField,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestoreSubtask {
    pub subtask: EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveSubtask {
    pub subtask: EntityRef,
    pub parent: EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateList {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ListField {
    Name(String),
    Color(String),
    Icon(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetListField {
    pub list: EntityRef,
    pub field: ListField,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestoreList {
    pub list: EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveList {
    pub list: EntityRef,
    pub before: Option<EntityRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CreateTag {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TagField {
    Name(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetTagField {
    pub tag: EntityRef,
    pub field: TagField,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RestoreTag {
    pub tag: EntityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MoveTask {
    pub task: EntityRef,
    pub before: Option<EntityRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SetTaskTag {
    pub task: EntityRef,
    pub tag: EntityRef,
    pub attached: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Command {
    CreateTask(CreateTask),
    SetTaskField(SetTaskField),
    SetTaskCompleted(SetTaskCompleted),
    DeleteTask(EntityRef),
    RestoreTask(RestoreTask),
    PurgeTask(EntityRef),
    CreateSubtask(CreateSubtask),
    SetSubtaskField(SetSubtaskField),
    DeleteSubtask(EntityRef),
    RestoreSubtask(RestoreSubtask),
    MoveSubtask(MoveSubtask),
    CreateList(CreateList),
    SetListField(SetListField),
    DeleteList(EntityRef),
    RestoreList(RestoreList),
    MoveList(MoveList),
    CreateTag(CreateTag),
    SetTagField(SetTagField),
    DeleteTag(EntityRef),
    RestoreTag(RestoreTag),
    MoveTask(MoveTask),
    SetTaskTag(SetTaskTag),
}
