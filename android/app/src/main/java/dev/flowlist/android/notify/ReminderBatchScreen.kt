package dev.flowlist.android.notify

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
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.NotificationsActive
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.flowlist.android.ui.flow
import java.time.Instant
import java.time.ZoneId
import java.time.format.DateTimeFormatter

internal data class ReminderBatchItem(
    val id: String,
    val title: String,
    val project: String?,
    val dueAt: Long,
)

/// One card for several reminders due at once. It deliberately has no
/// complete or snooze controls: the tasks are handled in the app, and the
/// card either opens the read-only list or dismisses the whole batch.
@Composable
internal fun ReminderBatchScreen(
    count: Int,
    lightContent: Boolean,
    onViewAll: () -> Unit,
    onOpen: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val ink = if (lightContent) Color.White else colors.ink
    val faint = ink.copy(alpha = 0.68f)
    Box(modifier = Modifier.fillMaxSize()) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .navigationBarsPadding()
                .padding(horizontal = 24.dp, vertical = 18.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            DismissRow(ink = faint, onDismiss = onDismiss)
            Spacer(modifier = Modifier.weight(1f))
            Icon(
                imageVector = Icons.Outlined.NotificationsActive,
                contentDescription = null,
                tint = ink.copy(alpha = 0.9f),
                modifier = Modifier.size(32.dp),
            )
            Spacer(modifier = Modifier.height(18.dp))
            Text(
                text = if (count == 1) "1 reminder due" else "$count reminders due",
                color = ink,
                fontSize = 30.sp,
                fontWeight = FontWeight.SemiBold,
                textAlign = TextAlign.Center,
            )
            Spacer(modifier = Modifier.height(10.dp))
            Text(
                text = "View them at a glance, or dismiss them all.",
                color = faint,
                fontSize = 14.sp,
                textAlign = TextAlign.Center,
            )
            Spacer(modifier = Modifier.weight(1.1f))
            Row(
                horizontalArrangement = Arrangement.spacedBy(10.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = "View all",
                    color = ink,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.Medium,
                    modifier = Modifier
                        .clip(RoundedCornerShape(percent = 50))
                        .background(ink.copy(alpha = 0.10f))
                        .clickable(onClick = onViewAll)
                        .padding(horizontal = 20.dp, vertical = 11.dp),
                )
                Text(
                    text = "Open Flowlist",
                    color = ink,
                    fontSize = 15.sp,
                    fontWeight = FontWeight.Medium,
                    modifier = Modifier
                        .clip(RoundedCornerShape(percent = 50))
                        .background(ink.copy(alpha = 0.16f))
                        .clickable(onClick = onOpen)
                        .padding(horizontal = 20.dp, vertical = 11.dp),
                )
            }
        }
    }
}

/// The read-only batch list. It only shows what is due; both buttons clear
/// the whole batch.
@Composable
internal fun ReminderBatchListScreen(
    items: List<ReminderBatchItem>,
    lightContent: Boolean,
    onBack: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val ink = if (lightContent) Color.White else colors.ink
    val faint = ink.copy(alpha = 0.62f)
    val card = if (lightContent) Color.White.copy(alpha = 0.14f) else colors.field
    Column(
        modifier = Modifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(horizontal = 20.dp, vertical = 16.dp),
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text(
                    text = "Due now",
                    color = ink,
                    fontSize = 28.sp,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = if (items.size == 1) "1 reminder" else "${items.size} reminders",
                    color = faint,
                    fontSize = 13.sp,
                )
            }
            Text(
                text = "Back",
                color = faint,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
                modifier = Modifier
                    .clip(RoundedCornerShape(percent = 50))
                    .background(card)
                    .clickable(onClick = onBack)
                    .padding(horizontal = 16.dp, vertical = 9.dp),
            )
        }
        Spacer(modifier = Modifier.height(16.dp))
        Column(
            modifier = Modifier
                .weight(1f)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState()),
        ) {
            items.forEach { item ->
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .clip(RoundedCornerShape(16.dp))
                        .background(card)
                        .padding(horizontal = 16.dp, vertical = 12.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = item.title,
                            color = ink,
                            fontSize = 16.sp,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                        Text(
                            text = "${batchTime(item.dueAt)} · ${item.project ?: "Inbox"}",
                            color = faint,
                            fontSize = 12.sp,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
                Spacer(modifier = Modifier.height(8.dp))
            }
        }
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Dismiss all",
            color = ink,
            fontSize = 15.sp,
            fontWeight = FontWeight.Medium,
            modifier = Modifier
                .align(Alignment.CenterHorizontally)
                .clip(RoundedCornerShape(percent = 50))
                .background(ink.copy(alpha = 0.14f))
                .clickable(onClick = onDismiss)
                .padding(horizontal = 22.dp, vertical = 11.dp),
        )
    }
}

@Composable
private fun DismissRow(ink: Color, onDismiss: () -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.End,
    ) {
        Row(
            modifier = Modifier
                .clip(RoundedCornerShape(percent = 50))
                .clickable(onClick = onDismiss)
                .padding(horizontal = 12.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                imageVector = Icons.Rounded.Close,
                contentDescription = null,
                tint = ink,
                modifier = Modifier.size(17.dp),
            )
            Spacer(modifier = Modifier.width(5.dp))
            Text(
                text = "Dismiss",
                color = ink,
                fontSize = 14.sp,
                fontWeight = FontWeight.Medium,
            )
        }
    }
}

private fun batchTime(dueAt: Long): String =
    Instant.ofEpochMilli(dueAt)
        .atZone(ZoneId.systemDefault())
        .format(DateTimeFormatter.ofPattern("HH:mm"))
