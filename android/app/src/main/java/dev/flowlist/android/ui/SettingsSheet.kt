package dev.flowlist.android.ui

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.data.AppState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsSheet(
    state: AppState,
    onDismiss: () -> Unit,
    onSync: () -> Unit,
    onSignOut: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var confirmSignOut by remember { mutableStateOf(false) }

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(modifier = Modifier.fillMaxWidth().padding(bottom = 30.dp)) {
            Text(
                text = "Settings",
                style = MaterialTheme.typography.headlineMedium,
                color = colors.ink,
                modifier = Modifier.padding(horizontal = 20.dp),
            )
            Spacer(modifier = Modifier.height(18.dp))
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
            SettingsRow(
                title = "Library",
                detail = "Flowlist for Android ${dev.flowlist.android.BuildConfig.VERSION_NAME}",
            )
            Spacer(modifier = Modifier.height(20.dp))
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
                Text(action, color = colors.ink, style = MaterialTheme.typography.labelMedium)
            }
        }
    }
    Hairline(Modifier.padding(horizontal = 20.dp))
}
