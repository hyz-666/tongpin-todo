package com.tongpin.todo.data

import com.tongpin.todo.CoreErrorCode

/**
 * Domain-level exception surfaced by [CoreRepository]. Wraps the UniFFI
 * [CoreErrorCode] so callers can react to specific failure classes without
 * depending on generated binding types.
 */
class TodoException(
    val code: CoreErrorCode,
    override val message: String,
) : Exception("code=$code, message=$message")
