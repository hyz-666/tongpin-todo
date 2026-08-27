-- Schema version 1: local-first todo replica.
-- Applied inside a single transaction by the migration executor.

CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
) STRICT;

CREATE TABLE local_identity (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    device_id BLOB NOT NULL,
    signing_public BLOB NOT NULL,
    noise_static_public BLOB NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE devices (
    device_id BLOB PRIMARY KEY,
    friendly_name TEXT,
    platform TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_success_at INTEGER
) WITHOUT ROWID;

CREATE TABLE membership_events (
    event_hash BLOB PRIMARY KEY,
    parent_hashes BLOB NOT NULL,
    kind TEXT NOT NULL,
    causal_cutoff BLOB,
    signer BLOB NOT NULL,
    signature BLOB NOT NULL
) WITHOUT ROWID;

CREATE TABLE operations (
    origin_device_id BLOB NOT NULL,
    origin_sequence INTEGER NOT NULL,
    canonical_bytes BLOB NOT NULL,
    signature BLOB,
    committed_at INTEGER NOT NULL,
    PRIMARY KEY (origin_device_id, origin_sequence)
) WITHOUT ROWID;

CREATE TABLE origin_frontiers (
    origin_device_id BLOB PRIMARY KEY,
    contiguous_sequence INTEGER NOT NULL
) WITHOUT ROWID;

CREATE TABLE origin_gaps (
    origin_device_id BLOB NOT NULL,
    start_sequence INTEGER NOT NULL,
    end_sequence INTEGER NOT NULL,
    PRIMARY KEY (origin_device_id, start_sequence)
) WITHOUT ROWID;

CREATE TABLE peer_ack_frontiers (
    peer_device_id BLOB NOT NULL,
    origin_device_id BLOB NOT NULL,
    acked_sequence INTEGER NOT NULL,
    PRIMARY KEY (peer_device_id, origin_device_id)
) WITHOUT ROWID;

CREATE TABLE entity_lifecycle (
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    generation INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    tombstone_operation BLOB,
    PRIMARY KEY (entity_type, entity_id)
) WITHOUT ROWID;

CREATE TABLE field_registers (
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    generation INTEGER NOT NULL,
    field_name TEXT NOT NULL,
    value BLOB NOT NULL,
    physical_millis INTEGER NOT NULL,
    logical INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    origin_sequence INTEGER NOT NULL,
    PRIMARY KEY (entity_type, entity_id, generation, field_name)
) WITHOUT ROWID;

CREATE TABLE tasks (
    entity_id BLOB PRIMARY KEY,
    generation INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    title TEXT,
    description TEXT,
    due_date TEXT,
    due_time TEXT,
    priority TEXT,
    list_id BLOB,
    completed INTEGER,
    completed_at INTEGER,
    rank BLOB
) WITHOUT ROWID;

CREATE TABLE subtasks (
    entity_id BLOB PRIMARY KEY,
    parent_task_id BLOB NOT NULL,
    generation INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    title TEXT,
    completed INTEGER,
    rank BLOB
) WITHOUT ROWID;

CREATE TABLE lists (
    entity_id BLOB PRIMARY KEY,
    generation INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    name TEXT,
    color TEXT,
    icon TEXT,
    rank BLOB
) WITHOUT ROWID;

CREATE TABLE tags (
    entity_id BLOB PRIMARY KEY,
    generation INTEGER NOT NULL,
    deleted INTEGER NOT NULL,
    name TEXT,
    normalized_name TEXT,
    rank BLOB
) WITHOUT ROWID;

CREATE TABLE task_tags (
    task_id BLOB NOT NULL,
    tag_id BLOB NOT NULL,
    generation INTEGER NOT NULL,
    attached INTEGER NOT NULL,
    PRIMARY KEY (task_id, tag_id, generation)
) WITHOUT ROWID;

CREATE TABLE tombstones (
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    generation INTEGER NOT NULL,
    tombstone_operation BLOB NOT NULL,
    PRIMARY KEY (entity_type, entity_id, generation)
) WITHOUT ROWID;

CREATE TABLE conflict_history (
    entity_type TEXT NOT NULL,
    entity_id BLOB NOT NULL,
    field_name TEXT NOT NULL,
    replaced_value BLOB NOT NULL,
    physical_millis INTEGER NOT NULL,
    logical INTEGER NOT NULL,
    device_id BLOB NOT NULL,
    origin_sequence INTEGER NOT NULL,
    observed_at INTEGER NOT NULL
) STRICT;

CREATE TABLE transfer_checkpoints (
    peer_device_id BLOB NOT NULL,
    transfer_id BLOB NOT NULL,
    requested_ranges BLOB NOT NULL,
    highest_acked BLOB NOT NULL,
    starting_summary BLOB NOT NULL,
    PRIMARY KEY (peer_device_id, transfer_id)
) WITHOUT ROWID;

CREATE TABLE diagnostic_events (
    id INTEGER PRIMARY KEY,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL,
    created_at INTEGER NOT NULL
) STRICT;

CREATE TABLE local_settings (
    key TEXT PRIMARY KEY,
    value BLOB NOT NULL
) WITHOUT ROWID;

-- Search and calendar indexes.
CREATE INDEX idx_tasks_due_date ON tasks(due_date);
CREATE INDEX idx_tasks_list_id ON tasks(list_id);
CREATE INDEX idx_tasks_completed ON tasks(completed);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_subtasks_parent ON subtasks(parent_task_id);
CREATE INDEX idx_tags_normalized ON tags(normalized_name);
CREATE INDEX idx_task_tags_tag ON task_tags(tag_id);
CREATE INDEX idx_operations_committed ON operations(committed_at);
