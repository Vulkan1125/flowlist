package dev.flowlist.android.notify

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.app.NotificationManagerCompat
import dev.flowlist.android.FlowlistApplication
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

/** Handles the Dismiss and Snooze buttons on the reminder notification. */
class ReminderActionReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val action = intent.action ?: return
        if (action != ACTION_DISMISS && action != ACTION_SNOOZE) return
        val taskId = intent.getStringExtra("task_id") ?: return
        val title = intent.getStringExtra("title") ?: "Task due"
        val project = intent.getStringExtra("project")
        val minutes = intent.getIntExtra("minutes", 15)
        TaskReminders.markFired(context, taskId)
        val pending = goAsync()
        CoroutineScope(SupervisorJob() + Dispatchers.IO).launch {
            try {
                val repository = (context.applicationContext as FlowlistApplication).repository
                repository.dismissReminder(taskId)
                if (action == ACTION_SNOOZE) {
                    TaskReminders.snooze(context, taskId, title, project, minutes)
                }
            } finally {
                NotificationManagerCompat.from(context).cancel(taskId.hashCode())
                // The overlay owns its window; the activity listens for the
                // close broadcast below.
                context.stopService(Intent(context, ReminderOverlayService::class.java))
                context.sendBroadcast(
                    Intent(ACTION_CLOSE)
                        .setPackage(context.packageName)
                        .putExtra("task_id", taskId),
                )
                pending.finish()
            }
        }
    }

    companion object {
        const val ACTION_DISMISS = "dev.flowlist.android.REMIND_DISMISS"
        const val ACTION_SNOOZE = "dev.flowlist.android.REMIND_SNOOZE"
        const val ACTION_CLOSE = "dev.flowlist.android.REMIND_CLOSE"
    }
}
