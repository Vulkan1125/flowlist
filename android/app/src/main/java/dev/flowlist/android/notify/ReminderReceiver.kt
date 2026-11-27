package dev.flowlist.android.notify

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.media.RingtoneManager
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import dev.flowlist.android.MainActivity
import dev.flowlist.android.R

/** Turns a scheduled task alarm into a high-importance full-screen reminder. */
class ReminderReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action != TaskReminders.ACTION) return
        val taskId = intent.getStringExtra("task_id") ?: return
        val title = intent.getStringExtra("title") ?: "Task due"
        val project = intent.getStringExtra("project")
        val dueAt = intent.getLongExtra("due_at", System.currentTimeMillis())
        TaskReminders.markFired(context, taskId)

        if (!DueNotifications.canPost(context)) return
        TaskReminders.ensureChannel(context)

        val reminder = Intent(context, ReminderActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            putExtra("task_id", taskId)
            putExtra("title", title)
            putExtra("project", project)
            putExtra("due_at", dueAt)
        }
        // Full-screen intents only take over automatically while the phone is
        // locked. Open the alarm directly too, so an unlocked phone gets the
        // same screen the Mac shows; the notification stays as the fallback
        // when the system refuses a background activity start.
        runCatching { context.startActivity(reminder) }
        val fullScreen = PendingIntent.getActivity(
            context,
            taskId.hashCode(),
            reminder,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val openApp = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("tab", "today")
        }
        val content = PendingIntent.getActivity(
            context,
            taskId.hashCode() + 1,
            openApp,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val text = when {
            project.isNullOrBlank() -> "Reminder · Inbox"
            else -> "Reminder · $project"
        }
        val notification = NotificationCompat.Builder(context, TaskReminders.CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_flowlist)
            .setContentTitle(title)
            .setContentText(text)
            .setCategory(NotificationCompat.CATEGORY_ALARM)
            .setPriority(NotificationCompat.PRIORITY_MAX)
            .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
            .setSound(RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM))
            .setWhen(dueAt)
            .setShowWhen(true)
            .setAutoCancel(true)
            .setContentIntent(content)
            .setFullScreenIntent(fullScreen, true)
            .build()
        NotificationManagerCompat.from(context).notify(taskId.hashCode(), notification)
    }
}
