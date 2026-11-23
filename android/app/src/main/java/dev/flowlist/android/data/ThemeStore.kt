package dev.flowlist.android.data

import android.content.Context
import dev.flowlist.android.ui.AccentMode
import dev.flowlist.android.ui.AccentPreset
import dev.flowlist.android.ui.AppearanceMode
import dev.flowlist.android.ui.Family
import dev.flowlist.android.ui.ThemeSettings

class ThemeStore(context: Context) {
    private val prefs = context.getSharedPreferences("flowlist.theme", Context.MODE_PRIVATE)

    fun load(): ThemeSettings = ThemeSettings(
        family = Family.fromSlug(prefs.getString("family", null)),
        mode = AppearanceMode.entries.firstOrNull {
            it.name == prefs.getString("mode", null)
        } ?: AppearanceMode.System,
        accentMode = AccentMode.entries.firstOrNull {
            it.name == prefs.getString("accent_mode", null)
        } ?: AccentMode.Theme,
        customAccent = AccentPreset.fromKey(prefs.getString("accent", null)),
        amoled = prefs.getBoolean("amoled", false),
    )

    fun save(settings: ThemeSettings) {
        prefs.edit()
            .putString("family", settings.family.slug)
            .putString("mode", settings.mode.name)
            .putString("accent_mode", settings.accentMode.name)
            .putString("accent", settings.customAccent.key)
            .putBoolean("amoled", settings.amoled)
            .commit()
    }
}
