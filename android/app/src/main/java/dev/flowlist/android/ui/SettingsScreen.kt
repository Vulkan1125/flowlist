package dev.flowlist.android.ui

import android.Manifest
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.ArrowBack
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.Palette
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.Checklist
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.Notifications
import androidx.compose.material.icons.rounded.Place
import androidx.compose.material.icons.rounded.PhoneAndroid
import androidx.compose.material.icons.rounded.Settings as SettingsIcon
import androidx.compose.material3.AlertDialog
import android.appwidget.AppWidgetManager
import android.content.ComponentName
import dev.flowlist.android.widget.FlowlistWidgetReceiver
import dev.flowlist.android.widget.WidgetUpdater
import androidx.glance.appwidget.updateAll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.AppState
import dev.flowlist.android.data.SettingsDto
import dev.flowlist.android.notify.DeviceLocation
import dev.flowlist.android.notify.NearbyReminders
import dev.flowlist.android.data.UpdateChecker
import dev.flowlist.android.data.UpdateInfo
import dev.flowlist.android.icon.AppIcon
import dev.flowlist.android.icon.AppIcons
import dev.flowlist.android.notify.DueNotifications
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.coroutines.launch

enum class SettingsSection(val label: String, val icon: ImageVector) {
    Appearance("Appearance", Icons.Outlined.Palette),
    Tasks("Tasks", Icons.Outlined.Tune),
    Library("Library", Icons.Rounded.Checklist),
    Notifications("Notifications", Icons.Rounded.Notifications),
    Geotags("Geotags", Icons.Rounded.Place),
    Connection("Connection", Icons.Rounded.PhoneAndroid),
    About("About", Icons.Outlined.Info),
}

@Composable
fun SettingsScreen(
    state: AppState,
    theme: ThemeSettings,
    onThemeChange: (ThemeSettings) -> Unit,
    preferences: AppPreferences,
    onPreferencesChange: (AppPreferences) -> Unit,
    librarySettings: SettingsDto,
    onLibrarySettings: (SettingsDto) -> Unit,
    onSync: () -> Unit,
    onSignOut: () -> Unit,
    onSaveAllPreference: (Boolean) -> Unit,
    onRemoveAllSaved: () -> Unit,
) {
    val context = LocalContext.current
    var section by rememberSaveable { mutableStateOf<String?>(null) }
    val current = SettingsSection.entries.firstOrNull { it.name == section }
    BackHandler(enabled = current != null) { section = null }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(flow.background),
    ) {
        if (current == null) {
            SettingsHome(
                theme = theme,
                preferences = preferences,
                librarySettings = librarySettings,
                state = state,
                context = context,
                onOpen = { section = it.name },
            )
        } else {
            Row(
                modifier = Modifier.fillMaxWidth().padding(start = 8.dp, end = 16.dp, top = 8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                IconButton(onClick = { section = null }) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Rounded.ArrowBack,
                        contentDescription = "Back",
                        tint = flow.faint,
                    )
                }
                Text(
                    text = current.label,
                    style = MaterialTheme.typography.headlineMedium,
                    color = flow.ink,
                )
            }
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 16.dp)
                    .padding(top = 8.dp, bottom = 32.dp),
            ) {
                when (current) {
                    SettingsSection.Appearance -> AppearanceSettings(
                        theme = theme,
                        onThemeChange = onThemeChange,
                        preferences = preferences,
                        onPreferencesChange = onPreferencesChange,
                    )
                    SettingsSection.Tasks -> TaskSettings(preferences, onPreferencesChange)
                    SettingsSection.Library -> {
                        val attachmentPaths = (
                            state.snapshot?.db?.tasks?.values?.flatMap { it.attachments }.orEmpty() +
                                state.snapshot?.db?.projects?.values?.flatMap { it.attachments }.orEmpty()
                            ).filter { it.startsWith("/") }.distinct()
                        LibrarySettings(
                            settings = librarySettings,
                            onChange = onLibrarySettings,
                            attachmentTotal = attachmentPaths.size,
                            attachmentSaved = attachmentPaths.count { it in state.savedAttachments },
                            savingAttachments = state.savingAttachments,
                            saveAllAttachments = preferences.saveAllAttachments,
                            onSaveAllPreference = onSaveAllPreference,
                            onRemoveAllSaved = onRemoveAllSaved,
                        )
                    }
                    SettingsSection.Notifications -> NotificationSettings()
                    SettingsSection.Geotags -> GeotagSettings(preferences, onPreferencesChange)
                    SettingsSection.Connection -> ConnectionSettings(state, onSync, onSignOut)
                    SettingsSection.About -> AboutSettings(state)
                }
            }
        }
    }
}

