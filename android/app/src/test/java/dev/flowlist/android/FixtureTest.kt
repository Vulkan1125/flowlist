package dev.flowlist.android

import dev.flowlist.android.data.Snapshot
import kotlinx.serialization.json.Json
import org.junit.Assert.assertTrue
import org.junit.Test

class FixtureTest {
    @Test
    fun a_real_mac_library_decodes() {
        val text = checkNotNull(javaClass.classLoader?.getResource("library.json")).readText()
        val snapshot = Json { ignoreUnknownKeys = true }
            .decodeFromString<Snapshot>(text)
        assertTrue(snapshot.db.tasks.isNotEmpty())
        assertTrue(snapshot.db.projects.isNotEmpty())
        assertTrue(snapshot.db.tags.isNotEmpty())
        assertTrue(snapshot.db.inboxOrder.isNotEmpty())
        val project = snapshot.db.projects.values.first()
        assertTrue(project.name.isNotBlank())
    }
}
