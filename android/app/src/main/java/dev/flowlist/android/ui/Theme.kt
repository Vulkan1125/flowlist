package dev.flowlist.android.ui

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

data class FlowColors(
    val background: Color,
    val field: Color,
    val raised: Color,
    val ink: Color,
    val faint: Color,
    val hairline: Color,
    val success: Color,
    val warning: Color,
    val danger: Color,
    val accent: Color,
    val onAccent: Color,
)

val LightFlow = FlowColors(
    background = Color(0xFFFFFFFF),
    field = Color(0xFFF6F6F7),
    raised = Color(0xFFFFFFFF),
    ink = Color(0xFF151517),
    faint = Color(0xFF75757C),
    hairline = Color(0xFFE6E6EA),
    success = Color(0xFF2E7D5B),
    warning = Color(0xFF9A6B12),
    danger = Color(0xFFC23B32),
    accent = Color(0xFF18181A),
    onAccent = Color(0xFFFFFFFF),
)

val DarkFlow = FlowColors(
    background = Color(0xFF0B0B0C),
    field = Color(0xFF17171A),
    raised = Color(0xFF1D1D21),
    ink = Color(0xFFF1F1F3),
    faint = Color(0xFFA0A0A8),
    hairline = Color(0xFF2A2A2F),
    success = Color(0xFF73C9A2),
    warning = Color(0xFFE3B15F),
    danger = Color(0xFFFF7468),
    accent = Color(0xFFF5F5F7),
    onAccent = Color(0xFF101012),
)

val LocalFlow = staticCompositionLocalOf { LightFlow }

private val typography = Typography(
    displaySmall = TextStyle(
        fontSize = 32.sp,
        lineHeight = 36.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.6).sp,
    ),
    headlineMedium = TextStyle(
        fontSize = 26.sp,
        lineHeight = 30.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.4).sp,
    ),
    titleLarge = TextStyle(
        fontSize = 20.sp,
        lineHeight = 25.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.2).sp,
    ),
    titleMedium = TextStyle(
        fontSize = 16.sp,
        lineHeight = 21.sp,
        fontWeight = FontWeight.Medium,
    ),
    bodyLarge = TextStyle(fontSize = 16.sp, lineHeight = 23.sp),
    bodyMedium = TextStyle(fontSize = 15.sp, lineHeight = 21.sp),
    labelLarge = TextStyle(
        fontSize = 14.sp,
        lineHeight = 18.sp,
        fontWeight = FontWeight.Medium,
    ),
    labelMedium = TextStyle(fontSize = 12.sp, lineHeight = 15.sp),
    labelSmall = TextStyle(fontSize = 11.sp, lineHeight = 14.sp),
)

@Composable
fun FlowlistTheme(
    dark: Boolean = isSystemInDarkTheme(),
    content: @Composable () -> Unit,
) {
    val flow = if (dark) DarkFlow else LightFlow
    val scheme = if (dark) {
        darkColorScheme(
            primary = flow.accent,
            onPrimary = flow.onAccent,
            background = flow.background,
            onBackground = flow.ink,
            surface = flow.background,
            onSurface = flow.ink,
            surfaceVariant = flow.field,
            onSurfaceVariant = flow.faint,
            outline = flow.hairline,
            error = flow.danger,
            onError = Color.White,
        )
    } else {
        lightColorScheme(
            primary = flow.accent,
            onPrimary = flow.onAccent,
            background = flow.background,
            onBackground = flow.ink,
            surface = flow.background,
            onSurface = flow.ink,
            surfaceVariant = flow.field,
            onSurfaceVariant = flow.faint,
            outline = flow.hairline,
            error = flow.danger,
            onError = Color.White,
        )
    }
    CompositionLocalProvider(LocalFlow provides flow) {
        MaterialTheme(colorScheme = scheme, typography = typography, content = content)
    }
}

val flow: FlowColors
    @Composable get() = LocalFlow.current
