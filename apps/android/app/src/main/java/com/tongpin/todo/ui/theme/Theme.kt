package com.tongpin.todo.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable

private val LightColors = lightColorScheme(
    primary = GreenPrimary,
    secondary = GreenSecondary,
    tertiary = GreenTertiary,
    error = ErrorRed,
)

private val DarkColors = darkColorScheme(
    primary = GreenTertiary,
    secondary = GreenSecondary,
    tertiary = GreenPrimary,
    error = ErrorRed,
    background = DarkBackground,
    surface = DarkSurface,
)

@Composable
fun TongpinTodoTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    MaterialTheme(
        colorScheme = if (darkTheme) DarkColors else LightColors,
        typography = Typography,
        content = content,
    )
}
