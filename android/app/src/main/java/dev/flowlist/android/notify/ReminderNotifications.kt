package dev.flowlist.android.notify

import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.media.RingtoneManager
import androidx.core.app.NotificationCompat
import dev.flowlist.android.MainActivity
import dev.flowlist.android.R

/** The high-priority alarm notification, shared by the receiver and overlay. */
fun buildReminderNotification(
    context: Context,
    taskId: String,
    title: String,
    project: String?,
    dueAt: Long,
): android.app.Notification {
    val reminder = Intent(context, ReminderActivity::class.java).apply {
        flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        putExtra("task_id", taskId)
        putExtra("title", title)
        putExtra("project", project)
        putExtra("due_at", dueAt)
    }
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
    val text = if (project.isNullOrBlank()) "Reminder · Inbox" else "Reminder · $project"
    return NotificationCompat.Builder(context, TaskReminders.CHANNEL)
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
}
