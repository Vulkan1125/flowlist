package dev.flowlist.android.notify

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Remove
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalHapticFeedback
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import androidx.lifecycle.lifecycleScope
import dev.flowlist.android.FlowlistApplication
import dev.flowlist.android.MainActivity
import dev.flowlist.android.data.PreferencesStore
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.ui.AppearanceMode
import dev.flowlist.android.ui.AppBackground
import dev.flowlist.android.ui.FlowlistTheme
import dev.flowlist.android.ui.reminderBackgroundFor
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import kotlin.math.roundToInt

/** The alarm-style task reminder. Shown over the lock screen by a full-screen intent. */
class ReminderActivity : ComponentActivity() {
    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        // A second alarm reuses this single-task activity; rebuild the screen
        // with the new task instead of showing the first one again.
        recreate()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O_MR1) {
            setShowWhenLocked(true)
            setTurnScreenOn(true)
        } else {
            @Suppress("DEPRECATION")
            window.addFlags(
                WindowManager.LayoutParams.FLAG_SHOW_WHEN_LOCKED or
                    WindowManager.LayoutParams.FLAG_TURN_SCREEN_ON,
            )
        }
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        // The reminder always paints a dark scrim, so keep the system bars
        // light even when the app itself is set to a light theme.
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = false
            isAppearanceLightNavigationBars = false
        }

        val taskId = intent.getStringExtra("task_id") ?: run {
            finish()
            return
        }
        val title = intent.getStringExtra("title") ?: "Task due"
        val project = intent.getStringExtra("project")
        val dueAt = intent.getLongExtra("due_at", System.currentTimeMillis())
        TaskReminders.markFired(this, taskId)
        getSystemService(android.app.NotificationManager::class.java)?.cancel(taskId.hashCode())
        val theme = ThemeStore(this).load()
        // A reminder is an alarm screen: use the dark side of the selected
        // theme family so the white type stays legible at any hour.
        val reminderTheme = theme.copy(mode = AppearanceMode.Dark)
        val preferences = PreferencesStore(this).load()
        val reminderBackground = preferences.reminderBackgroundFor(reminderTheme, true)

        setContent {
            FlowlistTheme(
                reminderTheme,
                background = reminderBackground,
                // Theme presets stay untouched; a photo still gets a veil so
                // the white reminder type remains readable.
                backgroundScrim = if (reminderBackground is AppBackground.Custom) {
                    Color.Black.copy(alpha = 0.45f)
                } else {
                    Color.Transparent
                },
            ) {
                ReminderScreen(
                    title = title,
                    project = project,
                    dueAt = dueAt,
                    initialSnoozeMinutes = preferences.snoozeMinutes,
                    onComplete = {
                        lifecycleScope.launch {
                            runCatching {
                                (application as FlowlistApplication).repository
                                    .complete(taskId, true)
                            }
                            finish()
                        }
                    },
                    onSnooze = { minutes ->
                        TaskReminders.snooze(
                            this,
                            taskId,
                            title,
                            project,
                            minutes,
                        )
                        finish()
                    },
                    onOpen = {
                        startActivity(
                            Intent(this, MainActivity::class.java).apply {
                                flags = Intent.FLAG_ACTIVITY_NEW_TASK or
                                    Intent.FLAG_ACTIVITY_CLEAR_TOP
                                putExtra("tab", "today")
                            },
                        )
                        finish()
                    },
                    onDismiss = { finish() },
                )
            }
        }
    }
}

private val reminderGreen = Color(0xFF18C66B)
private val reminderInk = Color.White

@Composable
private fun ReminderScreen(
    title: String,
    project: String?,
    dueAt: Long,
    initialSnoozeMinutes: Int,
    onComplete: () -> Unit,
    onSnooze: (Int) -> Unit,
    onOpen: () -> Unit,
    onDismiss: () -> Unit,
) {
    val dueInstant = remember(dueAt) {
        Instant.ofEpochMilli(dueAt).atZone(ZoneId.systemDefault())
    }
    val whenText = remember(dueInstant) {
        dueInstant.format(DateTimeFormatter.ofPattern("HH:mm"))
    }
    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(horizontal = 24.dp, vertical = 18.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.End,
            ) {
                DismissLink(onDismiss = onDismiss)
            }
            Spacer(modifier = Modifier.weight(1.15f))
            ReminderHero(
                title = title,
                project = project,
                whenText = whenText,
            )
            Spacer(modifier = Modifier.weight(0.85f))
            ReminderControls(
                initialSnoozeMinutes = initialSnoozeMinutes,
                onComplete = onComplete,
                onSnooze = onSnooze,
                onOpen = onOpen,
            )
        }
    }
}

