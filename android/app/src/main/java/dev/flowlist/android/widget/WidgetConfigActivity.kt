package dev.flowlist.android.widget

import android.app.Activity
import android.appwidget.AppWidgetManager
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.lifecycleScope
import androidx.glance.appwidget.updateAll
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.ui.FlowlistTheme
import dev.flowlist.android.ui.flow
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/// Runs when a Flowlist widget is placed or reconfigured. The choices are
/// stored per app-widget id, so two widgets can show different perspectives.
class WidgetConfigActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val appWidgetId = intent?.extras?.getInt(
            AppWidgetManager.EXTRA_APPWIDGET_ID,
            AppWidgetManager.INVALID_APPWIDGET_ID,
        ) ?: AppWidgetManager.INVALID_APPWIDGET_ID
        if (appWidgetId == AppWidgetManager.INVALID_APPWIDGET_ID) {
            finish()
            return
        }
        setResult(Activity.RESULT_CANCELED)
        val store = WidgetOptionsStore(this)
        val initial = store.load(appWidgetId)
        val isNew = !store.exists(appWidgetId)
        val theme = ThemeStore(this).load()
        setContent {
            FlowlistTheme(theme) {
                WidgetConfigScreen(
                    initial = initial,
                    isNew = isNew,
                    onSave = { options ->
                        store.save(appWidgetId, options)
                        lifecycleScope.launch {
                            FlowlistWidget().updateAll(applicationContext)
                            setResult(
                                Activity.RESULT_OK,
                                Intent().putExtra(
                                    AppWidgetManager.EXTRA_APPWIDGET_ID,
                                    appWidgetId,
                                ),
                            )
                            finish()
                        }
                    },
                    onCancel = { finish() },
                )
            }
        }
    }
}

