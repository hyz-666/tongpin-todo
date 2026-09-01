package com.tongpin.todo.data

/**
 * Task priority levels. [wire] is the string the Rust core understands
 * (see todo-domain model::Priority); [label] is the UI display name.
 */
enum class Priority(val wire: String, val label: String) {
    None("none", "无"),
    Low("low", "低"),
    Medium("medium", "中"),
    High("high", "高");

    companion object {
        fun fromWire(wire: String): Priority =
            entries.firstOrNull { it.wire == wire } ?: None
    }
}
