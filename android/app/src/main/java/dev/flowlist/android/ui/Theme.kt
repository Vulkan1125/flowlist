package dev.flowlist.android.ui

import android.os.Build
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import coil.compose.AsyncImage
import java.io.File

enum class Family(val slug: String, val label: String, val hint: String) {
    Tomato("tomato", "Tomato", "Red on white"),
    Plain("plain", "Plain", "White / ink"),
    Paper("paper", "Paper", "Warm neutral"),
    Mist("mist", "Mist", "Cool neutral"),
    Catppuccin("catppuccin", "Catppuccin", "Latte / Mocha"),
    Nord("nord", "Nord", "Snow / polar night"),
    Gruvbox("gruvbox", "Gruvbox", "Retro warm"),
    Dracula("dracula", "Dracula", "Alucard / purple night"),
    TokyoNight("tokyo-night", "Tokyo Night", "Day / neon city"),
    Vesper("vesper", "Vesper", "Ink and amber"),
    Graphite("graphite", "Graphite", "Warm grey"),
    Midnight("midnight", "Midnight", "Deep blue");

    companion object {
        fun fromSlug(slug: String?): Family =
            if (slug == "todoist") {
                Tomato
            } else {
                entries.firstOrNull { it.slug == slug } ?: Tomato
            }
    }
}

enum class AppearanceMode { System, Light, Dark }

enum class AccentMode { Theme, Wallpaper, Custom }

enum class AccentPreset(
    val key: String,
    val label: String,
    val light: Long,
    val dark: Long,
) {
    Ink("ink", "Ink", 0xFF181818, 0xFFF2F2F2),
    Red("red", "Red", 0xFFC0392B, 0xFFFF7B72),
    Orange("orange", "Orange", 0xFFB85C00, 0xFFFFA657),
    Amber("amber", "Amber", 0xFF8A5A00, 0xFFE3B341),
    Green("green", "Green", 0xFF1B6B3A, 0xFF56D364),
    Teal("teal", "Teal", 0xFF0B6B63, 0xFF39C5BB),
    Blue("blue", "Blue", 0xFF1F5FBF, 0xFF79B8FF),
    Indigo("indigo", "Indigo", 0xFF3B4CCA, 0xFFA5B4FC),
    Violet("violet", "Violet", 0xFF6B3FA0, 0xFFC297FF),
    Pink("pink", "Pink", 0xFFA61E63, 0xFFF778BA),
    Rose("rose", "Rose", 0xFFA1283C, 0xFFFF9AA8);

    companion object {
        fun fromKey(key: String?): AccentPreset =
            entries.firstOrNull { it.key == key } ?: Red
    }
}

data class ThemeSettings(
    /// The family used when the app is light, and the separate family used
    /// when it is dark. Keeping both lets a light Paper app switch to Nord at
    /// night without changing the light palette.
    val family: Family = Family.Tomato,
    val darkFamily: Family = Family.Tomato,
    val mode: AppearanceMode = AppearanceMode.System,
    val accentMode: AccentMode = AccentMode.Theme,
    val customAccent: AccentPreset = AccentPreset.Red,
    val amoled: Boolean = false,
)

data class FlowColors(
    val dark: Boolean,
    val background: Color,
    val field: Color,
    val raised: Color,
    val ink: Color,
    val faint: Color,
    val hairline: Color,
    val success: Color,
    val warning: Color,
    val danger: Color,
    val accent: Color,
    val onAccent: Color,
)

private data class Palette(
    val background: Long,
    val field: Long,
    val raised: Long,
    val ink: Long,
    val faint: Long,
    val hairline: Long,
    val accent: Long,
    val danger: Long,
    val warning: Long,
    val success: Long,
)