@Composable
private fun SettingsHome(
    theme: ThemeSettings,
    preferences: AppPreferences,
    librarySettings: SettingsDto,
    state: AppState,
    context: Context,
    onOpen: (SettingsSection) -> Unit,
) {
    val colors = flow
    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 32.dp),
    ) {
        items(SettingsSection.entries, key = { it.name }) { section ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { onOpen(section) }
                    .padding(horizontal = 20.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Box(
                    modifier = Modifier.size(34.dp).clip(CircleShape).background(colors.field),
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(section.icon, null, tint = colors.faint, modifier = Modifier.size(18.dp))
                }
                Spacer(modifier = Modifier.width(14.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(section.label, style = MaterialTheme.typography.bodyMedium, color = colors.ink)
                    Text(
                        text = summary(section, theme, preferences, librarySettings, state, context),
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                }
                Icon(
                    Icons.Rounded.ChevronRight,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(16.dp),
                )
            }
            Hairline(Modifier.padding(start = 68.dp))
        }
    }
}

private fun summary(
    section: SettingsSection,
    theme: ThemeSettings,
    preferences: AppPreferences,
    library: SettingsDto,
    state: AppState,
    context: Context,
): String = when (section) {
    SettingsSection.Appearance -> "${theme.family.label} · ${theme.mode.name.lowercase()}"
    SettingsSection.Tasks -> "Opens ${preferences.startTab.replaceFirstChar { it.uppercase() }} · week starts ${if (preferences.weekStartsMonday) "Monday" else "Sunday"}"
    SettingsSection.Library -> "Review ${library.defaultReviewDays}d · trash ${if (library.trashRetentionDays == 0) "forever" else "${library.trashRetentionDays}d"}"
    SettingsSection.Notifications -> if (DueNotifications.enabled(context)) "On" else "Off"
    SettingsSection.Geotags -> if (preferences.nearbyReminders) "Arrival reminders on" else "Off"
    SettingsSection.Connection -> if (state.online) "Connected" else "Offline"
    SettingsSection.About -> "Version ${dev.flowlist.android.BuildConfig.VERSION_NAME}"
}

// ---------------------------------------------------------------------------
// Sections

@Composable
private fun AppearanceSettings(
    theme: ThemeSettings,
    onThemeChange: (ThemeSettings) -> Unit,
    preferences: AppPreferences,
    onPreferencesChange: (AppPreferences) -> Unit,
) {
    val colors = flow
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var appIcon by remember { mutableStateOf(AppIcons.current(context)) }
    BottomBarSettings(preferences, onPreferencesChange)
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Theme") {
        ChoiceRow {
            AppearanceMode.entries.forEach { mode ->
                Pill(
                    when (mode) {
                        AppearanceMode.System -> "System"
                        AppearanceMode.Light -> "Light"
                        AppearanceMode.Dark -> "Dark"
                    },
                    theme.mode == mode,
                ) { onThemeChange(theme.copy(mode = mode)) }
            }
        }
        Spacer(modifier = Modifier.height(10.dp))
        ToggleRow(
            title = "AMOLED black",
            detail = "Pure black background in dark mode",
            checked = theme.amoled,
        ) {
            onThemeChange(theme.copy(amoled = it))
        }
        Spacer(modifier = Modifier.height(14.dp))
        LazyRow(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            items(Family.entries, key = { it.slug }) { family ->
                ThemeCard(family, theme.family == family, colors.dark) {
                    onThemeChange(theme.copy(family = family))
                }
            }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Accent") {
        ChoiceRow {
            val wallpaper = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
            Pill("Theme", theme.accentMode == AccentMode.Theme) {
                onThemeChange(theme.copy(accentMode = AccentMode.Theme))
            }
            Pill("Wallpaper", theme.accentMode == AccentMode.Wallpaper && wallpaper, wallpaper) {
                onThemeChange(theme.copy(accentMode = AccentMode.Wallpaper))
            }
            Pill("Custom", theme.accentMode == AccentMode.Custom) {
                onThemeChange(theme.copy(accentMode = AccentMode.Custom))
            }
        }
        if (theme.accentMode == AccentMode.Custom) {
            Spacer(modifier = Modifier.height(10.dp))
            LazyRow(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                items(AccentPreset.entries, key = { it.key }) { preset ->
                    val tone = Color(if (colors.dark) preset.dark else preset.light)
                    Box(
                        modifier = Modifier
                            .size(34.dp)
                            .clip(CircleShape)
                            .background(tone)
                            .border(
                                if (theme.customAccent == preset) 2.dp else 1.dp,
                                if (theme.customAccent == preset) colors.ink else colors.hairline,
                                CircleShape,
                            )
                            .clickable {
                                onThemeChange(theme.copy(accentMode = AccentMode.Custom, customAccent = preset))
                            },
                    )
                }
            }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("App icon") {
        LazyRow(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            items(AppIcon.entries, key = { it.key }) { icon ->
                Box(
                    modifier = Modifier
                        .size(46.dp)
                        .clip(RoundedCornerShape(13.dp))
                        .background(Color(icon.color))
                        .border(
                            if (appIcon == icon) 2.5.dp else 1.dp,
                            if (appIcon == icon) colors.ink else Color.Transparent,
                            RoundedCornerShape(13.dp),
                        )
                        .clickable {
                            AppIcons.apply(context, icon)
                            appIcon = icon
                            scope.launch {
                                WidgetUpdater.refresh(context.applicationContext)
                            }
                        },
                    contentAlignment = Alignment.Center,
                ) {
                    Icon(Icons.Rounded.Checklist, "${icon.label} icon", tint = Color.White, modifier = Modifier.size(22.dp))
                }
            }
        }
    }
}

@Composable
private fun TaskSettings(preferences: AppPreferences, onChange: (AppPreferences) -> Unit) {
    val context = LocalContext.current
    SettingsCard("Startup") {
        SmallLabel("Open on")
        ChoiceRow {
            listOf("inbox" to "Inbox", "today" to "Today", "forecast" to "Forecast", "browse" to "Browse")
                .forEach { (key, label) ->
                    Pill(label, preferences.startTab == key) { onChange(preferences.copy(startTab = key)) }
                }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Dates") {
        SmallLabel("Week starts on")
        ChoiceRow {
            Pill("Monday", preferences.weekStartsMonday) { onChange(preferences.copy(weekStartsMonday = true)) }
            Pill("Sunday", !preferences.weekStartsMonday) { onChange(preferences.copy(weekStartsMonday = false)) }
        }
        Spacer(modifier = Modifier.height(14.dp))
        SmallLabel("Default time")
        ChoiceRow {
            listOf(9 to 0, 12 to 0, 18 to 0).forEach { (hour, minute) ->
                val minutes = hour * 60 + minute
                Pill("%02d:%02d".format(hour, minute), preferences.defaultTimeMinutes == minutes) {
                    onChange(preferences.copy(defaultTimeMinutes = minutes))
                }
            }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Interaction") {
        ToggleRow("Haptics", "A tap when a task is completed", preferences.haptics) {
            onChange(preferences.copy(haptics = it))
        }
        ToggleRow("Swipe actions", "Right to complete, left to trash", preferences.swipeActions) {
            onChange(preferences.copy(swipeActions = it))
        }
        ToggleRow(
            "Keep completed subtasks",
            "Leave finished subtasks under the parent, struck through",
            preferences.keepCompletedSubtasks,
        ) {
            onChange(preferences.copy(keepCompletedSubtasks = it))
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Home-screen widgets") {
        Text(
            text = "Add a Today, Inbox or Upcoming widget. Each widget keeps its own theme, row count and options.",
            style = MaterialTheme.typography.bodySmall,
            color = flow.faint,
        )
        Spacer(modifier = Modifier.height(10.dp))
        Button(
            onClick = {
                AppWidgetManager.getInstance(context).requestPinAppWidget(
                    ComponentName(context, FlowlistWidgetReceiver::class.java),
                    null,
                    null,
                )
            },
            colors = ButtonDefaults.buttonColors(
                containerColor = flow.accent,
                contentColor = flow.onAccent,
            ),
        ) {
            Text("Add widget")
        }
    }
}

@Composable
private fun GeotagSettings(
    preferences: AppPreferences,
    onChange: (AppPreferences) -> Unit,
) {
    val colors = flow
    val context = LocalContext.current
    val backgroundLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { }
    val foregroundLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { grants ->
        if (grants.values.any { it } && Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            backgroundLauncher.launch(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
        }
    }
    SettingsCard("Arrival reminders") {
        ToggleRow(
            "Location reminders",
            "Remind me when I arrive near a geotagged tag",
            preferences.nearbyReminders,
        ) {
            onChange(preferences.copy(nearbyReminders = it))
            NearbyReminders.setEnabled(context, it)
        }
        if (preferences.nearbyReminders) {
            val hasForeground = DeviceLocation.hasForeground(context)
            val hasBackground = DeviceLocation.hasBackground(context)
            when {
                !hasForeground -> TextButton(onClick = {
                    foregroundLauncher.launch(
                        arrayOf(
                            Manifest.permission.ACCESS_FINE_LOCATION,
                            Manifest.permission.ACCESS_COARSE_LOCATION,
                        ),
                    )
                }) {
                    Text("Allow location", color = colors.accent)
                }
                !hasBackground -> TextButton(onClick = {
                    backgroundLauncher.launch(Manifest.permission.ACCESS_BACKGROUND_LOCATION)
                }) {
                    Text("Allow background location", color = colors.accent)
                }
            }
            Text(
                text = "Geotag a tag to get a reminder when you arrive within 250 m of it.",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
            TextButton(onClick = { NearbyReminders.sendTest(context) }) {
                Text("Send a test reminder", color = colors.accent)
            }
        }
    }
}

@Composable
private fun LibrarySettings(
    settings: SettingsDto,
    onChange: (SettingsDto) -> Unit,
    attachmentTotal: Int,
    attachmentSaved: Int,
    savingAttachments: Boolean,
    saveAllAttachments: Boolean,
    onSaveAllPreference: (Boolean) -> Unit,
    onRemoveAllSaved: () -> Unit,
) {
    var confirmRemove by remember { mutableStateOf(false) }
    SettingsCard("Review") {
        SmallLabel("Default review interval")
        ChoiceRow {
            listOf(7, 14, 30).forEach { days ->
                Pill("${days}d", settings.defaultReviewDays == days) {
                    onChange(settings.copy(defaultReviewDays = days))
                }
            }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("History") {
        SmallLabel("Changed window")
        ChoiceRow {
            listOf(7, 14, 30).forEach { days ->
                Pill("${days}d", settings.changedDays == days) {
                    onChange(settings.copy(changedDays = days))
                }
            }
        }
        Spacer(modifier = Modifier.height(14.dp))
        SmallLabel("Keep completed tasks visible")
        ChoiceRow {
            listOf(30, 60, 90).forEach { days ->
                Pill("${days}d", settings.showCompletedDays == days) {
                    onChange(settings.copy(showCompletedDays = days))
                }
            }
        }
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Trash") {
        SmallLabel("Keep deleted tasks for")
        ChoiceRow {
            listOf(0, 7, 30, 90).forEach { days ->
                Pill(if (days == 0) "Forever" else "${days}d", settings.trashRetentionDays == days) {
                    onChange(settings.copy(trashRetentionDays = days))
                }
            }
        }
        Spacer(modifier = Modifier.height(6.dp))
        Text(
            text = "These defaults live in the library, so the Mac sees them too.",
            style = MaterialTheme.typography.labelSmall,
            color = flow.faint,
        )
    }
    Spacer(modifier = Modifier.height(14.dp))
    SettingsCard("Offline files") {
        ToggleRow(
            title = "Save all attachments",
            detail = "Keep every attachment available offline",
            checked = saveAllAttachments,
        ) {
            onSaveAllPreference(it)
        }
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = when {
                savingAttachments -> "Saving attachments…"
                attachmentTotal == 0 -> "No attachments in the library yet."
                attachmentSaved >= attachmentTotal -> "All $attachmentTotal attachments are saved on this phone."
                else -> "$attachmentSaved of $attachmentTotal attachments saved on this phone."
            },
            style = MaterialTheme.typography.bodyMedium,
            color = flow.faint,
        )
        Spacer(modifier = Modifier.height(12.dp))
        Button(
            onClick = { confirmRemove = true },
            enabled = attachmentSaved > 0 && !savingAttachments,
            colors = ButtonDefaults.buttonColors(
                containerColor = flow.danger.copy(alpha = 0.12f),
                contentColor = flow.danger,
                disabledContainerColor = flow.field,
                disabledContentColor = flow.faint,
            ),
        ) {
            Text("Remove all offline copies")
        }
    }

    if (confirmRemove) {
        AlertDialog(
            onDismissRequest = { confirmRemove = false },
            title = { Text("Remove all offline copies?") },
            text = {
                Text(
                    "Every saved attachment is deleted from this phone. " +
                        "The files on the Mac are not touched.",
                )
            },
            confirmButton = {
                TextButton(
                    onClick = {
                        confirmRemove = false
                        onRemoveAllSaved()
                    },
                ) {
                    Text("Remove", color = flow.danger)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmRemove = false }) {
                    Text("Cancel", color = flow.faint)
                }
            },
            containerColor = flow.background,
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun NotificationSettings() {
    val context = LocalContext.current
    var enabled by remember { mutableStateOf(DueNotifications.enabled(context)) }
    var denied by remember { mutableStateOf(false) }
    val permission = rememberLauncherForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        if (granted) {
            DueNotifications.setEnabled(context, true)
            enabled = true
            denied = false
        } else {
            denied = true
        }
    }
    SettingsCard("Due reminders") {
        ToggleRow(
            title = "Due reminders",
            detail = when {
                denied -> "Allow notifications in Android settings"
                enabled -> "A summary when tasks are due today"
                else -> "Off"
            },
            checked = enabled,
            danger = denied,
        ) {
            if (enabled) {
                DueNotifications.setEnabled(context, false)
                enabled = false
            } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
                android.content.pm.PackageManager.PERMISSION_GRANTED
            ) {
                permission.launch(Manifest.permission.POST_NOTIFICATIONS)
            } else {
                DueNotifications.setEnabled(context, true)
                enabled = true
            }
        }
        if (enabled) {
            Spacer(modifier = Modifier.height(8.dp))
            TextButton(onClick = { DueNotifications.sendTest(context) }) {
                Text("Send a test notification", color = flow.accent)
            }
        }
        Spacer(modifier = Modifier.height(6.dp))
        Text(
            text = "Android may delay a summary to save battery.",
            style = MaterialTheme.typography.labelSmall,
            color = flow.faint,
        )
    }
}

@Composable
private fun ConnectionSettings(state: AppState, onSync: () -> Unit, onSignOut: () -> Unit) {
    val colors = flow
    var confirm by remember { mutableStateOf(false) }
    SettingsCard("This Mac") {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Column(modifier = Modifier.weight(1f)) {
                Text(state.server.ifEmpty { "Not paired" }, style = MaterialTheme.typography.labelLarge, color = colors.ink)
                Text(
                    text = when {
                        state.syncing -> "Syncing"
                        state.online -> "Connected"
                        else -> "Offline"
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = if (state.online) colors.success else colors.danger,
                )
            }
            TextButton(onClick = onSync) { Text("Sync now", color = colors.accent) }
        }
        Spacer(modifier = Modifier.height(4.dp))
        Text(lastSync(state), style = MaterialTheme.typography.labelSmall, color = colors.faint)
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "On the Mac, open Settings → Mobile for the address and a pairing code.",
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
            lineHeight = 17.sp,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Unpair this phone",
            style = MaterialTheme.typography.labelLarge,
            color = colors.danger,
            modifier = Modifier.fillMaxWidth().clickable { confirm = true }.padding(vertical = 8.dp),
        )
    }
    if (confirm) {
        AlertDialog(
            onDismissRequest = { confirm = false },
            title = { Text("Unpair this phone?") },
            text = { Text("You will need the address and a new pairing code from the Mac.") },
            confirmButton = {
                TextButton(onClick = { confirm = false; onSignOut() }) { Text("Unpair", color = colors.danger) }
            },
            dismissButton = { TextButton(onClick = { confirm = false }) { Text("Cancel") } },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun AboutSettings(state: AppState) {
    val context = LocalContext.current
    var checking by remember { mutableStateOf(false) }
    var update by remember { mutableStateOf<UpdateInfo?>(null) }
    var message by remember { mutableStateOf("Check for a newer APK") }
    val scope = rememberCoroutineScope()
    SettingsCard("Flowlist") {
        Text(
            text = "Version ${dev.flowlist.android.BuildConfig.VERSION_NAME}",
            style = MaterialTheme.typography.titleMedium,
            color = flow.ink,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
            text = "Local-first. Your tasks live on your Mac; this app talks to it directly. No account, no cloud.",
            style = MaterialTheme.typography.labelMedium,
            color = flow.faint,
            lineHeight = 18.sp,
        )
        Spacer(modifier = Modifier.height(12.dp))
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(message, style = MaterialTheme.typography.labelMedium, color = if (update != null) flow.success else flow.faint, modifier = Modifier.weight(1f))
            TextButton(
                onClick = {
                    if (update != null) {
                        context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(update!!.downloadUrl)))
                    } else {
                        checking = true
                        message = "Checking…"
                        scope.launch {
                            UpdateChecker.check().fold(
                                onSuccess = { found ->
                                    checking = false
                                    update = found
                                    message = if (found == null) "Up to date" else "v${found.latest} is available"
                                },
                                onFailure = {
                                    checking = false
                                    message = "Could not check · tap to retry"
                                },
                            )
                        }
                    }
                },
                enabled = !checking,
            ) {
                Text(if (update != null) "Download" else "Check for updates", color = flow.accent)
            }
        }
        Spacer(modifier = Modifier.height(8.dp))
        Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
            TextButton(onClick = {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/Vulkan1125/flowlist")))
            }) { Text("GitHub", color = flow.accent) }
            TextButton(onClick = {
                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/Vulkan1125/flowlist/wiki")))
            }) { Text("Wiki", color = flow.accent) }
        }
    }
}

// ---------------------------------------------------------------------------
// Shared controls

@Composable
private fun SettingsCard(title: String, content: @Composable ColumnScope.() -> Unit) {
    val colors = flow
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(colors.field)
            .border(1.dp, colors.hairline, RoundedCornerShape(16.dp))
            .padding(16.dp),
    ) {
        Text(title, style = MaterialTheme.typography.titleMedium, color = colors.ink)
        Spacer(modifier = Modifier.height(12.dp))
        content()
    }
}

@Composable
private fun ChoiceRow(content: @Composable () -> Unit) {
    Row(horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) { content() }
}

@Composable
private fun SmallLabel(text: String) {
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.7.sp,
        color = flow.faint,
        modifier = Modifier.padding(bottom = 6.dp),
    )
}

@Composable
private fun ToggleRow(
    title: String,
    detail: String,
    checked: Boolean,
    danger: Boolean = false,
    onChange: (Boolean) -> Unit,
) {
    val colors = flow
    Row(
        modifier = Modifier.fillMaxWidth().clickable { onChange(!checked) }.padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(title, style = MaterialTheme.typography.labelLarge, color = colors.ink)
            Text(detail, style = MaterialTheme.typography.labelMedium, color = if (danger) colors.danger else colors.faint)
        }
        Switch(
            checked = checked,
            onCheckedChange = null,
            colors = SwitchDefaults.colors(
                checkedThumbColor = colors.onAccent,
                checkedTrackColor = colors.accent,
                uncheckedThumbColor = colors.faint,
                uncheckedTrackColor = colors.background,
                uncheckedBorderColor = colors.hairline,
            ),
        )
    }
}

@Composable
private fun Pill(label: String, selected: Boolean, enabled: Boolean = true, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .border(1.dp, if (selected) colors.accent else colors.hairline, RoundedCornerShape(9.dp))
            .background(if (selected) colors.accent.copy(alpha = 0.10f) else Color.Transparent)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = when {
                !enabled -> colors.faint.copy(alpha = 0.45f)
                selected -> colors.accent
                else -> colors.ink
            },
        )
    }
}

@Composable
private fun BottomBarSettings(
    preferences: AppPreferences,
    onPreferencesChange: (AppPreferences) -> Unit,
) {
    val colors = flow
    val slots = resolveBottomBar(preferences.bottomBar)
    var editing by remember { mutableStateOf<Int?>(null) }

    SettingsCard("Bottom bar") {
        Text(
            text = "Pick the four perspectives on the bottom bar. Settings always stays at the far right.",
            style = MaterialTheme.typography.bodySmall,
            color = colors.faint,
        )
        Spacer(modifier = Modifier.height(6.dp))
        slots.forEachIndexed { index, item ->
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(10.dp))
                    .clickable { editing = index }
                    .padding(vertical = 10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "${index + 1}",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    modifier = Modifier.width(22.dp),
                )
                Icon(
                    imageVector = item.icon,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(18.dp),
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = item.label,
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.ink,
                    modifier = Modifier.weight(1f),
                )
                Icon(
                    imageVector = Icons.Rounded.ChevronRight,
                    contentDescription = null,
                    tint = colors.faint,
                    modifier = Modifier.size(17.dp),
                )
            }
        }
    }

    if (editing != null) {
        val slot = editing!!
        AlertDialog(
            onDismissRequest = { editing = null },
            title = { Text("Slot ${slot + 1}") },
            text = {
                Column(
                    modifier = Modifier
                        .heightIn(max = 420.dp)
                        .verticalScroll(rememberScrollState()),
                ) {
                    Perspective.entries.forEach { item ->
                        val selected = slots[slot] == item
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .clip(RoundedCornerShape(9.dp))
                                .clickable {
                                    val keys = slots.map { it.key }.toMutableList()
                                    val previous = slots[slot].key
                                    val existing = keys.indexOf(item.key)
                                    keys[slot] = item.key
                                    if (existing >= 0 && existing != slot) keys[existing] = previous
                                    onPreferencesChange(preferences.copy(bottomBar = keys))
                                    editing = null
                                }
                                .padding(vertical = 11.dp, horizontal = 4.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Icon(
                                imageVector = item.icon,
                                contentDescription = null,
                                tint = if (selected) colors.accent else colors.faint,
                                modifier = Modifier.size(18.dp),
                            )
                            Spacer(modifier = Modifier.width(12.dp))
                            Text(
                                text = item.label,
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (selected) colors.accent else colors.ink,
                                modifier = Modifier.weight(1f),
                            )
                            if (selected) {
                                Icon(
                                    imageVector = Icons.Rounded.Check,
                                    contentDescription = null,
                                    tint = colors.accent,
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                    }
                }
            },
            confirmButton = {},
            dismissButton = {
                TextButton(onClick = { editing = null }) {
                    Text("Cancel", color = colors.accent)
                }
            },
            containerColor = colors.background,
        )
    }
}

@Composable
private fun ThemeCard(family: Family, selected: Boolean, dark: Boolean, onClick: () -> Unit) {
    val colors = flow
    val preview = themePreview(family, dark)
    Column(
        modifier = Modifier
            .width(132.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(preview.background)
            .border(if (selected) 2.dp else 1.dp, if (selected) colors.accent else colors.hairline, RoundedCornerShape(12.dp))
            .clickable(onClick = onClick)
            .padding(12.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            Box(Modifier.size(14.dp).clip(CircleShape).background(preview.ink))
            Box(Modifier.size(14.dp).clip(CircleShape).background(preview.accent))
            Box(Modifier.size(14.dp).clip(CircleShape).border(1.dp, preview.ink.copy(alpha = 0.25f), CircleShape))
        }
        Spacer(modifier = Modifier.height(10.dp))
        Text(family.label, style = MaterialTheme.typography.labelLarge, color = preview.ink, maxLines = 1)
        Text(family.hint, style = MaterialTheme.typography.labelSmall, color = preview.ink.copy(alpha = 0.55f), maxLines = 1)
    }
}

private fun lastSync(state: AppState): String {
    val at = state.lastSyncAt ?: return "Not synced yet"
    val minutes = (System.currentTimeMillis() - at) / 60_000
    return when {
        minutes < 1 -> "Synced just now"
        minutes < 60 -> "Synced ${minutes}m ago"
        else -> {
            val time = Instant.ofEpochMilli(at).atZone(ZoneId.systemDefault())
                .format(DateTimeFormatter.ofPattern("HH:mm", Locale.getDefault()))
            "Synced at $time"
        }
    }
}
