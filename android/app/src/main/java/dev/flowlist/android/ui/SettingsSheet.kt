package dev.flowlist.android.ui

import android.Manifest
import android.os.Build
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
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
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Checklist
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.platform.LocalContext
import androidx.core.content.ContextCompat
import dev.flowlist.android.data.AppState
import dev.flowlist.android.notify.DueNotifications
import dev.flowlist.android.icon.AppIcon
import dev.flowlist.android.icon.AppIcons

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(
    state: AppState,
    theme: ThemeSettings,
    onThemeChange: (ThemeSettings) -> Unit,
    onDismiss: () -> Unit,
    onSync: () -> Unit,
    onSignOut: () -> Unit,
) {
    val colors = flow
    val context = LocalContext.current
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var confirmSignOut by remember { mutableStateOf(false) }
    var reminders by remember { mutableStateOf(DueNotifications.enabled(context)) }
    var appIcon by remember { mutableStateOf(AppIcons.current(context)) }
    var reminderDenied by remember { mutableStateOf(false) }
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
                .padding(bottom = 30.dp),
        ) {
            Spacer(modifier = Modifier.height(14.dp))
            Text(
                text = "Settings",
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp),
            )
            Spacer(modifier = Modifier.height(22.dp))

            SectionTitle("Appearance")
            Row(
                modifier = Modifier.padding(horizontal = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                AppearanceMode.entries.forEach { mode ->
                    ChoicePill(
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

            Spacer(modifier = Modifier.height(22.dp))
            SectionTitle("Theme")
            LazyRow(
                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                items(Family.entries, key = { it.slug }) { family ->
                    ThemeCard(
                        family = family,
                        selected = theme.family == family,
                        dark = colors.dark,
                        onClick = { onThemeChange(theme.copy(family = family)) },
                    )
                }
            }

            Spacer(modifier = Modifier.height(22.dp))
            SectionTitle("App icon")
            LazyRow(
                contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(10.dp),
            ) {
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
                        androidx.compose.material3.Icon(
                            imageVector = Icons.Rounded.Checklist,
                            contentDescription = "${icon.label} icon",
                            tint = Color.White,
                            modifier = Modifier.size(22.dp),
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(22.dp))
            SectionTitle("Accent")
            Row(
                modifier = Modifier.padding(horizontal = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                val wallpaper = Build.VERSION.SDK_INT >= Build.VERSION_CODES.S
                ChoicePill(
                    label = "Theme",
                    selected = theme.accentMode == AccentMode.Theme,
                    onClick = { onThemeChange(theme.copy(accentMode = AccentMode.Theme)) },
                )
                ChoicePill(
                    label = "Wallpaper",
                    selected = theme.accentMode == AccentMode.Wallpaper && wallpaper,
                    enabled = wallpaper,
                    onClick = { onThemeChange(theme.copy(accentMode = AccentMode.Wallpaper)) },
                )
                ChoicePill(
                    label = "Custom",
                    selected = theme.accentMode == AccentMode.Custom,
                    onClick = { onThemeChange(theme.copy(accentMode = AccentMode.Custom)) },
                )
            }
            if (theme.accentMode == AccentMode.Custom) {
                Spacer(modifier = Modifier.height(12.dp))
                LazyRow(
                    contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
                    horizontalArrangement = Arrangement.spacedBy(10.dp),
                ) {
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
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = "Each preset has a light and a dark tone.",
                    style = MaterialTheme.typography.labelSmall,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp),
                )
            }
            if (theme.accentMode == AccentMode.Wallpaper) {
                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                        "Uses Android's Material You colours from your wallpaper."
                    } else {
                        "Wallpaper colours need Android 12 or later."
                    },
                    style = MaterialTheme.typography.labelMedium,
                    color = colors.faint,
                    modifier = Modifier.padding(horizontal = 20.dp),
                    lineHeight = 17.sp,
                )
            }

            Spacer(modifier = Modifier.height(24.dp))
            Hairline(Modifier.padding(horizontal = 20.dp))
            Spacer(modifier = Modifier.height(12.dp))

            SettingsRow(
                title = "This Mac",
                detail = state.server.ifEmpty { "Not paired" },
            )
            SettingsRow(
                title = "Sync",
                detail = when {
                    state.pendingCount > 0 -> "${state.pendingCount} waiting"
                    state.syncing -> "Syncing now"
                    state.online -> "Up to date"
                    else -> "Offline"
                },
                action = "Sync now",
                onAction = onSync,
            )
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable(enabled = true) {
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
                    .padding(horizontal = 20.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Due reminders",
                        style = MaterialTheme.typography.labelLarge,
                        color = colors.ink,
                    )
                    Spacer(modifier = Modifier.height(2.dp))
                    Text(
                        text = when {
                            reminderDenied -> "Allow notifications in Android settings."
                            reminders -> "A summary when tasks are due today"
                            else -> "Off"
                        },
                        style = MaterialTheme.typography.labelMedium,
                        color = if (reminderDenied) colors.danger else colors.faint,
                    )
                }
                androidx.compose.material3.Switch(
                    checked = reminders,
                    onCheckedChange = null,
                    colors = androidx.compose.material3.SwitchDefaults.colors(
                        checkedThumbColor = colors.onAccent,
                        checkedTrackColor = colors.accent,
                        uncheckedThumbColor = colors.faint,
                        uncheckedTrackColor = colors.field,
                        uncheckedBorderColor = colors.hairline,
                    ),
                )
            }
            Hairline(Modifier.padding(horizontal = 20.dp))
            SettingsRow(
                title = "App",
                detail = "Flowlist for Android ${dev.flowlist.android.BuildConfig.VERSION_NAME}",
            )
            Spacer(modifier = Modifier.height(18.dp))
            Text(
                text = "Signing out forgets this phone's token. Your tasks stay on the Mac; pair again with a new code.",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                modifier = Modifier.padding(horizontal = 20.dp),
                lineHeight = 18.sp,
            )
            Spacer(modifier = Modifier.height(10.dp))
            Text(
                text = "Unpair this phone",
                style = MaterialTheme.typography.labelLarge,
                color = colors.danger,
                modifier = Modifier
                    .fillMaxWidth()
                    .clickable { confirmSignOut = true }
                    .padding(horizontal = 20.dp, vertical = 14.dp),
            )
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
private fun SectionTitle(text: String) {
    Text(
        text = text.uppercase(),
        style = MaterialTheme.typography.labelSmall,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.8.sp,
        color = flow.faint,
        modifier = Modifier.padding(horizontal = 20.dp, vertical = 8.dp),
    )
}

@Composable
private fun ChoicePill(
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
            .background(if (selected) colors.field else Color.Transparent)
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
            .width(138.dp)
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
            Box(
                modifier = Modifier
                    .size(14.dp)
                    .clip(CircleShape)
                    .background(preview.ink),
            )
            Box(
                modifier = Modifier
                    .size(14.dp)
                    .clip(CircleShape)
                    .background(preview.accent),
            )
            Box(
                modifier = Modifier
                    .size(14.dp)
                    .clip(CircleShape)
                    .border(1.dp, preview.ink.copy(alpha = 0.25f), CircleShape),
            )
        }
        Spacer(modifier = Modifier.height(12.dp))
        Text(
            text = family.label,
            style = MaterialTheme.typography.labelLarge,
            color = preview.ink,
            maxLines = 1,
        )
        Spacer(modifier = Modifier.height(1.dp))
        Text(
            text = family.hint,
            style = MaterialTheme.typography.labelSmall,
            color = preview.ink.copy(alpha = 0.55f),
            maxLines = 1,
        )
    }
}

@Composable
private fun SettingsRow(
    title: String,
    detail: String,
    action: String? = null,
    onAction: (() -> Unit)? = null,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp, vertical = 14.dp),
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
                color = colors.faint,
            )
        }
        if (action != null && onAction != null) {
            TextButton(onClick = onAction) {
                Text(action, color = colors.accent, style = MaterialTheme.typography.labelMedium)
            }
        }
    }
    Hairline(Modifier.padding(horizontal = 20.dp))
}
