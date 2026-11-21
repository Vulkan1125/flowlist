package dev.flowlist.android.notify

import android.Manifest
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.work.CoroutineWorker
import androidx.work.ExistingPeriodicWorkPolicy
import androidx.work.PeriodicWorkRequestBuilder
import androidx.work.WorkManager
import androidx.work.WorkerParameters
import dev.flowlist.android.MainActivity
import dev.flowlist.android.R
import dev.flowlist.android.data.FileStore
import dev.flowlist.android.data.today
import java.time.LocalDate
import java.util.concurrent.TimeUnit

object DueNotifications {
    const val CHANNEL = "due"
    private const val WORK = "flowlist-due"
    private const val PREFS = "flowlist.notifications"

    fun enabled(context: Context): Boolean =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean("due", false)

    fun setEnabled(context: Context, enabled: Boolean) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putBoolean("due", enabled)
            .apply()
        if (enabled) {
            ensureChannel(context)
            schedule(context)
        } else {
            cancel(context)
        }
    }

    fun ensureChannel(context: Context) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
        val channel = NotificationChannel(
            CHANNEL,
            "Due tasks",
            NotificationManager.IMPORTANCE_DEFAULT,
        ).apply {
            description = "A quiet summary when tasks are due today."
        }
        context.getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    fun schedule(context: Context) {
        val request = PeriodicWorkRequestBuilder<DueWorker>(2, TimeUnit.HOURS)
            .setInitialDelay(5, TimeUnit.MINUTES)
            .build()
        WorkManager.getInstance(context).enqueueUniquePeriodicWork(
            WORK,
            ExistingPeriodicWorkPolicy.UPDATE,
            request,
        )
    }

    fun cancel(context: Context) {
        WorkManager.getInstance(context).cancelUniqueWork(WORK)
    }

    fun sendTest(context: Context) {
        if (!canPost(context)) return
        ensureChannel(context)
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("tab", "today")
        }
        val pending = PendingIntent.getActivity(
            context,
            5,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(context, CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_flowlist)
            .setContentTitle("Flowlist reminders are on")
            .setContentText("You will get a quiet summary when tasks are due.")
            .setContentIntent(pending)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(5, notification)
    }

    fun canPost(context: Context): Boolean =
        Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
}

class DueWorker(
    appContext: Context,
    parameters: WorkerParameters,
) : CoroutineWorker(appContext, parameters) {
    override suspend fun doWork(): Result {
        val context = applicationContext
        if (!DueNotifications.enabled(context) || !DueNotifications.canPost(context)) {
            return Result.success()
        }
        val snapshot = FileStore(context).loadSnapshot() ?: return Result.success()
        val due = snapshot.db.today()
        if (due.isEmpty()) return Result.success()

        val today = LocalDate.now()
        val overdue = due.count { it.due?.local()?.toLocalDate()?.isBefore(today) == true }
        val count = due.size
        val prefs = context.getSharedPreferences("flowlist.notifications", Context.MODE_PRIVATE)
        val stamp = "$today:$count:${snapshot.revision}"
        if (prefs.getString("last", null) == stamp) return Result.success()
        prefs.edit().putString("last", stamp).apply()

        val summary = when {
            overdue > 0 && overdue < count -> "$overdue overdue · ${count - overdue} due today"
            overdue == count && overdue == 1 -> "1 task overdue"
            overdue == count -> "$overdue tasks overdue"
            count == 1 -> "1 task due today"
            else -> "$count tasks due today"
        }
        val details = due.take(4).joinToString("\n") { "• ${it.title}" }
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("tab", "today")
        }
        val pending = PendingIntent.getActivity(
            context,
            3,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
        val notification = NotificationCompat.Builder(context, DueNotifications.CHANNEL)
            .setSmallIcon(R.drawable.ic_stat_flowlist)
            .setContentTitle(summary)
            .setContentText(details.replace("\n", " · "))
            .setStyle(NotificationCompat.BigTextStyle().bigText(details))
            .setContentIntent(pending)
            .setAutoCancel(true)
            .setOnlyAlertOnce(true)
            .build()
        NotificationManagerCompat.from(context).notify(4, notification)
        return Result.success()
    }
}
