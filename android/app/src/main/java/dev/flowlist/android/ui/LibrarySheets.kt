package dev.flowlist.android.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.flowlist.android.data.FolderDto
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto

data class ProjectEditorTarget(val project: ProjectDto?, val folder: String? = null)

data class FolderEditorTarget(val folder: FolderDto? = null, val parent: String? = null)

data class TagEditorTarget(val tag: TagDto? = null, val parent: String? = null)

fun folderFamily(folderId: String, folders: List<FolderDto>): Set<String> {
    val ids = mutableSetOf(folderId)
    var changed = true
    while (changed) {
        changed = false
        folders.forEach { folder ->
            if (folder.parent in ids && ids.add(folder.id)) changed = true
        }
    }
    return ids
}

fun tagFamily(tagId: String, tags: List<TagDto>): Set<String> {
    val ids = mutableSetOf(tagId)
    var changed = true
    while (changed) {
        changed = false
        tags.forEach { tag ->
            if (tag.parent in ids && ids.add(tag.id)) changed = true
        }
    }
    return ids
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProjectEditorSheet(
    target: ProjectEditorTarget,
    folders: List<FolderDto>,
    onDismiss: () -> Unit,
    onSave: (name: String, folder: String?, status: String) -> Unit,
    onDelete: (() -> Unit)?,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val project = target.project
    var name by remember(project?.id) { mutableStateOf(project?.name.orEmpty()) }
    var folder by remember(project?.id, target.folder) {
        mutableStateOf(project?.folder ?: target.folder)
    }
    var status by remember(project?.id) { mutableStateOf(project?.status ?: "active") }
    val sortedFolders = remember(folders) { folders.sortedBy { it.name.lowercase() } }
    val canSave = name.isNotBlank()

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 20.dp)
                .padding(bottom = 30.dp),
        ) {
            SheetTitle(
                title = if (project == null) "New project" else "Edit project",
                onDismiss = onDismiss,
            )
            Spacer(modifier = Modifier.height(6.dp))
            OutlinedTextField(
                value = name,
                onValueChange = { name = it },
                placeholder = { Text("Project name", color = colors.faint) },
                singleLine = true,
                colors = libraryFieldColors(),
                modifier = Modifier.fillMaxWidth(),
            )

            LibraryLabel("Folder")
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                LibraryPill("No folder", folder == null) { folder = null }
                sortedFolders.forEach { item ->
                    LibraryPill(item.name, folder == item.id) { folder = item.id }
                }
            }

            if (project != null) {
                LibraryLabel("Status")
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    listOf(
                        "active" to "Active",
                        "on_hold" to "On Hold",
                        "done" to "Done",
                        "dropped" to "Dropped",
                    ).forEach { (key, label) ->
                        LibraryPill(label, status == key) { status = key }
                    }
                }
            }

            Spacer(modifier = Modifier.height(20.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                if (onDelete != null) {
                    TextButton(onClick = onDelete) {
                        Text("Delete", color = colors.danger)
                    }
                }
                Spacer(modifier = Modifier.weight(1f))
                TextButton(onClick = onDismiss) {
                    Text("Cancel", color = colors.faint)
                }
                Spacer(modifier = Modifier.width(6.dp))
                Button(
                    onClick = { onSave(name.trim(), folder, status) },
                    enabled = canSave,
                    colors = ButtonDefaults.buttonColors(
                        containerColor = colors.accent,
                        contentColor = colors.onAccent,
                    ),
                ) {
                    Text(if (project == null) "Create" else "Save")
                }
            }
        }
    }
}

@Composable
fun FolderEditorDialog(
    target: FolderEditorTarget,
    folders: List<FolderDto>,
    onDismiss: () -> Unit,
    onSave: (name: String, parent: String?) -> Unit,
    onDelete: (() -> Unit)?,
) {
    val colors = flow
    val folder = target.folder
    var name by remember(folder?.id) { mutableStateOf(folder?.name.orEmpty()) }
    var parent by remember(folder?.id, target.parent) {
        mutableStateOf(folder?.parent ?: target.parent)
    }
    val blocked = folder?.let { folderFamily(it.id, folders) }.orEmpty()
    val options = remember(folders, blocked) {
        folders.filterNot { it.id in blocked }.sortedBy { it.name.lowercase() }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (folder == null) "New folder" else "Edit folder") },
        text = {
            Column(
                modifier = Modifier
                    .heightIn(max = 460.dp)
                    .verticalScroll(rememberScrollState()),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    placeholder = { Text("Folder name", color = colors.faint) },
                    singleLine = true,
                    colors = libraryFieldColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
                LibraryLabel("Inside")
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    LibraryPill("No parent", parent == null) { parent = null }
                    options.forEach { item ->
                        LibraryPill(item.name, parent == item.id) { parent = item.id }
                    }
                }
                if (onDelete != null) {
                    TextButton(onClick = onDelete, modifier = Modifier.padding(top = 10.dp)) {
                        Text("Delete folder", color = colors.danger)
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSave(name.trim(), parent) },
                enabled = name.isNotBlank(),
            ) {
                Text(if (folder == null) "Create" else "Save", color = colors.accent)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel", color = colors.faint) }
        },
        containerColor = colors.background,
    )
}

