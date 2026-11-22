package dev.flowlist.android.widget

import android.content.Context

/// One widget's choices, stored by its app-widget id so two Flowlist widgets
/// can show different perspectives and densities.
data class WidgetOptions(
    val mode: String = "today",
    val theme: String = "app",
    val limit: Int = 5,
    /// compact, default, or large. Kept as a name so old widgets read safely.
    val textSize: String = "default",
    /// Independent from the body text so a large task font does not make the
    /// title bar huge.
    val headerSize: String = "default",
    /// 30..100, the alpha of the widget card.
    val opacity: Int = 100,
    val frosted: Boolean = false,
    val showProject: Boolean = true,
    val showCompleted: Boolean = false,
    val quickAdd: Boolean = true,
) {
    val title: String
        get() = when (mode) {
            "inbox" -> "Inbox"
            "upcoming" -> "Upcoming"
            else -> "Today"
        }
}

class WidgetOptionsStore(context: Context) {
    private val prefs = context.applicationContext
        .getSharedPreferences("flowlist.widgets", Context.MODE_PRIVATE)

    fun exists(appWidgetId: Int): Boolean = prefs.contains(key(appWidgetId, "mode"))

    fun load(appWidgetId: Int): WidgetOptions = WidgetOptions(
        mode = prefs.getString(key(appWidgetId, "mode"), null) ?: "today",
        theme = prefs.getString(key(appWidgetId, "theme"), null) ?: "app",
        limit = prefs.getInt(key(appWidgetId, "limit"), 5).coerceIn(3, 8),
        textSize = prefs.getString(key(appWidgetId, "text_size"), null) ?: "default",
        headerSize = prefs.getString(key(appWidgetId, "header_size"), null) ?: "default",
        opacity = prefs.getInt(key(appWidgetId, "opacity"), 100).coerceIn(30, 100),
        frosted = prefs.getBoolean(key(appWidgetId, "frosted"), false),
        showProject = prefs.getBoolean(key(appWidgetId, "show_project"), true),
        showCompleted = prefs.getBoolean(key(appWidgetId, "show_completed"), false),
        quickAdd = prefs.getBoolean(key(appWidgetId, "quick_add"), true),
    )

    fun save(appWidgetId: Int, value: WidgetOptions) {
        prefs.edit()
            .putString(key(appWidgetId, "mode"), value.mode)
            .putString(key(appWidgetId, "theme"), value.theme)
            .putInt(key(appWidgetId, "limit"), value.limit.coerceIn(3, 8))
            .putString(key(appWidgetId, "text_size"), value.textSize)
            .putString(key(appWidgetId, "header_size"), value.headerSize)
            .putInt(key(appWidgetId, "opacity"), value.opacity.coerceIn(30, 100))
            .putBoolean(key(appWidgetId, "frosted"), value.frosted)
            .putBoolean(key(appWidgetId, "show_project"), value.showProject)
            .putBoolean(key(appWidgetId, "show_completed"), value.showCompleted)
            .putBoolean(key(appWidgetId, "quick_add"), value.quickAdd)
            .apply()
    }

    fun delete(appWidgetIds: IntArray) {
        val editor = prefs.edit()
        appWidgetIds.forEach { id ->
            listOf(
                "mode", "theme", "limit", "text_size", "header_size", "opacity",
                "frosted", "show_project", "show_completed", "quick_add",
            )
                .forEach { name -> editor.remove(key(id, name)) }
        }
        editor.apply()
    }

    private fun key(id: Int, name: String) = "widget_${id}_$name"
}