@Composable
private fun WidgetConfigScreen(
    initial: WidgetOptions,
    isNew: Boolean,
    onSave: (WidgetOptions) -> Unit,
    onCancel: () -> Unit,
) {
    val colors = flow
    var mode by remember { mutableStateOf(initial.mode) }
    var theme by remember { mutableStateOf(initial.theme) }
    var limit by remember { mutableIntStateOf(initial.limit) }
    var textSize by remember { mutableStateOf(initial.textSize) }
    var headerSize by remember { mutableStateOf(initial.headerSize) }
    var opacity by remember { mutableIntStateOf(initial.opacity) }
    var frosted by remember { mutableStateOf(initial.frosted) }
    var showProject by remember { mutableStateOf(initial.showProject) }
    var showCompleted by remember { mutableStateOf(initial.showCompleted) }
    var quickAdd by remember { mutableStateOf(initial.quickAdd) }
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 22.dp, vertical = 24.dp),
    ) {
        Text(
            text = "Flowlist widget",
            style = MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.SemiBold,
            color = colors.ink,
        )
        Text(
            text = "Choose what this widget shows. Each widget keeps its own settings.",
            style = MaterialTheme.typography.bodyMedium,
            color = colors.faint,
            modifier = Modifier.padding(top = 4.dp, bottom = 22.dp),
        )

        SectionLabel("Show")
        ChoiceRow(
            choices = listOf(
                "today" to "Today",
                "inbox" to "Inbox",
                "upcoming" to "Upcoming",
            ),
            selected = mode,
            onSelect = { mode = it },
        )

        SectionLabel("Theme")
        ChoiceRow(
            choices = listOf(
                "app" to "Follow app",
                "light" to "Light",
                "dark" to "Dark",
            ),
            selected = theme,
            onSelect = { theme = it },
        )

        SectionLabel("Tasks")
        ChoiceRow(
            choices = listOf(
                3 to "3 rows",
                5 to "5 rows",
                8 to "8 rows",
            ),
            selected = limit,
            onSelect = { limit = it },
            label = { it.second },
            value = { it.first },
        )

        SectionLabel("Text size")
        ChoiceRow(
            choices = listOf(
                "compact" to "Compact",
                "default" to "Default",
                "large" to "Large",
                "xlarge" to "Extra large",
            ),
            selected = textSize,
            onSelect = { textSize = it },
        )

        SectionLabel("Header size")
        ChoiceRow(
            choices = listOf(
                "compact" to "Compact",
                "default" to "Default",
                "large" to "Large",
                "xlarge" to "Extra large",
            ),
            selected = headerSize,
            onSelect = { headerSize = it },
        )

        SectionLabel("Background")
        Text(
            text = "Opacity · $opacity%",
            style = MaterialTheme.typography.bodyMedium,
            color = colors.ink,
        )
        Slider(
            value = opacity.toFloat(),
            onValueChange = { opacity = it.roundToInt() },
            valueRange = 30f..100f,
            steps = 6,
            colors = androidx.compose.material3.SliderDefaults.colors(
                thumbColor = colors.accent,
                activeTrackColor = colors.accent,
                inactiveTrackColor = colors.hairline,
            ),
        )
        ToggleRow("Frosted glass", frosted) { frosted = it }
        Text(
            text = "A softer translucent tint. Android widgets cannot blur the wallpaper behind them.",
            style = MaterialTheme.typography.bodySmall,
            color = colors.faint,
            modifier = Modifier.padding(bottom = 6.dp),
        )

        Spacer(modifier = Modifier.height(8.dp))
        ToggleRow("Show project names", showProject) { showProject = it }
        ToggleRow("Show completed tasks", showCompleted) { showCompleted = it }
        ToggleRow("Quick add button", quickAdd) { quickAdd = it }

        Spacer(modifier = Modifier.height(26.dp))
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.End,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            TextButton(onClick = onCancel) {
                Text("Cancel", color = colors.faint)
            }
            Button(
                onClick = {
                    onSave(
                        WidgetOptions(
                            mode = mode,
                            theme = theme,
                            limit = limit,
                            textSize = textSize,
                            headerSize = headerSize,
                            opacity = opacity,
                            frosted = frosted,
                            showProject = showProject,
                            showCompleted = showCompleted,
                            quickAdd = quickAdd,
                        ),
                    )
                },
                colors = ButtonDefaults.buttonColors(
                    containerColor = colors.accent,
                    contentColor = colors.onAccent,
                ),
                modifier = Modifier.padding(start = 8.dp),
            ) {
                Text(if (isNew) "Add widget" else "Save")
            }
        }
    }
}

@Composable
private fun SectionLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.SemiBold,
        color = flow.faint,
        modifier = Modifier.padding(top = 18.dp, bottom = 8.dp),
    )
}

@Composable
private fun ChoiceRow(
    choices: List<Pair<String, String>>,
    selected: String,
    onSelect: (String) -> Unit,
) {
    ChoiceRow(
        choices = choices,
        selected = selected,
        onSelect = onSelect,
        label = { it.second },
        value = { it.first },
    )
}

@Composable
private fun <T> ChoiceRow(
    choices: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    label: (Pair<T, String>) -> String,
    value: (Pair<T, String>) -> T,
) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        choices.forEach { choice ->
            val chosen = value(choice) == selected
            Text(
                text = label(choice),
                style = MaterialTheme.typography.labelLarge,
                color = if (chosen) colors.accent else colors.faint,
                modifier = Modifier
                    .background(
                        if (chosen) colors.field else Color.Transparent,
                        RoundedCornerShape(9.dp),
                    )
                    .border(
                        1.dp,
                        if (chosen) colors.accent else colors.hairline,
                        RoundedCornerShape(9.dp),
                    )
                    .clickable { onSelect(value(choice)) }
                    .padding(horizontal = 12.dp, vertical = 8.dp),
            )
        }
    }
}

@Composable
private fun ToggleRow(label: String, checked: Boolean, onChange: (Boolean) -> Unit) {
    val colors = flow
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clickable { onChange(!checked) }
            .padding(vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.ink,
            modifier = Modifier.weight(1f),
        )
        Switch(checked = checked, onCheckedChange = onChange)
    }
}
