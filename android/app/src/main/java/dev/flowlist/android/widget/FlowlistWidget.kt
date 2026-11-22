package dev.flowlist.android.widget

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.action.actionParametersOf
import androidx.glance.appwidget.action.actionStartActivity
import androidx.glance.appwidget.action.actionRunCallback
import androidx.glance.action.clickable
import androidx.glance.appwidget.CheckboxDefaults
import androidx.glance.appwidget.CheckBox
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetManager
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.cornerRadius
import androidx.glance.appwidget.lazy.LazyColumn
import androidx.glance.appwidget.lazy.items
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.layout.Alignment
import androidx.glance.layout.Box
import androidx.glance.layout.Column
import androidx.glance.layout.Row
import androidx.glance.layout.Spacer
import androidx.glance.layout.height
import androidx.glance.layout.width
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.fillMaxWidth
import androidx.glance.layout.padding
import androidx.glance.layout.size
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextDecoration
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import dev.flowlist.android.MainActivity
import dev.flowlist.android.data.FileStore
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.data.label
import dev.flowlist.android.data.widgetTasks
import dev.flowlist.android.ui.themeColors
import java.time.LocalDate
import java.time.format.DateTimeFormatter
import java.util.Locale

class FlowlistWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val appWidgetId = runCatching {
            GlanceAppWidgetManager(context).getAppWidgetId(id)
        }.getOrDefault(-1)
        val options = if (appWidgetId >= 0) {
            WidgetOptionsStore(context).load(appWidgetId)
        } else {
            WidgetOptions()
        }
        val snapshot = FileStore(context).loadSnapshot()
        val allTasks = snapshot?.db?.widgetTasks(options.mode, options.showCompleted).orEmpty()
        val tasks = allTasks.take(options.limit)
        val openCount = allTasks.count { !it.completed }
        val themeSettings = ThemeStore(context).load()
        val systemDark =
            (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
                Configuration.UI_MODE_NIGHT_YES
        val dark = when (options.theme) {
            "light" -> false
            "dark" -> true
            else -> systemDark
        }
        val colors = themeColors(themeSettings, dark)
        val scale = when (options.textSize) {
            "compact" -> 0.9f
            "large" -> 1.25f
            "xlarge" -> 1.5f
            else -> 1f
        }
        val alpha = (options.opacity / 100f).coerceIn(0.3f, 1f)
        val tinted = if (options.frosted) {
            blend(colors.raised, colors.field, 0.5f).copy(alpha = alpha.coerceAtMost(0.82f))
        } else {
            colors.raised.copy(alpha = alpha)
        }
        val openApp = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
        }
        val quickAdd = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("capture", true)
        }
        provideContent {
            Column(
                modifier = GlanceModifier
                    .fillMaxSize()
                    .background(ColorProvider(tinted))
                    .cornerRadius(20.dp)
                    .padding(14.dp),
                verticalAlignment = Alignment.Top,
            ) {
                Row(
                    modifier = GlanceModifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Spacer(
                        modifier = GlanceModifier
                            .width(3.dp)
                            .height((30f * scale).dp)
                            .cornerRadius(2.dp)
                            .background(ColorProvider(colors.accent)),
                    )
                    Spacer(modifier = GlanceModifier.width(10.dp))
                    Column(modifier = GlanceModifier.defaultWeight()) {
                        Text(
                            text = options.title,
                            style = TextStyle(
                                color = ColorProvider(colors.ink),
                                fontSize = (21f * scale).sp,
                                fontWeight = FontWeight.Bold,
                            ),
                            maxLines = 1,
                        )
                        Text(
                            text = headerSubtitle(options, openCount),
                            style = TextStyle(
                                color = ColorProvider(colors.faint),
                                fontSize = (13f * scale).sp,
                            ),
                            modifier = GlanceModifier.padding(top = 1.dp),
                            maxLines = 1,
                        )
                    }
                    if (options.quickAdd) {
                        Box(
                            modifier = GlanceModifier
                                .size((34f * scale).dp)
                                .cornerRadius((17f * scale).dp)
                                .background(ColorProvider(colors.accent))
                                .clickable(actionStartActivity(quickAdd)),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "+",
                                style = TextStyle(
                                    color = ColorProvider(colors.onAccent),
                                    fontSize = (22f * scale).sp,
                                    fontWeight = FontWeight.Bold,
                                ),
                            )
                        }
                    }
                }
                Spacer(modifier = GlanceModifier.height(8.dp))
                Spacer(
                    modifier = GlanceModifier
                        .fillMaxWidth()
                        .height(1.dp)
                        .background(ColorProvider(colors.hairline)),
                )
                if (tasks.isEmpty()) {
                    if (options.quickAdd) {
                        Text(
                            text = "Tap + to capture",
                            style = TextStyle(
                                color = ColorProvider(colors.faint),
                                fontSize = (13f * scale).sp,
                            ),
                            modifier = GlanceModifier.padding(top = 12.dp),
                        )
                    }
                } else {
                    LazyColumn(
                        modifier = GlanceModifier
                            .fillMaxWidth()
                            .defaultWeight()
                            .padding(top = 6.dp),
                    ) {
                        items(tasks) { task ->
                            Row(
                                modifier = GlanceModifier
                                    .fillMaxWidth()
                                    .padding(vertical = 3.dp),
                                verticalAlignment = Alignment.CenterVertically,
                            ) {
                                CheckBox(
                                    checked = task.completed,
                                    onCheckedChange = if (task.completed) {
                                        null
                                    } else {
                                        actionRunCallback<
                                            CompleteWidgetTaskAction
                                            >(
                                            actionParametersOf(WidgetTaskKey to task.id),
                                        )
                                    },
                                    text = "",
                                    colors = CheckboxDefaults.colors(
                                        checkedColor = ColorProvider(colors.accent),
                                        uncheckedColor = ColorProvider(colors.faint),
                                    ),
                                )
                                Column(
                                    modifier = GlanceModifier
                                        .defaultWeight()
                                        .clickable(actionStartActivity(openApp)),
                                ) {
                                    Text(
                                        text = task.title,
                                        style = TextStyle(
                                            color = ColorProvider(
                                                if (task.completed) colors.faint else colors.ink,
                                            ),
                                            fontSize = (14f * scale).sp,
                                            textDecoration = if (task.completed) {
                                                TextDecoration.LineThrough
                                            } else {
                                                TextDecoration.None
                                            },
                                        ),
                                        maxLines = 2,
                                    )
                                    val detail = detail(task, options.showProject)
                                    if (detail != null) {
                                        Text(
                                            text = detail,
                                            style = TextStyle(
                                                color = ColorProvider(colors.faint),
                                                fontSize = (11f * scale).sp,
                                            ),
                                            maxLines = 1,
                                        )
                                    }
                                }
                                if (!task.completed) {
                                    Text(
                                        text = "Tmrw",
                                        style = TextStyle(
                                            color = ColorProvider(colors.faint),
                                            fontSize = (11f * scale).sp,
                                        ),
                                        modifier = GlanceModifier
                                            .padding(start = 4.dp)
                                            .clickable(
                                                actionRunCallback<
                                                    RescheduleWidgetTaskAction
                                                    >(
                                                    actionParametersOf(WidgetTaskKey to task.id),
                                                ),
                                            ),
                                    )
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun headerSubtitle(options: WidgetOptions, open: Int): String {
    val count = when {
        open == 0 -> "All clear"
        open == 1 -> "1 task"
        else -> "$open tasks"
    }
    val tail = when (options.mode) {
        "inbox" -> "captured"
        "upcoming" -> "next two weeks"
        else -> LocalDate.now().format(
            DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()),
        )
    }
    return "$count · $tail"
}

private fun detail(
    task: dev.flowlist.android.data.TaskView,
    showProject: Boolean,
): String? {
    val parts = buildList {
        if (showProject) task.projectName?.let { add(it) }
        task.due?.label()?.takeIf { it.isNotBlank() }?.let { add(it) }
    }
    return parts.joinToString(" · ").takeIf { it.isNotBlank() }
}

private fun blend(base: Color, tint: Color, amount: Float): Color = Color(
    red = base.red + (tint.red - base.red) * amount,
    green = base.green + (tint.green - base.green) * amount,
    blue = base.blue + (tint.blue - base.blue) * amount,
    alpha = base.alpha,
)

class FlowlistWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = FlowlistWidget()

    override fun onDeleted(context: Context, appWidgetIds: IntArray) {
        super.onDeleted(context, appWidgetIds)
        WidgetOptionsStore(context).delete(appWidgetIds)
    }
}