private val palettes: Map<Family, Pair<Palette, Palette>> = mapOf(
    Family.Tomato to (
        Palette(0xFFFFFFFF, 0xFFF5F5F5, 0xFFFFFFFF, 0xFF202020, 0xFF808080, 0xFFEDEDED, 0xFFDC4C3E, 0xFFDC4C3E, 0xFFEB8909, 0xFF058527) to
            Palette(0xFF1F1F1F, 0xFF2B2B2B, 0xFF262626, 0xFFEEEEEE, 0xFF9E9E9E, 0xFF363636, 0xFFE8564A, 0xFFFF7066, 0xFFFF9A14, 0xFF4CC38A)
        ),
    Family.Plain to (
        Palette(0xFFFFFFFF, 0xFFF4F4F4, 0xFFFFFFFF, 0xFF0A0A0A, 0xFF707070, 0xFFEDEDED, 0xFF181818, 0xFFA61B1B, 0xFF6E4A00, 0xFF1B5F35) to
            Palette(0xFF0B0B0B, 0xFF1C1C1C, 0xFF1F1F1F, 0xFFF7F7F7, 0xFF919191, 0xFF282828, 0xFFF2F2F2, 0xFFF0B4B4, 0xFFE8C070, 0xFF9BD8AB)
        ),
    Family.Paper to (
        Palette(0xFFF7F4EE, 0xFFEFEBE3, 0xFFFFFDF9, 0xFF27241E, 0xFF746C63, 0xFFE7E2D8, 0xFF245690, 0xFF903933, 0xFF734E13, 0xFF305F3C) to
            Palette(0xFF1B1A18, 0xFF262421, 0xFF2A2926, 0xFFECE8E0, 0xFFA4A098, 0xFF302D29, 0xFF9CC5F4, 0xFFEEB3AB, 0xFFE6BB6A, 0xFF97CEA5)
        ),
    Family.Mist to (
        Palette(0xFFF2F5F7, 0xFFE8EEF2, 0xFFFCFDFF, 0xFF1E2933, 0xFF606E7C, 0xFFE0E7EC, 0xFF115F59, 0xFF8D3B36, 0xFF744D14, 0xFF255F45) to
            Palette(0xFF131922, 0xFF1D2530, 0xFF222C3B, 0xFFE6ECF5, 0xFF9AA4B0, 0xFF242E3C, 0xFFADC3FF, 0xFFF5B4B4, 0xFFEBC06A, 0xFF7ED6A5)
        ),
    Family.Catppuccin to (
        Palette(0xFFEFF1F5, 0xFFE9EBF0, 0xFFFFFFFF, 0xFF3A3C50, 0xFF66697F, 0xFFDCE0E8, 0xFF0947C5, 0xFFA60C2E, 0xFF744910, 0xFF265E1A) to
            Palette(0xFF1E1E2E, 0xFF27273A, 0xFF313244, 0xFFCED7F4, 0xFF969AAD, 0xFF313244, 0xFF94BBFA, 0xFFF5A2B8, 0xFFF9E2AF, 0xFFA6E3A1)
        ),
    Family.Nord to (
        Palette(0xFFECEFF4, 0xFFE3E8EF, 0xFFFDFEFF, 0xFF2E3440, 0xFF5E6B85, 0xFFE5E9F0, 0xFF3C5475, 0xFF8B3841, 0xFF83402B, 0xFF455934) to
            Palette(0xFF2E3440, 0xFF383F4E, 0xFF3B4252, 0xFFFCFDFD, 0xFFB1B8C7, 0xFF3B4252, 0xFFC2E0E8, 0xFFEED5D6, 0xFFF1D8A8, 0xFFD1E0C6)
        ),
    Family.Gruvbox to (
        Palette(0xFFFBF1C7, 0xFFF2E5BD, 0xFFFFF9E6, 0xFF393534, 0xFF6E6158, 0xFFE5D5B5, 0xFF69440C, 0xFF9D0006, 0xFF882E03, 0xFF514D0A) to
            Palette(0xFF282828, 0xFF32302F, 0xFF3C3836, 0xFFEDEBB8, 0xFFA9A092, 0xFF3C3836, 0xFFFEB170, 0xFFFDAEA4, 0xFFFABD2F, 0xFFC6C929)
        ),
    Family.Dracula to (
        Palette(0xFFF8F8F2, 0xFFEFEFE9, 0xFFFFFBF5, 0xFF1F1F1F, 0xFF6C664B, 0xFFE8E8E0, 0xFF583CC5, 0xFF9F2E20, 0xFF675410, 0xFF12640A) to
            Palette(0xFF282A36, 0xFF313342, 0xFF343746, 0xFFF8F8F2, 0xFF98A3C4, 0xFF343746, 0xFFD4B9FB, 0xFFFFB0B0, 0xFFF1FA8C, 0xFF50FA7B)
        ),
    Family.TokyoNight to (
        Palette(0xFFE1E2E7, 0xFFD9DBE3, 0xFFF7F8FB, 0xFF2A3048, 0xFF565C85, 0xFFD8DCE8, 0xFF0F4490, 0xFF90072F, 0xFF6D3A00, 0xFF3A4C26) to
            Palette(0xFF1A1B26, 0xFF232433, 0xFF292E42, 0xFFCAD3F7, 0xFF8E96B8, 0xFF292E42, 0xFF96B7F9, 0xFFF99CAD, 0xFFE0AF68, 0xFF9ECE6A)
        ),
    Family.Vesper to (
        Palette(0xFFFBF6EF, 0xFFF3ECE2, 0xFFFFFDFA, 0xFF1A1A1A, 0xFF746D67, 0xFFEFE7DB, 0xFF8C4007, 0xFF9A340A, 0xFF7D4A06, 0xFF0D605A) to
            Palette(0xFF101010, 0xFF1A1A1A, 0xFF1C1C1C, 0xFFFFFFFF, 0xFF999999, 0xFF1F1F1F, 0xFFFFC799, 0xFFFF9D9D, 0xFFFFC799, 0xFF99FFE4)
        ),
    Family.Graphite to (
        Palette(0xFFF1F1EF, 0xFFE8E8E4, 0xFFFFFFFF, 0xFF232322, 0xFF6A6A66, 0xFFE3E3DF, 0xFF3F536D, 0xFF893833, 0xFF6B4C14, 0xFF345A42) to
            Palette(0xFF1B1A18, 0xFF252421, 0xFF2A2926, 0xFFECE8E0, 0xFFA4A098, 0xFF302D29, 0xFF9CC5F4, 0xFFEEB3AB, 0xFFE6BB6A, 0xFF97CEA5)
        ),
    Family.Midnight to (
        Palette(0xFFEEF2F9, 0xFFE3E9F3, 0xFFFCFDFF, 0xFF1B2433, 0xFF5E6C86, 0xFFE0E7F2, 0xFF2545C6, 0xFF982E22, 0xFF754A0C, 0xFF225D45) to
            Palette(0xFF131922, 0xFF1C2430, 0xFF222C3B, 0xFFE6ECF5, 0xFF9AA4B0, 0xFF242E3C, 0xFFADC3FF, 0xFFF5B4B4, 0xFFEBC06A, 0xFF7ED6A5)
        ),
)

