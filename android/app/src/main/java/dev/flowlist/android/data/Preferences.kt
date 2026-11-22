package dev.flowlist.android.data

import android.content.Context
import dev.flowlist.android.ui.AccentMode
import dev.flowlist.android.ui.AccentPreset
import dev.flowlist.android.ui.AppearanceMode
import dev.flowlist.android.ui.Family
import dev.flowlist.android.ui.ThemeSettings

data class AppPreferences(
    val weekStartsMonday: Boolean = true,
    val defaultTimeMinutes: Int = 9 * 60,
    val haptics: Boolean = true,
    val swipeActions: Boolean = true,
    val startTab: String = "inbox",
    val bottomBar: List<String> = listOf("inbox", "today", "forecast", "browse"),
    val saveAllAttachments: Boolean = false,
    val forecastView: String = "month",
    val keepCompletedSubtasks: Boolean = false,
    val nearbyReminders: Boolean = true,
    /// Drawer order, by perspective key. Empty means the built-in order.
    val perspectiveOrder: List<String> = emptyList(),
)

class PreferencesStore(context: Context) {
    private val prefs = context.getSharedPreferences("flowlist.prefs", Context.MODE_PRIVATE)

    fun load(): AppPreferences = AppPreferences(
        weekStartsMonday = prefs.getBoolean("week_monday", true),
        defaultTimeMinutes = prefs.getInt("default_time", 9 * 60),
        haptics = prefs.getBoolean("haptics", true),
        swipeActions = prefs.getBoolean("swipe_actions", true),
        startTab = prefs.getString("start_tab", "inbox") ?: "inbox",
        bottomBar = (prefs.getString("bottom_bar", null) ?: "inbox|today|forecast|browse")
            .split('|')
            .filter { it.isNotBlank() }
            .take(4)
            .ifEmpty { listOf("inbox", "today", "forecast", "browse") },
        saveAllAttachments = prefs.getBoolean("save_all_attachments", false),
        forecastView = prefs.getString("forecast_view", "month") ?: "month",
        keepCompletedSubtasks = prefs.getBoolean("keep_completed_subtasks", false),
        nearbyReminders = prefs.getBoolean("nearby_reminders", true),
        perspectiveOrder = (prefs.getString("perspective_order", null) ?: "")
            .split('|')
            .filter { it.isNotBlank() },
    )

    fun save(value: AppPreferences) {
        prefs.edit()
            .putBoolean("week_monday", value.weekStartsMonday)
            .putInt("default_time", value.defaultTimeMinutes)
            .putBoolean("haptics", value.haptics)
            .putBoolean("swipe_actions", value.swipeActions)
            .putString("start_tab", value.startTab)
            .putString("bottom_bar", value.bottomBar.take(4).joinToString("|"))
            .putBoolean("save_all_attachments", value.saveAllAttachments)
            .putString("forecast_view", value.forecastView)
            .putBoolean("keep_completed_subtasks", value.keepCompletedSubtasks)
        .putBoolean("nearby_reminders", value.nearbyReminders)
            .putString("perspective_order", value.perspectiveOrder.joinToString("|"))
            .apply()
    }
}
