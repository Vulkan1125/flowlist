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
    /// Take over the screen when a task comes due, not just a banner.
    val fullScreenReminders: Boolean = false,
    /// Use the ambient background that belongs to the current theme.
    val matchThemeBackground: Boolean = true,
    /// Ambient gradient id when [matchThemeBackground] is off.
    val ambientBackground: String = "ember",
    /// A user-picked image shown behind the app, when matching is off.
    val customWallpaper: String? = null,
    /// Trash swipe layout: restore_right (left deletes, right restores),
    /// restore_left, or off.
    val trashSwipe: String = "restore_right",
    /// Legacy global sort used as the default until a view has its own choice:
    /// manual, due, defer, name, priority, flagged, estimate, created or
    /// completed_at.
    val sortBy: String = "manual",
    /// Per-view sort, keyed by tab or MoreView name. Each view keeps its own.
    val viewSorts: Map<String, String> = emptyMap(),
    /// Drawer order, by perspective key. Empty means the built-in order.
    val perspectiveOrder: List<String> = emptyList(),
) {
    fun sortFor(view: String): String = viewSorts[view] ?: sortBy

    fun withSort(view: String, value: String): AppPreferences =
        copy(viewSorts = viewSorts + (view to value))
}

class PreferencesStore(context: Context) {
    private val prefs = context.getSharedPreferences("flowlist.prefs", Context.MODE_PRIVATE)

    fun load(): AppPreferences {
        val viewSorts = (prefs.getString("view_sorts", null) ?: "")
            .split('|')
            .mapNotNull { part ->
                val split = part.indexOf('=')
                if (split <= 0) {
                    null
                } else {
                    part.substring(0, split) to part.substring(split + 1)
                }
            }
            .toMap()
        return AppPreferences(
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
            fullScreenReminders = prefs.getBoolean("full_screen_reminders", false),
            matchThemeBackground = prefs.getBoolean("match_theme_background", true),
            ambientBackground = prefs.getString("ambient_background", "ember") ?: "ember",
            customWallpaper = prefs.getString("custom_wallpaper", null),
            trashSwipe = prefs.getString("trash_swipe", "restore_right") ?: "restore_right",
            sortBy = prefs.getString("sort_by", "manual") ?: "manual",
            viewSorts = viewSorts,
            perspectiveOrder = (prefs.getString("perspective_order", null) ?: "")
                .split('|')
                .filter { it.isNotBlank() },
        )
    }

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
            .putBoolean("full_screen_reminders", value.fullScreenReminders)
            .putBoolean("match_theme_background", value.matchThemeBackground)
            .putString("ambient_background", value.ambientBackground)
            .putString("custom_wallpaper", value.customWallpaper)
            .putString("trash_swipe", value.trashSwipe)
            .putString("sort_by", value.sortBy)
            .putString(
                "view_sorts",
                value.viewSorts.entries.joinToString("|") { "${it.key}=${it.value}" },
            )
            .putString("perspective_order", value.perspectiveOrder.joinToString("|"))
            .apply()
    }
}