data class ThemePreview(val background: Color, val ink: Color, val accent: Color)

fun themePreview(family: Family, dark: Boolean): ThemePreview {
    val pair = palettes[family] ?: palettes.getValue(Family.Tomato)
    val palette = if (dark) pair.second else pair.first
    return ThemePreview(
        background = Color(palette.background),
        ink = Color(palette.ink),
        accent = Color(palette.accent),
    )
}

private val typography = Typography(
    displaySmall = TextStyle(
        fontSize = 34.sp,
        lineHeight = 38.sp,
        fontWeight = FontWeight.Bold,
        letterSpacing = (-0.8).sp,
    ),
    headlineMedium = TextStyle(
        fontSize = 26.sp,
        lineHeight = 30.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.4).sp,
    ),
    titleLarge = TextStyle(
        fontSize = 20.sp,
        lineHeight = 25.sp,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.2).sp,
    ),
    titleMedium = TextStyle(
        fontSize = 16.sp,
        lineHeight = 21.sp,
        fontWeight = FontWeight.SemiBold,
    ),
    bodyLarge = TextStyle(fontSize = 16.sp, lineHeight = 23.sp),
    bodyMedium = TextStyle(fontSize = 15.sp, lineHeight = 21.sp),
    labelLarge = TextStyle(fontSize = 14.sp, lineHeight = 18.sp, fontWeight = FontWeight.Medium),
    labelMedium = TextStyle(fontSize = 12.sp, lineHeight = 15.sp),
    labelSmall = TextStyle(fontSize = 11.sp, lineHeight = 14.sp),
)

fun themeColors(settings: ThemeSettings, dark: Boolean): FlowColors {
    val family = if (dark) settings.darkFamily else settings.family
    val pair = palettes[family] ?: palettes.getValue(Family.Tomato)
    val palette = if (dark) pair.second else pair.first
    val accent = when (settings.accentMode) {
        AccentMode.Theme -> Color(palette.accent)
        AccentMode.Custom, AccentMode.Wallpaper -> {
            Color(if (dark) settings.customAccent.dark else settings.customAccent.light)
        }
    }
    val onAccent = if (accent.luminance() > 0.55f) Color(0xFF111111) else Color.White
    val amoled = dark && settings.amoled
    // The per-family warning colours drift into brown in light mode, which
    // reads too close to danger red. Keep Tomato's orange and give every other
    // family one legible yellow/gold pair.
    val warning = if (family == Family.Tomato) {
        Color(palette.warning)
    } else {
        Color(if (dark) 0xFFFACC15 else 0xFFC69214)
    }
    val danger = if (family == Family.Tomato) {
        Color(palette.danger)
    } else {
        Color(if (dark) 0xFFFF7066 else 0xFFDC4C3E)
    }
    return FlowColors(
        dark = dark,
        background = if (amoled) Color.Black else Color(palette.background),
        field = if (amoled) Color(0xFF0C0C0C) else Color(palette.field),
        raised = if (amoled) Color(0xFF121212) else Color(palette.raised),
        ink = Color(palette.ink),
        faint = Color(palette.faint),
        hairline = if (amoled) Color(0xFF222222) else Color(palette.hairline),
        success = Color(palette.success),
        warning = warning,
        danger = danger,
        accent = accent,
        onAccent = onAccent,
    )
}

