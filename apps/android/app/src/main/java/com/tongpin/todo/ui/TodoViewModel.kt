package com.tongpin.todo.ui

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.tongpin.todo.TodoApplication
import com.tongpin.todo.data.CoreRepository
import com.tongpin.todo.data.ListScope
import com.tongpin.todo.data.TaskItem
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * UI state holder for the task list. Delegates reads/writes to [CoreRepository]
 * and exposes scope/loading state as [StateFlow] for Compose.
 */
class TodoViewModel(application: Application) : AndroidViewModel(application) {

    private val repository: CoreRepository =
        (application as TodoApplication).container.repository

    private val _scope = MutableStateFlow(ListScope.Inbox)
    val scope: StateFlow<ListScope> = _scope.asStateFlow()

    val tasks: StateFlow<List<TaskItem>> = repository.tasks

    private val _loading = MutableStateFlow(false)
    val loading: StateFlow<Boolean> = _loading.asStateFlow()

    private val _showAddDialog = MutableStateFlow(false)
    val showAddDialog: StateFlow<Boolean> = _showAddDialog.asStateFlow()

    init {
        refresh()
    }

    fun setScope(scope: ListScope) {
        if (scope == _scope.value) return
        _scope.value = scope
        refresh()
    }

    fun onAddClick() {
        _showAddDialog.value = true
    }

    fun onDismissAdd() {
        _showAddDialog.value = false
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

    /** Quick-add with a title only; the full editor lives in the edit dialog. */
    fun addTask(title: String) {
        viewModelScope.launch {
            repository.createTask(title = title.trim())
            _showAddDialog.value = false
            refresh()
        }
    }
}
