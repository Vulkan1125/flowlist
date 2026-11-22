package dev.flowlist.android.widget

import android.content.Context
import androidx.glance.GlanceId
import androidx.glance.action.ActionParameters
import androidx.glance.appwidget.action.ActionCallback
import androidx.glance.appwidget.updateAll
import dev.flowlist.android.data.Repository

/// The task id carried by a widget row's check or reschedule button.
val WidgetTaskKey = ActionParameters.Key<String>("widget_task_id")

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

class RescheduleWidgetTaskAction : ActionCallback {
    override suspend fun onAction(
        context: Context,
        glanceId: GlanceId,
        parameters: ActionParameters,
    ) {
        val id = parameters[WidgetTaskKey] ?: return
        runCatching { Repository(context.applicationContext).rescheduleTomorrow(id) }
        FlowlistWidget().updateAll(context.applicationContext)
    }
}
