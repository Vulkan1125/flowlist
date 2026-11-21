package dev.flowlist.android.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AssistChip
import androidx.compose.material3.AssistChipDefaults
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CaptureSheet(onDismiss: () -> Unit, onAdd: (String) -> Unit) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    var text by rememberSaveable { mutableStateOf("") }
    val focus = remember { FocusRequester() }

    LaunchedEffect(Unit) {
        delay(220)
        focus.requestFocus()
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
                .imePadding()
                .padding(horizontal = 20.dp)
                .padding(bottom = 26.dp),
        ) {
            Text(
                text = "New task",
                style = MaterialTheme.typography.titleLarge,
                color = colors.ink,
            )
            Spacer(modifier = Modifier.height(14.dp))
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                placeholder = {
                    Text(
                        "What needs doing?",
                        style = TextStyle(fontSize = 18.sp),
                        color = colors.faint,
                    )
                },
                textStyle = TextStyle(fontSize = 18.sp, lineHeight = 25.sp, color = colors.ink),
                minLines = 1,
                maxLines = 4,
                shape = RoundedCornerShape(14.dp),
                colors = OutlinedTextFieldDefaults.colors(
                    focusedBorderColor = colors.accent,
                    unfocusedBorderColor = colors.hairline,
                    cursorColor = colors.accent,
                    focusedContainerColor = colors.background,
                    unfocusedContainerColor = colors.background,
                ),
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(
                    onDone = {
                        if (text.isNotBlank()) onAdd(text)
                    },
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .focusRequester(focus),
            )
            Spacer(modifier = Modifier.height(12.dp))
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                listOf("due:today", "est:30m", "#errand", "::Project").forEach { chip ->
                    AssistChip(
                        onClick = { text = (text.trimEnd() + " " + chip).trimStart() },
                        label = {
                            Text(
                                chip,
                                style = MaterialTheme.typography.labelSmall,
                                color = colors.faint,
                            )
                        },
                        shape = RoundedCornerShape(8.dp),
                        border = AssistChipDefaults.assistChipBorder(
                            enabled = true,
                            borderColor = colors.hairline,
                        ),
                        colors = AssistChipDefaults.assistChipColors(
                            containerColor = colors.background,
                        ),
                    )
                }
            }
            Spacer(modifier = Modifier.height(18.dp))
            Button(
                onClick = { if (text.isNotBlank()) onAdd(text) },
                enabled = text.isNotBlank(),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = colors.accent,
                    contentColor = colors.onAccent,
                    disabledContainerColor = colors.field,
                    disabledContentColor = colors.faint,
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .height(50.dp),
            ) {
                Text("Add to Inbox")
            }
            Spacer(modifier = Modifier.height(10.dp))
            Text(
                text = "It stays in the Inbox unless you add a date or project. The Mac parses every chip, so nothing is lost.",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
                textAlign = TextAlign.Center,
                modifier = Modifier.fillMaxWidth(),
                lineHeight = 17.sp,
            )
        }
    }
}
