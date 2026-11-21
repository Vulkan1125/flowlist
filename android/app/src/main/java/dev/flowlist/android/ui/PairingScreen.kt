package dev.flowlist.android.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun PairingScreen(
    busy: Boolean,
    error: String?,
    onPair: (String, String) -> Unit,
) {
    val colors = flow
    var address by rememberSaveable { mutableStateOf("") }
    var code by rememberSaveable { mutableStateOf("") }
    var help by rememberSaveable { mutableStateOf(false) }
    val canPair = address.isNotBlank() && code.trim().length >= 4 && !busy

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(colors.background)
            .imePadding()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 28.dp),
    ) {
        Spacer(modifier = Modifier.height(88.dp))
        Box(
            modifier = Modifier
                .size(62.dp)
                .clip(RoundedCornerShape(20.dp))
                .background(colors.accent),
            contentAlignment = Alignment.Center,
        ) {
            Icon(
                imageVector = Icons.Rounded.Check,
                contentDescription = null,
                tint = colors.onAccent,
                modifier = Modifier.size(30.dp),
            )
        }
        Spacer(modifier = Modifier.height(26.dp))
        Text(
            text = "Flowlist",
            style = MaterialTheme.typography.displaySmall,
            color = colors.ink,
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Your tasks stay on your Mac. This phone is a remote for that library — no account and nothing sent to a server.",
            style = MaterialTheme.typography.bodyMedium,
            color = colors.faint,
            lineHeight = 21.sp,
        )
        Spacer(modifier = Modifier.height(36.dp))

        Text(
            text = "Pair this phone",
            style = MaterialTheme.typography.titleMedium,
            color = colors.ink,
        )
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedTextField(
            value = address,
            onValueChange = { address = it },
            label = { Text("This Mac's address") },
            placeholder = { Text("192.168.1.33:7777") },
            singleLine = true,
            enabled = !busy,
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.Uri,
                imeAction = ImeAction.Next,
            ),
            shape = RoundedCornerShape(12.dp),
            colors = fieldColors(),
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(12.dp))
        OutlinedTextField(
            value = code,
            onValueChange = { value ->
                code = value.filter { it.isDigit() }.take(6)
            },
            label = { Text("Pairing code") },
            placeholder = { Text("123456") },
            singleLine = true,
            enabled = !busy,
            keyboardOptions = KeyboardOptions(
                keyboardType = KeyboardType.NumberPassword,
                imeAction = ImeAction.Done,
            ),
            shape = RoundedCornerShape(12.dp),
            colors = fieldColors(),
            modifier = Modifier.fillMaxWidth(),
        )

        if (error != null) {
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = error,
                style = MaterialTheme.typography.labelMedium,
                color = colors.danger,
            )
        }

        Spacer(modifier = Modifier.height(20.dp))
        Button(
            onClick = { onPair(address, code) },
            enabled = canPair,
            shape = RoundedCornerShape(12.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = colors.accent,
                contentColor = colors.onAccent,
                disabledContainerColor = colors.field,
                disabledContentColor = colors.faint,
            ),
            modifier = Modifier
                .fillMaxWidth()
                .height(52.dp),
        ) {
            if (busy) {
                CircularProgressIndicator(
                    modifier = Modifier.size(18.dp),
                    strokeWidth = 2.dp,
                    color = colors.faint,
                )
                Spacer(modifier = Modifier.size(10.dp))
                Text("Pairing…")
            } else {
                Text("Pair", fontWeight = FontWeight.Medium)
            }
        }

        Spacer(modifier = Modifier.height(22.dp))
        Row(
            modifier = Modifier
                .clip(RoundedCornerShape(8.dp))
                .clickable { help = !help }
                .padding(vertical = 6.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Text(
                text = "Where do I find these?",
                style = MaterialTheme.typography.labelLarge,
                color = colors.ink,
            )
            Text(
                text = if (help) "Hide" else "Show",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
        }
        AnimatedVisibility(visible = help) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(12.dp))
                    .background(colors.field)
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                HelpLine("1", "On your Mac, open Flowlist → Settings (⌘,) → Mobile.")
                HelpLine("2", "Turn on “Serve to my phone”. The login server keeps syncing when the window closes.")
                HelpLine("3", "Type one of the addresses shown there, then the six-digit pairing code.")
            }
            Spacer(modifier = Modifier.height(8.dp))
            Text(
                text = "Both devices need the same Wi-Fi, or both on Tailscale.",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
        }

        Spacer(modifier = Modifier.height(64.dp))
        Text(
            text = "Local-first · no cloud · no telemetry",
            style = MaterialTheme.typography.labelSmall,
            color = colors.faint,
            textAlign = TextAlign.Center,
            modifier = Modifier.fillMaxWidth(),
        )
        Spacer(modifier = Modifier.height(28.dp))
    }
}

@Composable
private fun HelpLine(number: String, text: String) {
    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Text(
            text = number,
            style = MaterialTheme.typography.labelSmall,
            color = flow.faint,
            modifier = Modifier.padding(top = 2.dp),
        )
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = flow.ink,
            lineHeight = 18.sp,
        )
    }
}

@Composable
private fun fieldColors() = OutlinedTextFieldDefaults.colors(
    focusedBorderColor = flow.accent,
    unfocusedBorderColor = flow.hairline,
    focusedLabelColor = flow.faint,
    unfocusedLabelColor = flow.faint,
    cursorColor = flow.accent,
    focusedTextColor = flow.ink,
    unfocusedTextColor = flow.ink,
    focusedContainerColor = flow.background,
    unfocusedContainerColor = flow.background,
    focusedPlaceholderColor = flow.faint,
    unfocusedPlaceholderColor = flow.faint,
)
