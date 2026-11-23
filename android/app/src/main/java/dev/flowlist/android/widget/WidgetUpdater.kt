package dev.flowlist.android.widget

import android.appwidget.AppWidgetManager
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/// Ask the widget receiver to rebuild every placed Flowlist widget. Going
/// through APPWIDGET_UPDATE starts a fresh Glance session, unlike calling
/// GlanceAppWidget.update from a background thread, which can report success
/// without re-running provideGlance.
object WidgetUpdater {
    suspend fun refresh(context: Context) = withContext(Dispatchers.IO) {
        val app = context.applicationContext
        val ids = AppWidgetManager.getInstance(app).getAppWidgetIds(
            ComponentName(app, FlowlistWidgetReceiver::class.java),
        )
        if (ids.isEmpty()) return@withContext
        val intent = Intent(app, FlowlistWidgetReceiver::class.java).apply {
            action = AppWidgetManager.ACTION_APPWIDGET_UPDATE
            putExtra(AppWidgetManager.EXTRA_APPWIDGET_IDS, ids)
        }
        app.sendBroadcast(intent)
    }
}
