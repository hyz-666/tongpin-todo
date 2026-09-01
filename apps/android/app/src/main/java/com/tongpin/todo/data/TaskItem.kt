package com.tongpin.todo.data

/**
 * UI-facing task summary. Mirrors the FFI [com.tongpin.todo.FfiTaskSummary]
 * record, decoupled from the generated binding so the Compose layer never
 * depends on UniFFI types directly.
 */
data class TaskItem(
    val id: String,
    val title: String,
    val completed: Boolean,
    val dueDate: String?,
    val priority: String,
)
