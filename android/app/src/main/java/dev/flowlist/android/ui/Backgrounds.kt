package dev.flowlist.android.ui

import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import dev.flowlist.android.data.AppPreferences
import java.io.File

/** One of the hand-tuned ambient gradients, named after the theme it suits. */
data class AmbientPreset(
    val id: String,
    val label: String,
    val light: List<Long>,
    val dark: List<Long>,
) {
    fun colors(dark: Boolean): List<Color> =
        (if (dark) this.dark else light).map(::Color)

    fun brush(dark: Boolean): Brush = Brush.linearGradient(colors(dark))
}

object AmbientBackgrounds {
    val all = listOf(
        AmbientPreset("ember", "Ember", listOf(0xFFFFE7DA, 0xFFFFC7AA), listOf(0xFF3B1712, 0xFF160708)),
        AmbientPreset("paper", "Paper", listOf(0xFFF8F2E5, 0xFFE8DDC8), listOf(0xFF27231D, 0xFF14120F)),
        AmbientPreset("sand", "Sand", listOf(0xFFF6EAD7, 0xFFE0C9A5), listOf(0xFF31291E, 0xFF17120C)),
        AmbientPreset("fog", "Fog", listOf(0xFFEAF0F5, 0xFFD0DCE8), listOf(0xFF1C242B, 0xFF0B1014)),
        AmbientPreset("latte", "Latte", listOf(0xFFF7E8D9, 0xFFE7C8B6), listOf(0xFF2B201B, 0xFF150F0D)),
        AmbientPreset("aurora", "Aurora", listOf(0xFFDFF4EF, 0xFFBCE4E8), listOf(0xFF0E2B2E, 0xFF061417)),
        AmbientPreset("sunset", "Sunset", listOf(0xFFFCE4C2, 0xFFE8B287), listOf(0xFF3B200E, 0xFF1A0C05)),
        AmbientPreset("nightshade", "Nightshade", listOf(0xFFEDE2F6, 0xFFD2C2E9), listOf(0xFF241531, 0xFF100817)),
        AmbientPreset("neon", "Neon", listOf(0xFFDFE9FF, 0xFFBFC7FF), listOf(0xFF101A3A, 0xFF060A18)),
        AmbientPreset("amber", "Amber", listOf(0xFFF7EACA, 0xFFE5CC91), listOf(0xFF2F2410, 0xFF150F06)),
        AmbientPreset("graphite", "Graphite", listOf(0xFFEAEAEA, 0xFFCECECE), listOf(0xFF252525, 0xFF101010)),
        AmbientPreset("deep", "Deep", listOf(0xFFDEE8F6, 0xFFBECDDF), listOf(0xFF0D1930, 0xFF050A14)),
    )

    fun byId(id: String): AmbientPreset? = all.firstOrNull { it.id == id }

    fun forFamily(family: Family): AmbientPreset = when (family) {
        Family.Tomato -> byId("ember")
        Family.Plain -> byId("paper")
        Family.Paper -> byId("sand")
        Family.Mist -> byId("fog")
        Family.Catppuccin -> byId("latte")
        Family.Nord -> byId("aurora")
        Family.Gruvbox -> byId("sunset")
        Family.Dracula -> byId("nightshade")
        Family.TokyoNight -> byId("neon")
        Family.Vesper -> byId("amber")
        Family.Graphite -> byId("graphite")
        Family.Midnight -> byId("deep")
    } ?: all.first()
}

sealed interface AppBackground {
    data object None : AppBackground
    data class Ambient(val preset: AmbientPreset) : AppBackground
    data class Custom(val path: String) : AppBackground

    val active: Boolean get() = this !is None
}

/** Resolve the saved choice against the current theme. */
fun AppPreferences.backgroundFor(theme: ThemeSettings, dark: Boolean): AppBackground {
    if (matchThemeBackground) {
        return AppBackground.Ambient(
            AmbientBackgrounds.forFamily(if (dark) theme.darkFamily else theme.family),
        )
    }
    customWallpaper?.takeIf { it.isNotBlank() && File(it).exists() }?.let {
        return AppBackground.Custom(it)
    }
    if (ambientBackground == "none") {
        return AppBackground.None
    }
    return AppBackground.Ambient(
        AmbientBackgrounds.byId(ambientBackground)
            ?: AmbientBackgrounds.forFamily(if (dark) theme.darkFamily else theme.family),
    )
}

/** Copy a picked image into app storage so it survives the picker's grant. */
suspend fun saveWallpaper(context: android.content.Context, uri: android.net.Uri): String? =
    kotlinx.coroutines.withContext(kotlinx.coroutines.Dispatchers.IO) {
        runCatching {
            val dir = File(context.filesDir, "backgrounds").apply { mkdirs() }
            dir.listFiles()?.forEach { it.delete() }
            val file = File(dir, "wallpaper-${System.currentTimeMillis()}.img")
            context.contentResolver.openInputStream(uri)?.use { input ->
                file.outputStream().use { output -> input.copyTo(output) }
            } ?: return@runCatching null
            file.absolutePath
        }.getOrNull()
    }