@Composable
fun TagEditorDialog(
    target: TagEditorTarget,
    tags: List<TagDto>,
    onDismiss: () -> Unit,
    onSave: (name: String, parent: String?) -> Unit,
    onDelete: (() -> Unit)?,
) {
    val colors = flow
    val tag = target.tag
    var name by remember(tag?.id) { mutableStateOf(tag?.name.orEmpty()) }
    var parent by remember(tag?.id, target.parent) { mutableStateOf(tag?.parent ?: target.parent) }
    val blocked = tag?.let { tagFamily(it.id, tags) }.orEmpty()
    val options = remember(tags, blocked) {
        tags.filterNot { it.id in blocked }.sortedBy { it.name.lowercase() }
    }

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(if (tag == null) "New tag" else "Edit tag") },
        text = {
            Column(
                modifier = Modifier
                    .heightIn(max = 460.dp)
                    .verticalScroll(rememberScrollState()),
            ) {
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it },
                    placeholder = { Text("Tag name", color = colors.faint) },
                    singleLine = true,
                    colors = libraryFieldColors(),
                    modifier = Modifier.fillMaxWidth(),
                )
                LibraryLabel("Inside")
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .horizontalScroll(rememberScrollState()),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    LibraryPill("No parent", parent == null) { parent = null }
                    options.forEach { item ->
                        LibraryPill("#${item.name}", parent == item.id) { parent = item.id }
                    }
                }
                if (onDelete != null) {
                    TextButton(onClick = onDelete, modifier = Modifier.padding(top = 10.dp)) {
                        Text("Delete tag", color = colors.danger)
                    }
                }
            }
        },
        confirmButton = {
            TextButton(
                onClick = { onSave(name.trim().removePrefix("#"), parent) },
                enabled = name.isNotBlank(),
            ) {
                Text(if (tag == null) "Create" else "Save", color = colors.accent)
            }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel", color = colors.faint) }
        },
        containerColor = colors.background,
    )
}

@Composable
fun ConfirmDialog(
    title: String,
    body: String,
    confirmLabel: String,
    onConfirm: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = { Text(body) },
        confirmButton = {
            TextButton(onClick = onConfirm) { Text(confirmLabel, color = colors.danger) }
        },
        dismissButton = {
            TextButton(onClick = onDismiss) { Text("Cancel", color = colors.faint) }
        },
        containerColor = colors.background,
    )
}

@Composable
private fun SheetTitle(title: String, onDismiss: () -> Unit) {
    val colors = flow
    Row(verticalAlignment = Alignment.CenterVertically) {
        Text(
            text = title,
            style = MaterialTheme.typography.titleLarge,
            color = colors.ink,
            modifier = Modifier.weight(1f),
        )
        IconButton(onClick = onDismiss) {
            Icon(Icons.Rounded.Close, contentDescription = "Close", tint = colors.faint)
        }
    }
}

@Composable
private fun LibraryLabel(text: String) {
    Text(
        text = text,
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.SemiBold,
        color = flow.faint,
        modifier = Modifier.padding(top = 16.dp, bottom = 8.dp),
    )
}

@Composable
private fun LibraryPill(label: String, selected: Boolean, onClick: () -> Unit) {
    val colors = flow
    Box(
        modifier = Modifier
            .heightIn(min = 34.dp)
            .background(
                color = if (selected) colors.field else Color.Transparent,
                shape = RoundedCornerShape(9.dp),
            )
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 7.dp),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelLarge,
            color = if (selected) colors.ink else colors.faint,
            maxLines = 1,
            softWrap = false,
        )
    }
}

@Composable
private fun libraryFieldColors() = androidx.compose.material3.OutlinedTextFieldDefaults.colors(
    focusedBorderColor = flow.hairline,
    unfocusedBorderColor = flow.hairline,
    focusedContainerColor = flow.background,
    unfocusedContainerColor = flow.background,
    cursorColor = flow.accent,
    focusedTextColor = flow.ink,
    unfocusedTextColor = flow.ink,
)