@Composable
private fun DismissLink(onDismiss: () -> Unit) {
    Row(
        modifier = Modifier
            .clip(RoundedCornerShape(percent = 50))
            .clickable(role = Role.Button, onClick = onDismiss)
            .padding(horizontal = 12.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            imageVector = Icons.Rounded.Close,
            contentDescription = null,
            tint = reminderInk.copy(alpha = 0.78f),
            modifier = Modifier.size(17.dp),
        )
        Spacer(modifier = Modifier.width(5.dp))
        Text(
            text = "Dismiss",
            color = reminderInk.copy(alpha = 0.78f),
            fontSize = 14.sp,
            fontWeight = FontWeight.Medium,
        )
    }
}

@Composable
private fun ReminderHero(
    title: String,
    project: String?,
    whenText: String,
) {
    Column(
        modifier = Modifier.fillMaxWidth(),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        Icon(
            imageVector = Icons.Outlined.NotificationsActive,
            contentDescription = null,
            tint = reminderInk.copy(alpha = 0.9f),
            modifier = Modifier.size(32.dp),
        )
        Spacer(modifier = Modifier.height(20.dp))
        Text(
            text = title,
            color = reminderInk,
            fontSize = 38.sp,
            lineHeight = 42.sp,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
            maxLines = 3,
            overflow = TextOverflow.Ellipsis,
        )
        Spacer(modifier = Modifier.height(12.dp))
        Text(
            text = whenText,
            color = reminderInk.copy(alpha = 0.82f),
            fontSize = 20.sp,
            fontWeight = FontWeight.Medium,
        )
        Spacer(modifier = Modifier.height(6.dp))
        Text(
            text = project?.takeIf { it.isNotBlank() } ?: "Inbox",
            color = reminderInk.copy(alpha = 0.58f),
            fontSize = 14.sp,
            fontWeight = FontWeight.Medium,
            textAlign = TextAlign.Center,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis,
        )
    }
}

@Composable
private fun ReminderControls(
    initialSnoozeMinutes: Int,
    onComplete: () -> Unit,
    onSnooze: (Int) -> Unit,
    onOpen: () -> Unit,
) {
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .widthIn(max = 440.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        SlideToComplete(
            onComplete = onComplete,
            modifier = Modifier
                .widthIn(max = 280.dp)
                .fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(22.dp))
        SnoozeRow(
            initialMinutes = initialSnoozeMinutes,
            onSnooze = onSnooze,
        )
        Spacer(modifier = Modifier.height(6.dp))
        TextButton(
            onClick = onOpen,
            colors = ButtonDefaults.textButtonColors(
                contentColor = reminderInk.copy(alpha = 0.68f),
            ),
        ) {
            Text("Open Flowlist", fontWeight = FontWeight.Medium)
        }
    }
}

