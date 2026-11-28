package dev.flowlist.android.notify

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationManagerCompat

/** Turns a scheduled task alarm into a high-importance full-screen reminder. */
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != TaskReminders.ACTION) return
        val taskId = intent.getStringExtra("task_id") ?: return
        val title = intent.getStringExtra("title") ?: "Task due"
        val project = intent.getStringExtra("project")
        val dueAt = intent.getLongExtra("due_at", System.currentTimeMillis())
        TaskReminders.markFired(context, taskId)

        // A reminder card is already on screen: add this task to the batch
        // silently instead of ringing one alarm after another.
        if (ReminderOverlayService.active) {
            runCatching {
                androidx.core.content.ContextCompat.startForegroundService(
                    context,
                    Intent(context, ReminderOverlayService::class.java).apply {
                        putExtra("task_id", taskId)
                        putExtra("title", title)
                        putExtra("project", project)
                        putExtra("due_at", dueAt)
                    },
                )
            }
            return
        }

        if (!DueNotifications.canPost(context)) return
        TaskReminders.ensureChannel(context)

        val notification = buildReminderNotification(
            context,
            taskId,
            title,
            project,
            dueAt,
        )

        // Full-screen intents only take over the lock screen. If the phone is
        // unlocked and the user granted "display over other apps", a small
        // foreground service draws the same reminder above whatever is in
        // front, including YouTube.
        val keyguard = context.getSystemService(android.app.KeyguardManager::class.java)
        val canOverlay = android.provider.Settings.canDrawOverlays(context)
        if (keyguard?.isKeyguardLocked == false && canOverlay) {
            ReminderOverlayService.active = true
            runCatching {
                androidx.core.content.ContextCompat.startForegroundService(
                    context,
                    Intent(context, ReminderOverlayService::class.java).apply {
                        putExtra("task_id", taskId)
                        putExtra("title", title)
                        putExtra("project", project)
                        putExtra("due_at", dueAt)
                    },
                )
            }.onFailure {
                ReminderOverlayService.active = false
            }
        }

        NotificationManagerCompat.from(context).notify(taskId.hashCode(), notification)
    }
}
