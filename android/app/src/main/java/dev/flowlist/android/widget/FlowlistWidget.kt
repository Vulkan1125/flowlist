package dev.flowlist.android.widget

import android.content.ComponentName
import android.content.Context
import android.content.res.Configuration
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.glance.GlanceId
import androidx.glance.GlanceModifier
import androidx.glance.action.actionStartActivity
import androidx.glance.action.clickable
import androidx.glance.appwidget.GlanceAppWidget
import androidx.glance.appwidget.GlanceAppWidgetReceiver
import androidx.glance.appwidget.provideContent
import androidx.glance.background
import androidx.glance.appwidget.cornerRadius
import androidx.glance.layout.Alignment
import androidx.glance.layout.Column
import androidx.glance.layout.fillMaxSize
import androidx.glance.layout.padding
import androidx.glance.text.FontWeight
import androidx.glance.text.Text
import androidx.glance.text.TextStyle
import androidx.glance.unit.ColorProvider
import dev.flowlist.android.MainActivity
import dev.flowlist.android.data.FileStore
import dev.flowlist.android.data.ThemeStore
import dev.flowlist.android.data.inbox
import dev.flowlist.android.data.today
import dev.flowlist.android.ui.ThemeSettings
import dev.flowlist.android.ui.themeColors

class FlowlistWidget : GlanceAppWidget() {
    override suspend fun provideGlance(context: Context, id: GlanceId) {
        val snapshot = FileStore(context).loadSnapshot()
        val colors = themeColors(
            ThemeStore(context).load(),
            (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
                Configuration.UI_MODE_NIGHT_YES,
        )
        val inbox = snapshot?.db?.inbox()?.size ?: 0
        val today = snapshot?.db?.today()?.size ?: 0
        provideContent {
            Column(
                modifier = GlanceModifier
                    .fillMaxSize()
                    .background(ColorProvider(colors.raised))
                    .cornerRadius(20.dp)
                    .padding(14.dp)
                    .clickable(
                        actionStartActivity(
                            ComponentName(context, MainActivity::class.java),
                        ),
                    ),
                verticalAlignment = Alignment.Top,
            ) {
                Text(
                    text = "Flowlist",
                    style = TextStyle(
                        color = ColorProvider(colors.accent),
                        fontSize = 12.sp,
                        fontWeight = FontWeight.Bold,
                    ),
                )
                Text(
                    text = if (today > 0) "$today due today" else "Nothing due today",
                    style = TextStyle(
                        color = ColorProvider(colors.ink),
                        fontSize = 19.sp,
                        fontWeight = FontWeight.Bold,
                    ),
                    modifier = GlanceModifier.padding(top = 4.dp),
                )
                Text(
                    text = "$inbox in Inbox",
                    style = TextStyle(
                        color = ColorProvider(colors.faint),
                        fontSize = 12.sp,
                    ),
                    modifier = GlanceModifier.padding(top = 3.dp),
                )
            }
        }
    }
}

class FlowlistWidgetReceiver : GlanceAppWidgetReceiver() {
    override val glanceAppWidget: GlanceAppWidget = FlowlistWidget()
}
