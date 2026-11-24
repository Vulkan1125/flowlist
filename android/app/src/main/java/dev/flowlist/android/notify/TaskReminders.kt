package dev.flowlist.android.notify

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.os.Build
import android.os.Handler
import android.os.Looper
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.FileStore
import androidx.core.app.NotificationManagerCompat
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.reminderStamp
import dev.flowlist.android.data.wantsFullscreenReminder
import java.time.LocalDate
import java.time.LocalTime
import java.time.ZoneId

/**
 * Exact, per-task reminders. When full-screen reminders are enabled the alarm
 * fires a [ReminderReceiver], which posts a high-importance notification with a
 * full-screen intent. The system shows it over the lock screen and, on most
 * phones, takes over the screen.
 */
object TaskReminders {
    const val CHANNEL = "full_screen"
    const val ACTION = "dev.flowlist.android.REMIND"
    private const val PREFS = "flowlist.notifications"
    private const val SCHEDULED = "scheduled_reminders"
    private const val HORIZON_DAYS = 30L

    fun enabled(context: Context): Boolean =
        context.getSharedPreferences("flowlist.prefs", Context.MODE_PRIVATE)
            .getBoolean("full_screen_reminders", false)

    fun setEnabled(context: Context, enabled: Boolean) {
        if (enabled) {
            ensureChannel(context)
            scheduleFromDisk(context)
        } else {
            cancelAll(context)
        }
    }

    fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            CHANNEL,
            "Full-screen reminders",
            NotificationManager.IMPORTANCE_HIGH,
        ).apply {
            description = "Takes over the screen when a task comes due."
            lockscreenVisibility = android.app.Notification.VISIBILITY_PUBLIC
            setSound(
                RingtoneManager.getDefaultUri(RingtoneManager.TYPE_ALARM),
                AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_ALARM).build(),
            )
            enableVibration(true)
            vibrationPattern = longArrayOf(0, 500, 300, 500)
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    fun scheduleFromDisk(context: Context) {
        if (!enabled(context)) return
        val snapshot = FileStore(context).loadSnapshot() ?: return
        sync(context, snapshot.db)
    }

    fun sync(context: Context, db: Db) {
        cancelAll(context)
        if (!enabled(context)) return
        ensureChannel(context)
        val now = System.currentTimeMillis()
        val horizon = now + HORIZON_DAYS * 24L * 60L * 60L * 1000L
        val ids = mutableSetOf<Int>()
        for (task in db.tasks.values) {
            if (task.completed || task.trashedAt != null) continue
            if (!db.wantsFullscreenReminder(task)) continue
            val at = reminderAt(db.reminderStamp(task)) ?: continue
            if (at <= now || at > horizon) continue
            val project = task.project?.let { db.projects[it]?.name }
            schedule(context, task.id, task.title, project, at)
            ids += requestCode(task.id)
        }
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(SCHEDULED, ids.joinToString("|"))
            .apply()
    }

    fun snooze(context: Context, taskId: String, title: String, project: String?, minutes: Int) {
        val at = System.currentTimeMillis() + minutes * 60_000L
        schedule(context, taskId, title, project, at)
        val saved = scheduled(context).toMutableSet()
        saved += requestCode(taskId)
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(SCHEDULED, saved.joinToString("|"))
            .apply()
    }

    fun sendTest(context: Context) {
        ensureChannel(context)
        val intent = intentFor(context, "test-reminder", "Flowlist test reminder", null, System.currentTimeMillis())
        Handler(Looper.getMainLooper()).postDelayed({
            ReminderReceiver().onReceive(context, intent)
        }, 2_000)
    }

    /** Open the alarm screen directly, so the background can be previewed. */
    fun preview(context: Context) {
        context.startActivity(
            Intent(context, ReminderActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                putExtra("task_id", "test-reminder")
                putExtra("title", "Full-screen reminder test")
                putExtra("project", "Flowlist")
                putExtra("due_at", System.currentTimeMillis())
            },
        )
    }

    fun markFired(context: Context, taskId: String) {
        val saved = scheduled(context).toMutableSet()
        saved.remove(requestCode(taskId))
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString(SCHEDULED, saved.joinToString("|"))
            .apply()
    }

    private fun schedule(
        context: Context,
        taskId: String,
        title: String,
        project: String?,
        at: Long,
    ) {
        if (at <= System.currentTimeMillis()) return
        val manager = context.getSystemService(AlarmManager::class.java)
        val pending = PendingIntent.getBroadcast(
            context,
            requestCode(taskId),
            intentFor(context, taskId, title, project, at),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val exact = Build.VERSION.SDK_INT < Build.VERSION_CODES.S ||
            manager.canScheduleExactAlarms()
        if (exact) {
            manager.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
        } else {
            manager.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pending)
        }
    }

    fun cancelAll(context: Context) {
        val manager = context.getSystemService(AlarmManager::class.java)
        val prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
        for (code in scheduled(context)) {
            val intent = Intent(context, ReminderReceiver::class.java).apply {
                action = ACTION
            }
            val pending = PendingIntent.getBroadcast(
                context,
                code,
                intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
            )
            manager.cancel(pending)
            pending.cancel()
            NotificationManagerCompat.from(context).cancel(code)
        }
        prefs.edit().putString(SCHEDULED, "").apply()
    }

    private fun scheduled(context: Context): Set<Int> =
        (context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getString(SCHEDULED, null) ?: "")
            .split('|')
            .mapNotNull { it.toIntOrNull() }
            .toSet()

    private fun requestCode(taskId: String): Int = taskId.hashCode() and 0x7FFFFFFF

    private fun intentFor(
        context: Context,
        taskId: String,
        title: String,
        project: String?,
        dueAt: Long,
    ): Intent = Intent(context, ReminderReceiver::class.java).apply {
        action = ACTION
        putExtra("task_id", taskId)
        putExtra("title", title)
        putExtra("project", project)
        putExtra("due_at", dueAt)
    }

    private fun reminderAt(stamp: StampDto?): Long? {
        val stamp = stamp ?: return null
        val zoned = stamp.local() ?: return null
        return if (stamp.hasTime) {
            zoned.toInstant().toEpochMilli()
        } else {
            LocalDate.from(zoned)
                .atTime(LocalTime.of(9, 0))
                .atZone(ZoneId.systemDefault())
                .toInstant()
                .toEpochMilli()
        }
    }
}
