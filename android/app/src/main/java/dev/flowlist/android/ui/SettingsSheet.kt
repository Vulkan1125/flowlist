package dev.flowlist.android.ui

import android.Manifest
import android.content.Intent
import android.net.Uri
import android.os.Build
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Checklist
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.AppState
import dev.flowlist.android.icon.AppIcon
import dev.flowlist.android.icon.AppIcons
import dev.flowlist.android.data.UpdateChecker
import dev.flowlist.android.data.UpdateInfo
import dev.flowlist.android.notify.DueNotifications
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(
    state: AppState,
    theme: ThemeSettings,
    onThemeChange: (ThemeSettings) -> Unit,
    preferences: AppPreferences,
    onPreferencesChange: (AppPreferences) -> Unit,
    onDismiss: () -> Unit,
    onSync: () -> Unit,
    onSignOut: () -> Unit,
) {
    val colors = flow
    val context = LocalContext.current
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var confirmSignOut by remember { mutableStateOf(false) }
    var reminders by remember { mutableStateOf(DueNotifications.enabled(context)) }
    var reminderDenied by remember { mutableStateOf(false) }
    var appIcon by remember { mutableStateOf(AppIcons.current(context)) }
    var checkingUpdate by remember { mutableStateOf(false) }
    var updateInfo by remember { mutableStateOf<UpdateInfo?>(null) }
    var updateMessage by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    val permission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (granted) {
            DueNotifications.setEnabled(context, true)
            reminders = true
            reminderDenied = false
        } else {
            reminderDenied = true
        }
    }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .statusBarsPadding()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp)
                .padding(bottom = 32.dp),
        ) {
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "Settings",
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 4.dp),
            )
            Spacer(modifier = Modifier.height(18.dp))

            SettingsCard("Appearance") {
                ChoiceRow {
                    AppearanceMode.entries.forEach { mode ->
                        Pill(
                            label = when (mode) {
                                AppearanceMode.System -> "System"
                                AppearanceMode.Light -> "Light"
                                AppearanceMode.Dark -> "Dark"
                            },
                            selected = theme.mode == mode,
                            onClick = { onThemeChange(theme.copy(mode = mode)) },
                        )
                    }
                }
                Spacer(modifier = Modifier.height(14.dp))
                SmallLabel("Theme")
                LazyRow(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    items(Family.entries, key = { it.slug }) { family ->
                        ThemeCard(
                            family = family,
                            selected = theme.family == family,
                            dark = colors.dark,
                            onClick = { onThemeChange(theme.copy(family = family)) },
                        )
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
                SmallLabel("Accent")
                ChoiceRow {
                    val wallpaper = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                    Pill(
                        "Theme",
                        theme.accentMode == AccentMode.Theme,
                    ) { onThemeChange(theme.copy(accentMode = AccentMode.Theme)) }
                    Pill(
                        "Wallpaper",
                        theme.accentMode == AccentMode.Wallpaper && wallpaper,
                        enabled = wallpaper,
                    ) { onThemeChange(theme.copy(accentMode = AccentMode.Wallpaper)) }
                    Pill(
                        "Custom",
                        theme.accentMode == AccentMode.Custom,
                    ) { onThemeChange(theme.copy(accentMode = AccentMode.Custom)) }
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
                                        width = if (theme.customAccent == preset) 2.dp else 1.dp,
                                        color = if (theme.customAccent == preset) {
                                            colors.ink
                                        } else {
                                            colors.hairline
                                        },
                                        shape = CircleShape,
                                    )
                                    .clickable {
                                        onThemeChange(
                                            theme.copy(
                                                accentMode = AccentMode.Custom,
                                                customAccent = preset,
                                            ),
                                        )
                                    },
                            )
                        }
                    }
                }
                Spacer(modifier = Modifier.height(16.dp))
                SmallLabel("App icon")
                LazyRow(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                    items(AppIcon.entries, key = { it.key }) { icon ->
                        val selected = appIcon == icon
                        Box(
                            modifier = Modifier
                                .size(46.dp)
                                .clip(RoundedCornerShape(13.dp))
                                .background(Color(icon.color))
                                .border(
                                    width = if (selected) 2.5.dp else 1.dp,
                                    color = if (selected) colors.ink else Color.Transparent,
                                    shape = RoundedCornerShape(13.dp),
                                )
                                .clickable {
                                    AppIcons.apply(context, icon)
                                    appIcon = icon
                                    android.widget.Toast.makeText(
                                        context,
                                        "${icon.label} icon selected",
                                        android.widget.Toast.LENGTH_SHORT,
                                    ).show()
                                },
                            contentAlignment = Alignment.Center,
                        ) {
                            Icon(
                                imageVector = Icons.Rounded.Checklist,
                                contentDescription = "${icon.label} icon",
                                tint = Color.White,
                                modifier = Modifier.size(22.dp),
                            )
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            SettingsCard("Tasks") {
                SmallLabel("Week starts on")
                ChoiceRow {
                    Pill("Monday", preferences.weekStartsMonday) {
                        onPreferencesChange(preferences.copy(weekStartsMonday = true))
                    }
                    Pill("Sunday", !preferences.weekStartsMonday) {
                        onPreferencesChange(preferences.copy(weekStartsMonday = false))
                    }
                }
                Spacer(modifier = Modifier.height(14.dp))
                SmallLabel("Open on")
                ChoiceRow {
                    listOf("inbox" to "Inbox", "today" to "Today", "forecast" to "Forecast", "browse" to "Browse")
                        .forEach { (key, label) ->
                            Pill(label, preferences.startTab == key) {
                                onPreferencesChange(preferences.copy(startTab = key))
                            }
                        }
                }
                Spacer(modifier = Modifier.height(14.dp))
                SmallLabel("Default time")
                ChoiceRow {
                    listOf(9 to 0, 12 to 0, 18 to 0).forEach { (hour, minute) ->
                        val minutes = hour * 60 + minute
                        Pill(
                            label = "%02d:%02d".format(hour, minute),
                            selected = preferences.defaultTimeMinutes == minutes,
                        ) {
                            onPreferencesChange(preferences.copy(defaultTimeMinutes = minutes))
                        }
                    }
                }
                Spacer(modifier = Modifier.height(10.dp))
                ToggleRow(
                    title = "Haptics",
                    detail = "A tap when a task is completed",
                    checked = preferences.haptics,
                ) {
                    onPreferencesChange(preferences.copy(haptics = it))
                }
                ToggleRow(
                    title = "Swipe actions",
                    detail = "Right to complete, left to trash",
                    checked = preferences.swipeActions,
                ) {
                    onPreferencesChange(preferences.copy(swipeActions = it))
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            SettingsCard("Notifications") {
                ToggleRow(
                    title = "Due reminders",
                    detail = when {
                        reminderDenied -> "Allow notifications in Android settings"
                        reminders -> "A summary when tasks are due today"
                        else -> "Off"
                    },
                    checked = reminders,
                    danger = reminderDenied,
                ) {
                    if (reminders) {
                        DueNotifications.setEnabled(context, false)
                        reminders = false
                    } else if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
                        ContextCompat.checkSelfPermission(
                            context,
                            Manifest.permission.POST_NOTIFICATIONS,
                        ) != android.content.pm.PackageManager.PERMISSION_GRANTED
                    ) {
                        permission.launch(Manifest.permission.POST_NOTIFICATIONS)
                    } else {
                        DueNotifications.setEnabled(context, true)
                        reminders = true
                    }
                }
                if (reminders) {
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(
                            text = if (DueNotifications.canPost(context)) {
                                "Android may delay a summary to save battery."
                            } else {
                                "Notifications are blocked in Android settings."
                            },
                            style = MaterialTheme.typography.labelSmall,
                            color = colors.faint,
                            modifier = Modifier.weight(1f),
                        )
                        TextButton(onClick = { DueNotifications.sendTest(context) }) {
                            Text("Send test", color = colors.accent, style = MaterialTheme.typography.labelMedium)
                        }
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))
            SettingsCard("Connection") {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "This Mac",
                            style = MaterialTheme.typography.labelLarge,
                            color = colors.ink,
                        )
                        Spacer(modifier = Modifier.height(2.dp))
                        Text(
                            text = state.server.ifEmpty { "Not paired" },
                            style = MaterialTheme.typography.labelMedium,
                            color = colors.faint,
                        )
                    }
                    Box(
                        modifier = Modifier
                            .size(7.dp)
                            .clip(CircleShape)
                            .background(
                                when {
                                    state.syncing -> colors.warning
                                    state.online -> colors.success
                                    else -> colors.danger
                                },
                            ),
                    )
                    Spacer(modifier = Modifier.width(6.dp))
                    Text(
                        text = when {
                            state.syncing -> "Syncing"
                            state.online -> "Connected"
                            else -> "Offline"
                        },
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                }
                Spacer(modifier = Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = lastSync(state),
                        style = MaterialTheme.typography.labelSmall,
                        color = colors.faint,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(onClick = onSync) {
                        Text("Sync now", color = colors.accent)
                    }
                }
                Spacer(modifier = Modifier.height(4.dp))
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
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { confirmSignOut = true }
                        .padding(vertical = 10.dp),
                )
            }

            Spacer(modifier = Modifier.height(14.dp))
            SettingsCard("About") {
                Text(
                    text = "Flowlist for Android ${dev.flowlist.android.BuildConfig.VERSION_NAME}",
                    style = MaterialTheme.typography.labelLarge,
                    color = colors.ink,
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = "Local-first. Your tasks live on your Mac; this app talks to it directly. No account, no cloud.",
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    lineHeight = 18.sp,
                )
                Spacer(modifier = Modifier.height(8.dp))
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Text(
                        text = updateMessage ?: "Check for a newer APK",
                        style = MaterialTheme.typography.labelMedium,
                        color = if (updateInfo != null) colors.success else colors.faint,
                        modifier = Modifier.weight(1f),
                    )
                    TextButton(
                        onClick = {
                            if (updateInfo != null) {
                                val target = updateInfo!!.downloadUrl
                                context.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(target)))
                            } else {
                                checkingUpdate = true
                                updateMessage = "Checking…"
                                scope.launch {
                                    UpdateChecker.check().fold(
                                        onSuccess = { found ->
                                            checkingUpdate = false
                                            updateInfo = found
                                            updateMessage = if (found == null) {
                                                "Up to date"
                                            } else {
                                                "v${found.latest} is available"
                                            }
                                        },
                                        onFailure = {
                                            checkingUpdate = false
                                            updateMessage = "Could not check · tap to retry"
                                        },
                                    )
                                }
                            }
                        },
                        enabled = !checkingUpdate,
                    ) {
                        Text(
                            if (updateInfo != null) "Download" else "Check for updates",
                            color = colors.accent,
                        )
                    }
                }
                Spacer(modifier = Modifier.height(4.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    TextButton(
                        onClick = {
                            context.startActivity(
                                Intent(Intent.ACTION_VIEW, Uri.parse("https://github.com/Vulkan1125/flowlist")),
                            )
                        },
                    ) {
                        Text("GitHub", color = colors.accent)
                    }
                    TextButton(
                        onClick = {
                            context.startActivity(
                                Intent(
                                    Intent.ACTION_VIEW,
                                    Uri.parse("https://github.com/Vulkan1125/flowlist/wiki"),
                                ),
                            )
                        },
                    ) {
                        Text("Wiki", color = colors.accent)
                    }
                }
            }
        }
    }

    if (confirmSignOut) {
        AlertDialog(
            onDismissRequest = { confirmSignOut = false },
            title = { Text("Unpair this phone?") },
            text = { Text("You will need the address and a new pairing code from Settings → Mobile on the Mac.") },
            confirmButton = {
                TextButton(onClick = { confirmSignOut = false; onSignOut() }) {
                    Text("Unpair", color = colors.danger)
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmSignOut = false }) { Text("Cancel") }
            },
            containerColor = colors.background,
        )
    }
}

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
        Text(
            text = title,
            style = MaterialTheme.typography.titleMedium,
            color = colors.ink,
        )
        Spacer(modifier = Modifier.height(12.dp))
        content()
    }
}

