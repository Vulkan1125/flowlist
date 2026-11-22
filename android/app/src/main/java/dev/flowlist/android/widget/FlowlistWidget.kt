package dev.flowlist.android.widget

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.Image
import androidx.glance.ImageProvider
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
import androidx.glance.layout.ContentScale
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
import dev.flowlist.android.R
import dev.flowlist.android.data.FileStore
import dev.flowlist.android.data.TaskView
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
        val headerScale = when (options.headerSize) {
            "compact" -> 0.85f
            "large" -> 1.15f
            "xlarge" -> 1.3f
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
                    Image(
                        provider = ImageProvider(R.mipmap.ic_launcher_red),
                        contentDescription = null,
                        modifier = GlanceModifier
                            .size((24f * headerScale).dp)
                            .cornerRadius(7.dp),
                        contentScale = ContentScale.Fit,
                    )
                    Spacer(modifier = GlanceModifier.width(8.dp))
                    Text(
                        text = options.title,
                        style = TextStyle(
                            color = ColorProvider(colors.ink),
                            fontSize = (18f * headerScale).sp,
                            fontWeight = FontWeight.Bold,
                        ),
                        maxLines = 1,
                    )
                    if (openCount > 0) {
                        Spacer(modifier = GlanceModifier.width(7.dp))
                        Box(
                            modifier = GlanceModifier
                                .cornerRadius(8.dp)
                                .background(ColorProvider(colors.accent.copy(alpha = 0.16f)))
                                .padding(horizontal = 6.dp, vertical = 2.dp),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "$openCount",
                                style = TextStyle(
                                    color = ColorProvider(colors.accent),
                                    fontSize = (11f * headerScale).sp,
                                    fontWeight = FontWeight.Bold,
                                ),
                            )
                        }
                    }
                    Spacer(modifier = GlanceModifier.defaultWeight())
                    if (options.quickAdd) {
                        Box(
                            modifier = GlanceModifier
                                .size((30f * headerScale).dp)
                                .cornerRadius((15f * headerScale).dp)
                                .background(ColorProvider(colors.accent))
                                .clickable(actionStartActivity(quickAdd)),
                            contentAlignment = Alignment.Center,
                        ) {
                            Text(
                                text = "+",
                                style = TextStyle(
                                    color = ColorProvider(colors.onAccent),
                                    fontSize = (20f * headerScale).sp,
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
                            .padding(top = 4.dp),
                    ) {
                        widgetSections(options.mode, tasks).forEach { section ->
                            section.label?.let { label ->
                                item {
                                    Text(
                                        text = label,
                                        style = TextStyle(
                                            color = ColorProvider(
                                                if (label == "Overdue") colors.danger else colors.faint,
                                            ),
                                            fontSize = (11f * scale).sp,
                                            fontWeight = FontWeight.Bold,
                                        ),
                                        modifier = GlanceModifier.padding(top = 7.dp, bottom = 2.dp),
                                        maxLines = 1,
                                    )
                                }
                            }
                            items(section.tasks) { task ->
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
                                        val due = task.due?.label()?.takeIf { it.isNotBlank() }
                                        if (due != null) {
                                            val overdue = !task.completed && task.due
                                                ?.local()
                                                ?.toLocalDate()
                                                ?.isBefore(LocalDate.now()) == true
                                            Text(
                                                text = due,
                                                style = TextStyle(
                                                    color = ColorProvider(
                                                        if (overdue) colors.danger else colors.faint,
                                                    ),
                                                    fontSize = (11f * scale).sp,
                                                ),
                                                maxLines = 1,
                                            )
                                        }
                                    }
                                    if (options.showProject) {
                                        val project = if (options.mode == "inbox") {
                                            task.projectName
                                        } else {
                                            task.projectName ?: "Inbox"
                                        }
                                        if (project != null) {
                                            Spacer(modifier = GlanceModifier.width(6.dp))
                                            Text(
                                                text = project,
                                                style = TextStyle(
                                                    color = ColorProvider(colors.faint),
                                                    fontSize = (10f * scale).sp,
                                                ),
                                                maxLines = 1,
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
    }
}

private data class WidgetSection(val label: String?, val tasks: List<TaskView>)

private fun widgetSections(mode: String, tasks: List<TaskView>): List<WidgetSection> {
    if (tasks.isEmpty()) return emptyList()
    if (mode == "inbox") return listOf(WidgetSection(null, tasks))
    val today = LocalDate.now()
    if (mode == "upcoming") {
        return tasks
            .groupBy { it.due?.local()?.toLocalDate() ?: today }
            .toSortedMap()
            .map { (date, items) -> WidgetSection(dayLabel(date, today), items) }
    }
    val overdue = tasks.filter { task ->
        !task.completed && task.due?.local()?.toLocalDate()?.isBefore(today) == true
    }
    val overdueIds = overdue.map { it.id }.toSet()
    val todayTasks = tasks.filterNot { it.id in overdueIds }
    return buildList {
        if (overdue.isNotEmpty()) add(WidgetSection("Overdue", overdue))
        if (todayTasks.isNotEmpty()) add(WidgetSection(dayLabel(today, today), todayTasks))
    }
}

private fun dayLabel(date: LocalDate, today: LocalDate): String = when (date) {
    today -> date.format(DateTimeFormatter.ofPattern("EEEE, d MMM", Locale.getDefault()))
    today.plusDays(1) -> "Tomorrow"
    else -> date.format(DateTimeFormatter.ofPattern("EEE, d MMM", Locale.getDefault()))
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
