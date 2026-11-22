package dev.flowlist.android.ui

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import dev.flowlist.android.data.TaskView

enum class TaskDropKind { Nest, RootAfter }

data class TaskDropTarget(val kind: TaskDropKind, val id: String)

/**
 * Shared drag state for the task lists. Each row records its painted bounds;
 * the dragged row follows the finger and the row under the pointer decides
 * whether a release nests the task or moves it to the project root.
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
    }

    private fun hitTest(point: Offset): TaskDropTarget? {
        bounds.forEach { (key, rect) ->
            if (!key.startsWith("task:") || !rect.contains(point)) return@forEach
            val id = key.removePrefix("task:")
            // The bottom third of a root row is the "move out to the root"
            // zone; the rest of a row is a nest target.
            val root = roots.containsKey(id)
            val rootZone = root && point.y >= rect.bottom - rect.height * 0.32f
            return if (rootZone) {
                TaskDropTarget(TaskDropKind.RootAfter, id)
            } else {
                TaskDropTarget(TaskDropKind.Nest, id)
            }
        }
        return null
    }
}

val LocalTaskDrag = staticCompositionLocalOf<TaskDragController?> { null }
