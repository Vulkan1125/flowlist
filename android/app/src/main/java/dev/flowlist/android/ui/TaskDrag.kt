package dev.flowlist.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import dev.flowlist.android.data.TaskView

enum class TaskDropKind {
    /// Put the task under the target row.
    Nest,

    /// Put the task beside the target row, before or after it.
    Before,
    After,

    /// File the task at the end of a project, a tag or the Inbox.
    Project,
    Tag,
    Inbox,

    /// Dropped on the shelf's "Choose" chip; the app opens a picker.
    ChooseProject,
    ChooseTag,

    /// Dropped on a Forecast day; the task's due date moves to that day.
    Day,
}

data class TaskDropTarget(val kind: TaskDropKind, val id: String)

/**
 * Shared drag state for the task lists. Each row records its painted bounds;
 * the dragged row follows the finger and the row under the pointer decides
 * whether a release nests the task, places it beside a sibling, files it into
 * a project or the Inbox, or adds a tag.
 */
class TaskDragController {
    var dragging: TaskView? by mutableStateOf(null)
    var offset: Offset by mutableStateOf(Offset.Zero)
    var target: TaskDropTarget? by mutableStateOf(null)
    var onDrop: ((TaskView, TaskDropTarget) -> Unit)? = null

    val bounds = mutableStateMapOf<String, Rect>()
    val roots = mutableStateMapOf<String, Boolean>()
    /// Parents opened by a drop so the newly nested task is visible.
    val revealed = mutableStateMapOf<String, Boolean>()

    private var pointer = Offset.Zero

    fun start(task: TaskView, rowBounds: Rect, local: Offset) {
        dragging = task
        offset = Offset.Zero
        pointer = rowBounds.topLeft + local
        target = null
    }

    fun move(delta: Offset) {
        offset += delta
        pointer += delta
        target = hitTest(pointer)
    }

    fun end() {
        val task = dragging
        val drop = target
        reset()
        if (task != null && drop != null) onDrop?.invoke(task, drop)
    }

    fun cancel() = reset()

    fun reset() {
        dragging = null
        offset = Offset.Zero
        target = null
        // Drop-shelf chips only exist while the shelf is on screen. Calendar
        // days persist because they are painted with the calendar.
        bounds.keys
            .filter {
                it.startsWith("project:") ||
                    it.startsWith("tag:") ||
                    it == "inbox" ||
                    it == "choose-project" ||
                    it == "choose-tag"
            }
            .forEach { bounds.remove(it) }
    }

    private fun hitTest(point: Offset): TaskDropTarget? {
        // The shelf and destination chips paint over the rows, so they win.
        bounds.forEach { (key, rect) ->
            if (!rect.contains(point)) return@forEach
            when {
                key.startsWith("project:") -> return TaskDropTarget(
                    TaskDropKind.Project,
                    key.removePrefix("project:"),
                )
                key.startsWith("tag:") -> return TaskDropTarget(
                    TaskDropKind.Tag,
                    key.removePrefix("tag:"),
                )
                key == "inbox" -> return TaskDropTarget(TaskDropKind.Inbox, "inbox")
                key == "choose-project" -> return TaskDropTarget(TaskDropKind.ChooseProject, "")
                key == "choose-tag" -> return TaskDropTarget(TaskDropKind.ChooseTag, "")
                key.startsWith("day:") -> return TaskDropTarget(
                    TaskDropKind.Day,
                    key.removePrefix("day:"),
                )
            }
        }
        bounds.forEach { (key, rect) ->
            if (!key.startsWith("task:") || !rect.contains(point)) return@forEach
            val id = key.removePrefix("task:")
            val zone = (point.y - rect.top) / rect.height
            return when {
                zone <= 0.22f -> TaskDropTarget(TaskDropKind.Before, id)
                zone >= 0.75f -> TaskDropTarget(TaskDropKind.After, id)
                else -> TaskDropTarget(TaskDropKind.Nest, id)
            }
        }
        return null
    }
}

val LocalTaskDrag = staticCompositionLocalOf<TaskDragController?> { null }
