package dev.flowlist.android.notify

import dev.flowlist.android.data.Repository
import kotlinx.coroutines.withTimeoutOrNull

/// True when the Mac already completed, trashed or dismissed this reminder.
///
/// The alarm is scheduled from the last synced snapshot, so a Mac action can
/// leave it to fire after the fact. The shared dismissal list is written by
/// every Mac full-screen action, so an alarm that fires later can stand down
/// without waiting for a full sync. Offline (or a slow server) keeps the local
/// answer and shows the reminder.
internal suspend fun reminderAlreadyHandled(repository: Repository, taskId: String): Boolean {
    // The Settings preview is not a real task.
    if (taskId == "test-reminder") return false
    val task = repository.state.value.snapshot?.db?.tasks?.get(taskId)
    if (task != null && (task.completed || task.trashedAt != null)) return true
    val dismissed = withTimeoutOrNull(1_500) {
        runCatching { repository.dismissedReminders() }.getOrDefault(emptySet())
    }.orEmpty()
    return taskId in dismissed
}
