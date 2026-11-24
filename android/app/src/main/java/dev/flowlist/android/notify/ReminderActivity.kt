package dev.flowlist.android.notify

import android.content.Intent
import android.os.Build
import android.os.Bundle
import android.view.WindowManager
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import dev.flowlist.android.FlowlistApplication
import dev.flowlist.android.MainActivity
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.ui.FlowlistTheme
import dev.flowlist.android.ui.flow
import kotlinx.coroutines.launch

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

        setContent {
            FlowlistTheme(theme) {
                ReminderScreen(
                    title = title,
                    project = project,
                    dueAt = dueAt,
                    onComplete = {
                        lifecycleScope.launch {
                            runCatching {
                                (application as FlowlistApplication).repository
                                    .complete(taskId, true)
                            }
                            finish()
                        }
                    },
                    onSnooze = {
                        TaskReminders.snooze(
                            this,
                            taskId,
                            title,
                            project,
                            10,
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

@Composable
private fun ReminderScreen(
    title: String,
    project: String?,
    dueAt: Long,
    onComplete: () -> Unit,
    onSnooze: () -> Unit,
    onOpen: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val whenText = runCatching {
        java.time.Instant.ofEpochMilli(dueAt)
            .atZone(java.time.ZoneId.systemDefault())
            .format(java.time.format.DateTimeFormatter.ofPattern("HH:mm"))
    }.getOrDefault("now")
    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(colors.background)
            .padding(horizontal = 28.dp, vertical = 40.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.Center,
    ) {
        Text(
            text = "DUE NOW · $whenText",
            style = MaterialTheme.typography.labelLarge,
            color = colors.accent,
            fontWeight = FontWeight.SemiBold,
        )
        Spacer(modifier = Modifier.height(18.dp))
        Text(
            text = title,
            style = MaterialTheme.typography.displaySmall,
            color = colors.ink,
            fontWeight = FontWeight.Bold,
            textAlign = TextAlign.Center,
        )
        Spacer(modifier = Modifier.height(10.dp))
        Text(
            text = project?.takeIf { it.isNotBlank() } ?: "Inbox",
            style = MaterialTheme.typography.titleMedium,
            color = colors.faint,
        )
        Spacer(modifier = Modifier.height(38.dp))
        Button(
            onClick = onComplete,
            modifier = Modifier
                .fillMaxWidth()
                .height(58.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = colors.accent,
                contentColor = colors.onAccent,
            ),
        ) {
            Text("Complete", style = MaterialTheme.typography.titleMedium)
        }
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedButton(
            onClick = onSnooze,
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp),
        ) {
            Text("Snooze 10 minutes", color = colors.ink)
        }
        Spacer(modifier = Modifier.height(8.dp))
        TextButton(onClick = onOpen, modifier = Modifier.fillMaxWidth()) {
            Text("Open Flowlist", color = colors.accent)
        }
        TextButton(onClick = onDismiss, modifier = Modifier.fillMaxWidth()) {
            Text("Dismiss", color = colors.faint)
        }
    }
}
