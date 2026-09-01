package com.tongpin.todo.data

import com.tongpin.todo.FfiCommand
import com.tongpin.todo.FfiException
import com.tongpin.todo.FfiMutationReceipt
import com.tongpin.todo.FfiPage
import com.tongpin.todo.FfiTaskQuery
import com.tongpin.todo.FfiTaskSummary
import com.tongpin.todo.core.CoreSession
import java.time.LocalDate
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext

/**
 * Single write/read entry point for the UI. Wraps the [CoreSession], runs all
 * native calls on the IO dispatcher, maps UniFFI errors to [TodoException], and
 * exposes the current task list as a [StateFlow] for Compose.
 */
class CoreRepository(
    private val session: CoreSession,
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO,
) {

    private val _tasks = MutableStateFlow<List<TaskItem>>(emptyList())

    /** Current task list for the active scope. */
    val tasks: StateFlow<List<TaskItem>> = _tasks.asStateFlow()

    /** Refreshes the task list for [scope] and updates [tasks]. */
    suspend fun refresh(scope: ListScope): Result<Unit> = withIo {
        val query = FfiTaskQuery(list = scope.wire, activeOnly = scope != ListScope.Completed)
        val page = FfiPage(cursor = null, limit = PAGE_LIMIT)
        val result = session.listTasks(query, page, today())
        _tasks.value = result.items.map(FfiTaskSummary::toTaskItem)
    }

    suspend fun createTask(
        title: String,
        description: String = "",
        dueDate: String? = null,
        dueTime: String? = null,
        priority: String = "none",
        listId: String? = null,
    ): Result<FfiMutationReceipt> = withIo {
        session.dispatch(
            FfiCommand.CreateTask(
                title = title,
                description = description,
                dueDate = dueDate,
                dueTime = dueTime,
                priority = priority,
                listId = listId,
                tags = emptyList(),
            ),
        )
    }

    suspend fun setCompleted(taskId: String, completed: Boolean): Result<FfiMutationReceipt> =
        withIo { session.dispatch(FfiCommand.SetTaskCompleted(task = taskId, completed = completed)) }

    suspend fun deleteTask(taskId: String): Result<FfiMutationReceipt> =
        withIo { session.dispatch(FfiCommand.DeleteTask(task = taskId)) }

    suspend fun restoreTask(taskId: String): Result<FfiMutationReceipt> =
        withIo { session.dispatch(FfiCommand.RestoreTask(task = taskId)) }

    /** Full-text search; returns matching task ids in relevance order. */
    suspend fun searchIds(text: String): Result<List<String>> = withIo {
        session.search(text, SEARCH_LIMIT).map { it.taskId }
    }

    /** Runs [block] on the IO dispatcher and maps UniFFI errors to [TodoException]. */
    private suspend fun <T> withIo(block: suspend () -> T): Result<T> = try {
        Result.success(withContext(ioDispatcher) { block() })
    } catch (e: FfiException.Core) {
        Result.failure(TodoException(e.code, e.message))
    } catch (e: Throwable) {
        Result.failure(e)
    }

    private fun today(): String = LocalDate.now().toString()

    private fun FfiTaskSummary.toTaskItem() = TaskItem(
        id = id,
        title = title,
        completed = completed,
        dueDate = dueDate,
        priority = priority,
    )

    private companion object {
        const val PAGE_LIMIT = 200u
        const val SEARCH_LIMIT = 50u
    }
}
