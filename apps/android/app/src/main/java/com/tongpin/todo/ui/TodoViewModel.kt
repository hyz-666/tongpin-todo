package com.tongpin.todo.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.tongpin.todo.TodoApplication
import com.tongpin.todo.data.CoreRepository
import com.tongpin.todo.data.ListScope
import com.tongpin.todo.data.Priority
import com.tongpin.todo.data.TaskItem
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * UI state holder for the task list. Delegates reads/writes to [CoreRepository]
 * and exposes scope/loading/edit state as [StateFlow] for Compose.
 */
class TodoViewModel(application: Application) : AndroidViewModel(application) {

    private val repository: CoreRepository =
        (application as TodoApplication).container.repository

    private val _scope = MutableStateFlow(ListScope.Inbox)
    val scope: StateFlow<ListScope> = _scope.asStateFlow()

    val tasks: StateFlow<List<TaskItem>> = repository.tasks

    private val _loading = MutableStateFlow(false)
    val loading: StateFlow<Boolean> = _loading.asStateFlow()

    private val _editTarget = MutableStateFlow<EditTarget?>(null)
    val editTarget: StateFlow<EditTarget?> = _editTarget.asStateFlow()

    init {
        refresh()
    }

    fun setScope(scope: ListScope) {
        if (scope == _scope.value) return
        _scope.value = scope
        refresh()
    }

    fun onAddClick() {
        _editTarget.value = EditTarget.New
    }

    fun onEditClick(task: TaskItem) {
        _editTarget.value = EditTarget.Existing(task)
    }

    fun onDismissEdit() {
        _editTarget.value = null
    }

    fun refresh() {
        viewModelScope.launch {
            _loading.value = true
            repository.refresh(_scope.value)
            _loading.value = false
        }
    }

    fun toggleCompleted(task: TaskItem) {
        viewModelScope.launch {
            repository.setCompleted(task.id, !task.completed)
            refresh()
        }
    }

    fun deleteTask(task: TaskItem) {
        viewModelScope.launch {
            repository.deleteTask(task.id)
            refresh()
        }
    }

    /** Creates a new task from the editor's field values. */
    fun createTask(
        title: String,
        description: String,
        dueDate: String,
        dueTime: String,
        priority: Priority,
    ) {
        viewModelScope.launch {
            repository.createTask(
                title = title.trim(),
                description = description,
                dueDate = dueDate.ifBlank { null },
                dueTime = dueTime.ifBlank { null },
                priority = priority.wire,
            )
            _editTarget.value = null
            refresh()
        }
    }

    /** Persists edited fields for an existing task. */
    fun saveTask(
        task: TaskItem,
        title: String,
        description: String,
        dueDate: String,
        dueTime: String,
        priority: Priority,
    ) {
        viewModelScope.launch {
            repository.setField(task.id, "title", title.trim())
            repository.setField(task.id, "description", description)
            repository.setField(task.id, "priority", priority.wire)
            repository.setField(task.id, "due_date", dueDate)
            repository.setField(task.id, "due_time", dueTime)
            _editTarget.value = null
            refresh()
        }
    }
}

/** Target of the task editor dialog: create a new task or edit an existing one. */
sealed interface EditTarget {
    data object New : EditTarget
    data class Existing(val task: TaskItem) : EditTarget
}
