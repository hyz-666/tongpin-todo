package com.tongpin.todo.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.FilterChip
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.tongpin.todo.data.Priority
import com.tongpin.todo.data.TaskItem

/**
 * Full task editor used for both creation and editing. On edit it pre-populates
 * fields from the existing task; due date/time are free-text (YYYY-MM-DD and
 * HH:MM) matching the core's string parsing.
 */
@Composable
fun TaskEditDialog(
    target: EditTarget,
    onConfirm: (title: String, description: String, dueDate: String, dueTime: String, priority: Priority) -> Unit,
    onDismiss: () -> Unit,
) {
    val existing: TaskItem? = (target as? EditTarget.Existing)?.task

    var title by remember { mutableStateOf(existing?.title.orEmpty()) }
    var description by remember { mutableStateOf("") }
    var dueDate by remember { mutableStateOf(existing?.dueDate.orEmpty()) }
    var dueTime by remember { mutableStateOf("") }
    var priority by remember { mutableStateOf(existing?.priority?.let(Priority::fromWire) ?: Priority.None) }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (existing == null) "新建任务" else "编辑任务") },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = title,
                    onValueChange = { title = it },
                    singleLine = true,
                    label = { Text("标题") },
                    modifier = Modifier.fillMaxWidth(),
                )
                OutlinedTextField(
                    value = description,
                    onValueChange = { description = it },
                    label = { Text("描述") },
                    minLines = 2,
                    modifier = Modifier.fillMaxWidth(),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = dueDate,
                        onValueChange = { dueDate = it },
                        singleLine = true,
                        label = { Text("截止日期 (YYYY-MM-DD)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Ascii),
                        modifier = Modifier.weight(1f),
                    )
                    OutlinedTextField(
                        value = dueTime,
                        onValueChange = { dueTime = it },
                        singleLine = true,
                        label = { Text("时间 (HH:MM)") },
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Ascii),
                        modifier = Modifier.width(120.dp),
                    )
                }
                PrioritySelector(selected = priority, onSelect = { priority = it })
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onConfirm(title, description, dueDate, dueTime, priority) },
                enabled = title.isNotBlank(),
            ) {
                Text(if (existing == null) "添加" else "保存")
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("取消") }
        },
    )
}

@Composable
private fun PrioritySelector(selected: Priority, onSelect: (Priority) -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        Priority.entries.forEach { priority ->
            FilterChip(
                selected = priority == selected,
                onClick = { onSelect(priority) },
                label = { Text(priority.label) },
            )
        }
    }
}
