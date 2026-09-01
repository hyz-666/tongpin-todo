package com.tongpin.todo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import com.tongpin.todo.ui.TaskListScreen
import com.tongpin.todo.ui.TodoViewModel
import com.tongpin.todo.ui.theme.TongpinTodoTheme

class MainActivity : ComponentActivity() {

    private val viewModel: TodoViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            TongpinTodoTheme {
                TaskListScreen(viewModel)
            }
        }
    }
}
