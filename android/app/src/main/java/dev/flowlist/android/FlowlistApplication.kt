package dev.flowlist.android

import android.app.Application
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.res.Configuration
import androidx.core.content.ContextCompat
import dev.flowlist.android.widget.WidgetUpdater
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/// The widget colours are baked into RemoteViews, so a system light/dark
/// change needs a repaint. Application callbacks fire while the process is
/// cached, which is enough to keep the widget in step without a service.
class FlowlistApplication : Application() {
    private val themeReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            repaintWidgets()
        }
    }

    override fun onCreate() {
        super.onCreate()
        val filter = IntentFilter().apply {
            addAction(Intent.ACTION_CONFIGURATION_CHANGED)
            addAction("android.intent.action.UI_MODE_CHANGED")
        }
        ContextCompat.registerReceiver(this, themeReceiver, filter, ContextCompat.RECEIVER_EXPORTED)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        repaintWidgets()
    }

    private fun repaintWidgets() {
        CoroutineScope(Dispatchers.IO).launch {
            runCatching { WidgetUpdater.refresh(this@FlowlistApplication) }
        }
    }
}
