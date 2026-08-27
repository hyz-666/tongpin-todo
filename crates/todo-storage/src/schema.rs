//! Schema constants.

/// Application id recorded in the database header ("TPTD").
pub const APPLICATION_ID: i32 = 0x5450_5444;

/// The latest schema version this build understands.
pub const SCHEMA_VERSION: i32 = 2;

/// Every table schema version 1 must contain.
pub const TABLES: &[&str] = &[
    "meta",
    "local_identity",
    "devices",
    "membership_events",
    "operations",
    "origin_frontiers",
    "origin_gaps",
    "peer_ack_frontiers",
    "entity_lifecycle",
    "field_registers",
    "tasks",
    "subtasks",
    "lists",
    "tags",
    "task_tags",
    "tombstones",
    "conflict_history",
    "transfer_checkpoints",
    "diagnostic_events",
    "local_settings",
];
