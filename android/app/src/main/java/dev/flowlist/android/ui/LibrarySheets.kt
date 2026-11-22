package dev.flowlist.android.ui

import android.location.Geocoder
import java.util.Locale
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import androidx.compose.material.icons.rounded.Place
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.CircularProgressIndicator
import android.content.Context
import androidx.compose.foundation.layout.size
import android.Manifest
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.platform.LocalContext
import dev.flowlist.android.data.GeoDto
import dev.flowlist.android.notify.DeviceLocation
import kotlinx.coroutines.launch
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
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.KeyboardType
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
    defaultReviewDays: Int = 7,
    onDismiss: () -> Unit,
    onSave: (
        name: String,
        folder: String?,
        status: String,
        kind: String,
        notes: String,
        reviewDays: Int,
    ) -> Unit,
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
    var kind by remember(project?.id) { mutableStateOf(project?.kind ?: "parallel") }
    var notes by remember(project?.id) { mutableStateOf(project?.notes.orEmpty()) }
    var reviewDays by remember(project?.id) {
        mutableStateOf((project?.reviewIntervalDays ?: defaultReviewDays).toString())
    }
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

            LibraryLabel("Type")
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .horizontalScroll(rememberScrollState()),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                listOf(
                    "parallel" to "Parallel",
                    "sequential" to "Sequential",
                    "single_actions" to "Single action",
                ).forEach { (key, label) ->
                    LibraryPill(label, kind == key) { kind = key }
                }
            }

            LibraryLabel("Notes")
            OutlinedTextField(
                value = notes,
                onValueChange = { notes = it },
                placeholder = { Text("Anything worth keeping", color = colors.faint) },
                minLines = 3,
                colors = libraryFieldColors(),
                modifier = Modifier.fillMaxWidth(),
            )

            LibraryLabel("Review every")
            Row(verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = reviewDays,
                    onValueChange = { value ->
                        reviewDays = value.filter { it.isDigit() }.take(3)
                    },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                    colors = libraryFieldColors(),
                    modifier = Modifier.width(110.dp),
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text("days", style = MaterialTheme.typography.bodyMedium, color = colors.faint)
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
                    onClick = {
                        onSave(
                            name.trim(),
                            folder,
                            status,
                            kind,
                            notes.trim(),
                            reviewDays.toIntOrNull()?.coerceIn(1, 365) ?: defaultReviewDays,
                        )
                    },
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
    onSave: (name: String, parent: String?, location: GeoDto?) -> Unit,
    onDelete: (() -> Unit)?,
) {
    val colors = flow
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val tag = target.tag
    var name by remember(tag?.id) { mutableStateOf(tag?.name.orEmpty()) }
    var parent by remember(tag?.id, target.parent) { mutableStateOf(tag?.parent ?: target.parent) }
    var lat by remember(tag?.id) {
        mutableStateOf(tag?.location?.lat?.let { "%.5f".format(it) }.orEmpty())
    }
    var lon by remember(tag?.id) {
        mutableStateOf(tag?.location?.lon?.let { "%.5f".format(it) }.orEmpty())
    }
    var placeQuery by remember(tag?.id) { mutableStateOf("") }
    var placeLabel by remember(tag?.id) { mutableStateOf<String?>(null) }
    var suggestions by remember(tag?.id) { mutableStateOf<List<PlaceSuggestion>>(emptyList()) }
    var searching by remember(tag?.id) { mutableStateOf(false) }
    var searched by remember(tag?.id) { mutableStateOf(false) }
    val findPlace = {
        val query = placeQuery.trim()
        if (query.isNotEmpty()) {
            searching = true
            searched = true
            suggestions = emptyList()
            scope.launch {
                suggestions = geocodePlaces(context, query)
                searching = false
            }
        }
    }
    val place = remember(lat, lon) {
        val latitude = lat.toDoubleOrNull()
        val longitude = lon.toDoubleOrNull()
        if (latitude == null || longitude == null) {
            null
        } else if (latitude in -90.0..90.0 && longitude in -180.0..180.0) {
            GeoDto(latitude, longitude)
        } else {
            null
        }
    }
    val locationPermission = rememberLauncherForActivityResult(
        ActivityResultContracts.RequestMultiplePermissions(),
    ) { grants ->
        if (grants.values.any { it }) {
            scope.launch {
                DeviceLocation.current(context)?.let { fix ->
                    lat = "%.5f".format(fix.lat)
                    lon = "%.5f".format(fix.lon)
                }
            }
        }
    }
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
                LibraryLabel("Geotag")
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = lat,
                        onValueChange = { lat = it.filter { ch -> ch.isDigit() || ch == '-' || ch == '.' }.take(12) },
                        label = { Text("Latitude") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                        colors = libraryFieldColors(),
                        modifier = Modifier.weight(1f),
                    )
                    OutlinedTextField(
                        value = lon,
                        onValueChange = { lon = it.filter { ch -> ch.isDigit() || ch == '-' || ch == '.' }.take(12) },
                        label = { Text("Longitude") },
                        singleLine = true,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal),
                        colors = libraryFieldColors(),
                        modifier = Modifier.weight(1f),
                    )
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    OutlinedTextField(
                        value = placeQuery,
                        onValueChange = { placeQuery = it },
                        placeholder = { Text("Search a place, e.g. Guntur", color = colors.faint) },
                        singleLine = true,
                        colors = libraryFieldColors(),
                        keyboardOptions = KeyboardOptions(imeAction = androidx.compose.ui.text.input.ImeAction.Search),
                        keyboardActions = androidx.compose.foundation.text.KeyboardActions(
                            onSearch = { findPlace() },
                        ),
                        modifier = Modifier.weight(1f),
                    )
                    IconButton(onClick = { findPlace() }) {
                        Icon(Icons.Rounded.Search, contentDescription = "Search places", tint = colors.accent)
                    }
                }
                if (searching) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(14.dp),
                            strokeWidth = 2.dp,
                            color = colors.accent,
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Text("Searching…", style = MaterialTheme.typography.labelMedium, color = colors.faint)
                    }
                }
                suggestions.forEach { place ->
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable {
                                lat = "%.5f".format(place.lat)
                                lon = "%.5f".format(place.lon)
                                placeLabel = place.name
                                suggestions = emptyList()
                            }
                            .padding(vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(
                            imageVector = Icons.Rounded.Place,
                            contentDescription = null,
                            tint = colors.faint,
                            modifier = Modifier.size(16.dp),
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column(modifier = Modifier.weight(1f)) {
                            Text(
                                text = place.name,
                                style = MaterialTheme.typography.bodyMedium,
                                color = colors.ink,
                                maxLines = 2,
                                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                            )
                            Text(
                                text = "%.5f, %.5f".format(place.lat, place.lon),
                                style = MaterialTheme.typography.labelSmall,
                                color = colors.faint,
                            )
                        }
                    }
                }
                if (searched && !searching && suggestions.isEmpty()) {
                    Text(
                        text = "No places found. Check the spelling or enter coordinates.",
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.faint,
                    )
                }
                if (placeLabel != null) {
                    Text(
                        text = placeLabel!!,
                        style = MaterialTheme.typography.labelMedium,
                        color = colors.accent,
                    )
                }
                Row(verticalAlignment = Alignment.CenterVertically) {
                    TextButton(onClick = {
                        locationPermission.launch(
                            arrayOf(
                                Manifest.permission.ACCESS_FINE_LOCATION,
                                Manifest.permission.ACCESS_COARSE_LOCATION,
                            ),
                        )
                    }) {
                        Text("Use my location", color = colors.accent)
                    }
                    if (place != null) {
                        TextButton(onClick = {
                            val label = Uri.encode(name.ifBlank { "Tag" })
                            runCatching {
                                context.startActivity(
                                    Intent(
                                        Intent.ACTION_VIEW,
                                        Uri.parse(
                                            "geo:${place.lat},${place.lon}?q=${place.lat},${place.lon}($label)",
                                        ),
                                    ),
                                )
                            }
                        }) {
                            Text("Maps", color = colors.accent)
                        }
                        TextButton(onClick = {
                            lat = ""
                            lon = ""
                            placeLabel = null
                        }) {
                            Text("Clear", color = colors.faint)
                        }
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
                onClick = { onSave(name.trim().removePrefix("#"), parent, place) },
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

private data class PlaceSuggestion(val name: String, val lat: Double, val lon: Double)

private suspend fun geocodePlaces(context: Context, query: String): List<PlaceSuggestion> =
    withContext(Dispatchers.IO) {
        if (!Geocoder.isPresent()) return@withContext emptyList()
        runCatching {
            @Suppress("DEPRECATION")
            Geocoder(context, Locale.getDefault())
                .getFromLocationName(query, 6)
                .orEmpty()
                .filter { it.hasLatitude() && it.hasLongitude() }
                .map { address ->
                    val name = address.getAddressLine(0)?.takeIf { it.isNotBlank() }
                        ?: listOfNotNull(
                            address.locality,
                            address.adminArea,
                            address.countryName,
                        ).joinToString(", ")
                    PlaceSuggestion(name.ifBlank { query }, address.latitude, address.longitude)
                }
        }.getOrDefault(emptyList())
    }