@Composable
fun FlowlistTheme(
    settings: ThemeSettings,
    background: AppBackground = AppBackground.None,
    backgroundScrim: Color? = null,
    content: @Composable () -> Unit,
) {
    val context = LocalContext.current
    val systemDark = isSystemInDarkTheme()
    val dark = when (settings.mode) {
        AppearanceMode.System -> systemDark
        AppearanceMode.Light -> false
        AppearanceMode.Dark -> true
    }
    var colors = themeColors(settings, dark)
    if (settings.accentMode == AccentMode.Wallpaper && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
        val dynamic = if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        val accent = dynamic.primary
        colors = colors.copy(
            accent = accent,
            onAccent = if (accent.luminance() > 0.55f) Color(0xFF111111) else Color.White,
        )
    }
    val baseColors = colors
    // The ambient layer gets exactly one scrim under the main content. A photo
    // needs a heavier veil than a low-contrast gradient so task text never
    // competes with the image. Every card, sheet and menu keeps the opaque
    // theme surface regardless.
    val ambientScrim = backgroundScrim ?: baseColors.background.copy(
        alpha = if (background is AppBackground.Custom) 0.45f else 0.62f,
    )
    val surfaceColors = baseColors
    val scheme = if (dark) {
        darkColorScheme(
            primary = surfaceColors.accent,
            onPrimary = surfaceColors.onAccent,
            background = surfaceColors.background,
            onBackground = surfaceColors.ink,
            surface = surfaceColors.background,
            onSurface = surfaceColors.ink,
            surfaceVariant = surfaceColors.field,
            onSurfaceVariant = surfaceColors.faint,
            outline = surfaceColors.hairline,
            error = surfaceColors.danger,
            onError = Color.White,
        )
    } else {
        lightColorScheme(
            primary = surfaceColors.accent,
            onPrimary = surfaceColors.onAccent,
            background = surfaceColors.background,
            onBackground = surfaceColors.ink,
            surface = surfaceColors.background,
            onSurface = surfaceColors.ink,
            surfaceVariant = surfaceColors.field,
            onSurfaceVariant = surfaceColors.faint,
            outline = surfaceColors.hairline,
            error = surfaceColors.danger,
            onError = Color.White,
        )
    }
    CompositionLocalProvider(LocalFlow provides surfaceColors) {
        MaterialTheme(colorScheme = scheme, typography = typography) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .background(baseColors.background),
            ) {
                when (val layer = background) {
                    AppBackground.None -> Unit
                    is AppBackground.Ambient -> Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(layer.preset.brush(dark)),
                    )
                    is AppBackground.Custom -> {
                        AsyncImage(
                            model = File(layer.path),
                            contentDescription = null,
                            contentScale = ContentScale.Crop,
                            alpha = layer.opacity,
                            modifier = Modifier.fillMaxSize(),
                        )
                    }
                }
                if (background.active) {
                    Box(
                        modifier = Modifier
                            .fillMaxSize()
                            .background(ambientScrim),
                    )
                }
                content()
            }
        }
    }
}

val flow: FlowColors
    @Composable get() = LocalFlow.current

val LocalFlow = staticCompositionLocalOf {
    FlowColors(
        dark = false,
        background = Color.White,
        field = Color(0xFFF5F5F5),
        raised = Color.White,
        ink = Color(0xFF202020),
        faint = Color(0xFF808080),
        hairline = Color(0xFFEDEDED),
        success = Color(0xFF058527),
        warning = Color(0xFFEB8909),
        danger = Color(0xFFDC4C3E),
        accent = Color(0xFFDC4C3E),
        onAccent = Color.White,
    )
}

/** Resolves the appearance for the system bar icons. */
@Composable
fun ThemeSettings.isDark(): Boolean = when (mode) {
    AppearanceMode.System -> isSystemInDarkTheme()
    AppearanceMode.Light -> false
    AppearanceMode.Dark -> true
}
