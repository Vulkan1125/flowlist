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
)

class PreferencesStore(context: Context) {
    private val prefs = context.getSharedPreferences("flowlist.prefs", Context.MODE_PRIVATE)

    fun load(): AppPreferences = AppPreferences(
        weekStartsMonday = prefs.getBoolean("week_monday", true),
        defaultTimeMinutes = prefs.getInt("default_time", 9 * 60),
        haptics = prefs.getBoolean("haptics", true),
        swipeActions = prefs.getBoolean("swipe_actions", true),
    )

    fun save(value: AppPreferences) {
        prefs.edit()
            .putBoolean("week_monday", value.weekStartsMonday)
            .putInt("default_time", value.defaultTimeMinutes)
            .putBoolean("haptics", value.haptics)
            .putBoolean("swipe_actions", value.swipeActions)
            .apply()
    }
}
