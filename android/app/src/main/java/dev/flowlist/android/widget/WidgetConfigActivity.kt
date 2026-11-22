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
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Surface
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
import androidx.glance.appwidget.updateAll
import androidx.lifecycle.lifecycleScope
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
    var blur by remember { mutableIntStateOf(initial.blur) }
    var showProject by remember { mutableStateOf(initial.showProject) }
    var showCompleted by remember { mutableStateOf(initial.showCompleted) }
    var quickAdd by remember { mutableStateOf(initial.quickAdd) }
    val save = {
        onSave(
            WidgetOptions(
                mode = mode,
                theme = theme,
                limit = limit,
                textSize = textSize,
                headerSize = headerSize,
                opacity = opacity,
                blur = blur,
                frosted = initial.frosted,
                showProject = showProject,
                showCompleted = showCompleted,
                quickAdd = quickAdd,
            ),
        )
    }
    Scaffold(
        containerColor = colors.background,
        topBar = {
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .statusBarsPadding()
                    .padding(start = 20.dp, end = 8.dp, top = 8.dp, bottom = 4.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Widget settings",
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.SemiBold,
                        color = colors.ink,
                    )
                    Text(
                        text = "Changes apply to this widget only.",
                        style = MaterialTheme.typography.bodySmall,
                        color = colors.faint,
                    )
                }
                IconButton(onClick = onCancel) {
                    Icon(Icons.Rounded.Close, contentDescription = "Cancel", tint = colors.faint)
                }
            }
        },
        bottomBar = {
            Surface(color = colors.background, tonalElevation = 2.dp) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .navigationBarsPadding()
                        .padding(horizontal = 20.dp, vertical = 12.dp),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TextButton(onClick = onCancel) {
                        Text("Cancel", color = colors.faint)
                    }
                    Spacer(modifier = Modifier.padding(start = 8.dp))
                    Button(
                        onClick = save,
                        colors = ButtonDefaults.buttonColors(
                            containerColor = colors.accent,
                            contentColor = colors.onAccent,
                        ),
                    ) {
                        Text(if (isNew) "Add widget" else "Save")
                    }
                }
            }
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 16.dp, vertical = 4.dp),
        ) {
            ConfigCard("Show") {
                ChoiceRow(
                    choices = listOf(
                        "today" to "Today",
                        "inbox" to "Inbox",
                        "upcoming" to "Upcoming",
                    ),
                    selected = mode,
                    onSelect = { mode = it },
                )
            }

            ConfigCard("Layout") {
                SmallLabel("Rows")
                ChoiceRow(
                    choices = listOf(3 to "3", 5 to "5", 8 to "8"),
                    selected = limit,
                    onSelect = { limit = it },
                    label = { it.second },
                    value = { it.first },
                )
                SmallLabel("Task text")
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
                SmallLabel("Header")
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
            }

            ConfigCard("Appearance") {
                SmallLabel("Theme")
                ChoiceRow(
                    choices = listOf(
                        "app" to "Follow app",
                        "light" to "Light",
                        "dark" to "Dark",
                    ),
                    selected = theme,
                    onSelect = { theme = it },
                )
                SliderRow(
                    title = "Opacity",
                    value = opacity,
                    range = 30..100,
                    onChange = { opacity = it },
                )
                SliderRow(
                    title = "Blur",
                    value = blur,
                    range = 0..100,
                    onChange = { blur = it },
                )
                Text(
                    text = "Blur adds a frosted veil; Android widgets cannot sample and blur the wallpaper itself.",
                    style = MaterialTheme.typography.bodySmall,
                    color = colors.faint,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }

            ConfigCard("Content") {
                ToggleRow("Show project names", showProject) { showProject = it }
                ToggleRow("Show completed tasks", showCompleted) { showCompleted = it }
                ToggleRow("Quick add button", quickAdd) { quickAdd = it }
            }
            Spacer(modifier = Modifier.height(12.dp))
        }
    }
}

@Composable
private fun ConfigCard(title: String, content: @Composable ColumnScope.() -> Unit) {
    val colors = flow
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = 12.dp),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = colors.raised),
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.labelLarge,
                fontWeight = FontWeight.SemiBold,
                color = colors.faint,
            )
            Spacer(modifier = Modifier.height(8.dp))
            content()
        }
    }
}

@Composable
private fun SmallLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        color = flow.faint,
        modifier = Modifier.padding(top = 12.dp, bottom = 6.dp),
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

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun <T> ChoiceRow(
    choices: List<Pair<T, String>>,
    selected: T,
    onSelect: (T) -> Unit,
    label: (Pair<T, String>) -> String,
    value: (Pair<T, String>) -> T,
) {
    val colors = flow
    FlowRow(
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
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
private fun SliderRow(
    title: String,
    value: Int,
    range: IntRange,
    onChange: (Int) -> Unit,
) {
    val colors = flow
    Column(modifier = Modifier.padding(top = 10.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(
                text = title,
                style = MaterialTheme.typography.bodyMedium,
                color = colors.ink,
                modifier = Modifier.weight(1f),
            )
            Text(
                text = "$value%",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
        }
        Slider(
            value = value.toFloat(),
            onValueChange = { onChange(it.roundToInt()) },
            valueRange = range.first.toFloat()..range.last.toFloat(),
            steps = 0,
            colors = SliderDefaults.colors(
                thumbColor = colors.accent,
                activeTrackColor = colors.accent,
                inactiveTrackColor = colors.hairline,
            ),
        )
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
