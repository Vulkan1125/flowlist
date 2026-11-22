package dev.flowlist.android.widget

import android.content.Context
import androidx.glance.GlanceId
import androidx.glance.action.ActionParameters
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.updateAll
import dev.flowlist.android.data.Repository

/// The task id carried by a widget row's check button.
val WidgetTaskKey = ActionParameters.Key<String>("widget_task_id")

/// The app-widget id for actions that edit this widget's options.
val WidgetIdKey = ActionParameters.Key<Int>("widget_id")

/// A mode picked from the inline chooser.
val WidgetModeKey = ActionParameters.Key<String>("widget_mode")

class CompleteWidgetTaskAction : ActionCallback {
    override suspend fun onAction(
        context: Context,
        glanceId: GlanceId,
        parameters: ActionParameters,
    ) {
        val id = parameters[WidgetTaskKey] ?: return
        runCatching { Repository(context.applicationContext).complete(id, true) }
        FlowlistWidget().updateAll(context.applicationContext)
    }
}

class ToggleWidgetChooserAction : ActionCallback {
    override suspend fun onAction(
        context: Context,
        glanceId: GlanceId,
        parameters: ActionParameters,
    ) {
        val id = parameters[WidgetIdKey] ?: return
        val store = WidgetOptionsStore(context.applicationContext)
        store.setChooserOpen(id, !store.chooserOpen(id))
        FlowlistWidget().updateAll(context.applicationContext)
    }
}

class SetWidgetModeAction : ActionCallback {
    override suspend fun onAction(
        context: Context,
        glanceId: GlanceId,
        parameters: ActionParameters,
    ) {
        val id = parameters[WidgetIdKey] ?: return
        val mode = parameters[WidgetModeKey] ?: return
        val store = WidgetOptionsStore(context.applicationContext)
        store.save(id, store.load(id).copy(mode = mode))
        store.setChooserOpen(id, false)
        FlowlistWidget().updateAll(context.applicationContext)
    }
}
