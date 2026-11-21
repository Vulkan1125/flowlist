package dev.flowlist.android.icon

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager

enum class AppIcon(
    val key: String,
    val label: String,
    val alias: String,
    val color: Long,
) {
    Red("red", "Red", ".IconRed", 0xFFDC4C3E),
    Blue("blue", "Blue", ".IconBlue", 0xFF3D8CD6),
    Green("green", "Green", ".IconGreen", 0xFF2E8B57),
    Orange("orange", "Orange", ".IconOrange", 0xFFE07A2F),
    Purple("purple", "Purple", ".IconPurple", 0xFF7B5CC7),
    Graphite("graphite", "Graphite", ".IconGraphite", 0xFF3E3E42);

    companion object {
        fun fromKey(key: String?): AppIcon = entries.firstOrNull { it.key == key } ?: Red
    }
}

object AppIcons {
    private const val PREFS = "flowlist.icons"

    fun current(context: Context): AppIcon {
        // The saved choice is authoritative; component state only helps when a
        // package update resets it.
        val stored = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .getString("icon", null)
        if (stored != null) return AppIcon.fromKey(stored)
        val manager = context.packageManager
        for (icon in AppIcon.entries) {
            val state = manager.getComponentEnabledSetting(component(context, icon))
            if (state == PackageManager.COMPONENT_ENABLED_STATE_ENABLED) return icon
        }
        return AppIcon.Red
    }

    fun apply(context: Context, icon: AppIcon) {
        val manager = context.packageManager
        for (candidate in AppIcon.entries) {
            manager.setComponentEnabledSetting(
                component(context, candidate),
                if (candidate == icon) {
                    PackageManager.COMPONENT_ENABLED_STATE_ENABLED
                } else {
                    PackageManager.COMPONENT_ENABLED_STATE_DISABLED
                },
                PackageManager.DONT_KILL_APP,
            )
        }
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit()
            .putString("icon", icon.key)
            .apply()
    }

    private fun component(context: Context, icon: AppIcon): ComponentName =
        ComponentName(context.packageName, context.packageName + icon.alias)
}