@Composable
private fun ChoiceRow(content: @Composable () -> Unit) {
    Row(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        content()
    }
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
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onChange(!checked) }
            .padding(vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(
                text = title,
                style = MaterialTheme.typography.labelLarge,
                color = colors.ink,
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = detail,
                style = MaterialTheme.typography.labelMedium,
                color = if (danger) colors.danger else colors.faint,
            )
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
private fun Pill(
    label: String,
    selected: Boolean,
    enabled: Boolean = true,
    onClick: () -> Unit,
) {
    val colors = flow
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(9.dp))
            .border(
                width = 1.dp,
                color = if (selected) colors.accent else colors.hairline,
                shape = RoundedCornerShape(9.dp),
            )
            .background(if (selected) colors.accent.copy(alpha = 0.10f) else Color.Transparent)
            .clickable(enabled = enabled, onClick = onClick)
            .padding(horizontal = 13.dp, vertical = 7.dp),
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
private fun ThemeCard(
    family: Family,
    selected: Boolean,
    dark: Boolean,
    onClick: () -> Unit,
) {
    val colors = flow
    val preview = themePreview(family, dark)
    Column(
        modifier = Modifier
            .width(132.dp)
            .clip(RoundedCornerShape(12.dp))
            .background(preview.background)
            .border(
                width = if (selected) 2.dp else 1.dp,
                color = if (selected) colors.accent else colors.hairline,
                shape = RoundedCornerShape(12.dp),
            )
            .clickable(onClick = onClick)
            .padding(12.dp),
    ) {
        Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            Box(Modifier.size(14.dp).clip(CircleShape).background(preview.ink))
            Box(Modifier.size(14.dp).clip(CircleShape).background(preview.accent))
            Box(
                Modifier
                    .size(14.dp)
                    .clip(CircleShape)
                    .border(1.dp, preview.ink.copy(alpha = 0.25f), CircleShape),
            )
        }
        Spacer(modifier = Modifier.height(10.dp))
        Text(
            text = family.label,
            style = MaterialTheme.typography.labelLarge,
            color = preview.ink,
            maxLines = 1,
        )
        Text(
            text = family.hint,
            style = MaterialTheme.typography.labelSmall,
            color = preview.ink.copy(alpha = 0.55f),
            maxLines = 1,
        )
    }
}

private fun lastSync(state: AppState): String {
    val at = state.lastSyncAt ?: return "Not synced yet"
    val minutes = (System.currentTimeMillis() - at) / 60_000
    return when {
        minutes < 1 -> "Synced just now"
        minutes < 60 -> "Synced ${minutes}m ago"
        else -> {
            val time = Instant.ofEpochMilli(at)
                .atZone(ZoneId.systemDefault())
                .format(DateTimeFormatter.ofPattern("HH:mm", Locale.getDefault()))
            "Synced at $time"
        }
    }
}
