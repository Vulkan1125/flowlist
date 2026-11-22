package dev.flowlist.android.data

import android.net.Uri

/** What another app handed to Flowlist from the share sheet. */
sealed interface SharedPayload {
    /** Plain text: opens the capture sheet prefilled. */
    data class Text(val text: String) : SharedPayload

    /** A shared link: becomes a task with the URL as its attachment. */
    data class Link(val url: String, val title: String?) : SharedPayload

    /** A shared file: becomes a task with the file uploaded as its attachment. */
    data class File(val uri: Uri, val name: String?) : SharedPayload
}
