package com.tongpin.todo.data

/**
 * Smart-list scopes exposed by the core query layer. The [wire] value is the
 * string the Rust core understands (see todo-core ListScope); [label] is the
 * UI display name.
 */
enum class ListScope(val wire: String, val label: String) {
    Inbox("inbox", "收件箱"),
    Today("today", "今天"),
    Tomorrow("tomorrow", "明天"),
    Next7Days("next7", "未来 7 天"),
    Completed("completed", "已完成"),
    All("all", "全部"),
}
