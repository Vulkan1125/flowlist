package dev.flowlist.android.ui

import android.graphics.Bitmap
import android.graphics.pdf.PdfRenderer
import android.os.ParcelFileDescriptor
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ChevronLeft
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import java.io.File

data class PreviewTarget(val path: String, val file: File)

/** A card for an image or PDF attachment, with Open externally still there. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AttachmentPreviewSheet(
    target: PreviewTarget,
    onOpen: () -> Unit,
    onDismiss: () -> Unit,
) {
    val colors = flow
    val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    val name = target.path.substringAfterLast('/').ifBlank { "Attachment" }
    val isPdf = name.substringAfterLast('.', "").equals("pdf", ignoreCase = true)
    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheet,
        containerColor = colors.background,
        dragHandle = { SheetHandle() },
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .navigationBarsPadding()
                .padding(horizontal = 16.dp)
                .padding(bottom = 18.dp),
        ) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Text(
                    text = name,
                    style = MaterialTheme.typography.titleMedium,
                    color = colors.ink,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Rounded.Close, contentDescription = "Close", tint = colors.faint)
                }
            }
            Spacer(modifier = Modifier.height(8.dp))
            if (isPdf) {
                PdfPreview(target.file)
            } else {
                AsyncImage(
                    model = target.file,
                    contentDescription = name,
                    contentScale = ContentScale.Fit,
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 520.dp)
                        .clip(RoundedCornerShape(10.dp)),
                )
            }
            Spacer(modifier = Modifier.height(10.dp))
            Row(verticalAlignment = Alignment.CenterVertically) {
                TextButton(onClick = onOpen) {
                    Text("Open externally", color = colors.accent)
                }
                Spacer(modifier = Modifier.weight(1f))
                TextButton(onClick = onDismiss) {
                    Text("Close", color = colors.faint)
                }
            }
        }
    }
}

@Composable
private fun PdfPreview(file: File) {
    val colors = flow
    val renderer = remember(file) {
        runCatching {
            PdfRenderer(ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY))
        }.getOrNull()
    }
    DisposableEffect(renderer) {
        onDispose { renderer?.close() }
    }
    if (renderer == null) {
        Text(
            text = "Couldn't open this PDF.",
            style = MaterialTheme.typography.bodyMedium,
            color = colors.faint,
            modifier = Modifier.padding(16.dp),
        )
        return
    }
    var page by remember { mutableIntStateOf(0) }
    val count = renderer.pageCount
    val bitmap = remember(renderer, page) {
        runCatching {
            renderer.openPage(page).use { pdfPage ->
                val width = 1080
                val height = (width * pdfPage.height / pdfPage.width.toFloat())
                    .toInt()
                    .coerceAtLeast(1)
                Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888).also { image ->
                    image.eraseColor(android.graphics.Color.WHITE)
                    pdfPage.render(image, null, null, PdfRenderer.Page.RENDER_MODE_FOR_DISPLAY)
                }
            }
        }.getOrNull()
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        if (bitmap != null) {
            Image(
                bitmap = bitmap.asImageBitmap(),
                contentDescription = "Page ${page + 1}",
                contentScale = ContentScale.Fit,
                modifier = Modifier
                    .fillMaxWidth()
                    .heightIn(max = 480.dp)
                    .clip(RoundedCornerShape(10.dp)),
            )
        } else {
            Text(
                text = "Couldn't render this page.",
                style = MaterialTheme.typography.bodyMedium,
                color = colors.faint,
                modifier = Modifier.padding(16.dp),
            )
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = { if (page > 0) page-- }, enabled = page > 0) {
                Icon(
                    Icons.Rounded.ChevronLeft,
                    contentDescription = "Previous page",
                    tint = if (page > 0) colors.ink else colors.faint,
                )
            }
            Text(
                text = "Page ${page + 1} of $count",
                style = MaterialTheme.typography.labelMedium,
                color = colors.faint,
            )
            IconButton(onClick = { if (page < count - 1) page++ }, enabled = page < count - 1) {
                Icon(
                    Icons.Rounded.ChevronRight,
                    contentDescription = "Next page",
                    tint = if (page < count - 1) colors.ink else colors.faint,
                )
            }
        }
    }
}