@Composable
private fun SlideToComplete(
    onComplete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val density = LocalDensity.current
    val haptics = LocalHapticFeedback.current
    val scope = rememberCoroutineScope()
    val thumbSize = 48.dp
    val thumbSizePx = with(density) { thumbSize.toPx() }
    val insetPx = with(density) { 4.dp.toPx() }
    val thumb = remember { Animatable(0f) }
    var trackWidth by remember { mutableFloatStateOf(0f) }
    var completed by remember { mutableStateOf(false) }
    val maxOffset = (trackWidth - thumbSizePx - insetPx * 2f).coerceAtLeast(0f)
    val progress = if (maxOffset > 0f) (thumb.value / maxOffset).coerceIn(0f, 1f) else 0f
    val labelAlpha = (1f - progress * 1.55f).coerceIn(0f, 1f)

    Box(
        modifier = modifier
            .height(56.dp)
            .clip(RoundedCornerShape(percent = 50))
            .background(Color.White.copy(alpha = 0.13f))
            .border(1.dp, reminderGreen.copy(alpha = 0.45f), RoundedCornerShape(percent = 50))
            .onSizeChanged { trackWidth = it.width.toFloat() }
            .semantics(mergeDescendants = true) {
                role = Role.Button
                contentDescription = "Slide to complete"
                onClick(label = "Complete") {
                    if (!completed) {
                        completed = true
                        onComplete()
                    }
                    true
                }
            }
            .pointerInput(maxOffset) {
                detectHorizontalDragGestures(
                    onDragStart = {
                        if (!completed) {
                            scope.launch { thumb.stop() }
                        }
                    },
                    onHorizontalDrag = { change, dragAmount ->
                        if (!completed) {
                            change.consume()
                            scope.launch {
                                thumb.snapTo((thumb.value + dragAmount).coerceIn(0f, maxOffset))
                            }
                        }
                    },
                    onDragEnd = {
                        if (!completed) {
                            if (maxOffset > 0f && thumb.value >= maxOffset * 0.82f) {
                                completed = true
                                scope.launch {
                                    thumb.animateTo(maxOffset, tween(120))
                                    haptics.performHapticFeedback(HapticFeedbackType.LongPress)
                                    delay(120)
                                    onComplete()
                                }
                            } else {
                                scope.launch {
                                    thumb.animateTo(
                                        targetValue = 0f,
                                        animationSpec = spring(
                                            dampingRatio = Spring.DampingRatioMediumBouncy,
                                            stiffness = Spring.StiffnessMedium,
                                        ),
                                    )
                                }
                            }
                        }
                    },
                    onDragCancel = {
                        if (!completed) {
                            scope.launch { thumb.animateTo(0f, tween(160)) }
                        }
                    },
                )
            },
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(4.dp),
        ) {
            Box(
                modifier = Modifier
                    .fillMaxHeight()
                    .width(
                        with(density) {
                            (thumb.value + thumbSizePx)
                                .coerceAtLeast(thumbSizePx)
                                .toDp()
                        },
                    )
                    .clip(RoundedCornerShape(percent = 50))
                    .background(reminderGreen),
            )
            Text(
                text = if (completed) "Completed" else "Slide to complete",
                color = reminderInk.copy(alpha = labelAlpha * 0.95f),
                fontSize = 15.sp,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.align(Alignment.Center),
            )
            Box(
                modifier = Modifier
                    .offset { IntOffset(thumb.value.roundToInt(), 0) }
                    .size(thumbSize)
                    .clip(CircleShape)
                    .background(reminderInk),
                contentAlignment = Alignment.Center,
            ) {
                Icon(
                    imageVector = Icons.Rounded.Check,
                    contentDescription = null,
                    tint = reminderGreen,
                    modifier = Modifier.size(22.dp),
                )
            }
        }
    }
}

@Composable
private fun SnoozeRow(
    initialMinutes: Int,
    onSnooze: (Int) -> Unit,
) {
    var minutes by remember(initialMinutes) {
        mutableIntStateOf(initialMinutes.coerceIn(5, 180))
    }
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .height(54.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RoundControl(
            icon = Icons.Rounded.Remove,
            description = "Less snooze time",
            onClick = { minutes = (minutes - 5).coerceAtLeast(5) },
        )
        Spacer(modifier = Modifier.width(10.dp))
        Box(
            modifier = Modifier
                .weight(1f)
                .fillMaxHeight()
                .clip(RoundedCornerShape(percent = 50))
                .background(Color.White.copy(alpha = 0.14f))
                .clickable(role = Role.Button) { onSnooze(minutes) },
            contentAlignment = Alignment.Center,
        ) {
            Text(
                text = snoozeLabel(minutes),
                color = reminderInk,
                fontSize = 16.sp,
                fontWeight = FontWeight.SemiBold,
            )
        }
        Spacer(modifier = Modifier.width(10.dp))
        RoundControl(
            icon = Icons.Rounded.Add,
            description = "More snooze time",
            onClick = { minutes = (minutes + 5).coerceAtMost(180) },
        )
    }
}

@Composable
private fun RoundControl(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    description: String,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .size(52.dp)
            .clip(CircleShape)
            .background(Color.White.copy(alpha = 0.14f))
            .clickable(role = Role.Button, onClick = onClick)
            .semantics { contentDescription = description },
        contentAlignment = Alignment.Center,
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = reminderInk.copy(alpha = 0.9f),
            modifier = Modifier.size(24.dp),
        )
    }
}

private fun snoozeLabel(minutes: Int): String = when {
    minutes == 60 -> "Snooze 1 hour"
    minutes > 60 && minutes % 60 == 0 -> "Snooze ${minutes / 60} hours"
    minutes > 60 -> "Snooze ${minutes / 60} hr ${minutes % 60} min"
    else -> "Snooze $minutes mins"
}
