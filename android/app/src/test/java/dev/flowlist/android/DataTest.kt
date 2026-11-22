package dev.flowlist.android

import dev.flowlist.android.data.Db
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskDto
import dev.flowlist.android.data.Ulid
import dev.flowlist.android.data.tagBlocking
import dev.flowlist.android.data.tagIsBlocked
import dev.flowlist.android.data.displayTitle
import dev.flowlist.android.data.normalizeBase
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DataTest {
    @Test
    fun addresses_get_a_scheme_and_must_have_a_port() {
        assertEquals("http://192.168.1.33:7777", normalizeBase("192.168.1.33:7777"))
        assertEquals("http://mac:7777", normalizeBase("http://mac:7777/"))
        assertNull(normalizeBase("mac"))
        assertNull(normalizeBase(""))
    }

    @Test
    fun chips_are_hidden_until_the_mac_parses_them() {
        assertEquals(
            "Call the plumber",
            displayTitle("Call the plumber due:tomorrow #home est:30m"),
        )
        assertEquals("Buy stamps", displayTitle("Buy stamps!"))
    }

    @Test
    fun ulids_are_26_crockford_characters_and_sort_by_time() {
        val first = Ulid.next()
        Thread.sleep(2)
        val second = Ulid.next()
        assertEquals(26, first.length)
        assertTrue(first < second)
        assertTrue(first.all { it in "0123456789ABCDEFGHJKMNPQRSTVWXYZ" })
    }

    @Test
    fun blocked_tag_status_inherits_to_child_tags_and_tasks() {
        val db = Db(
            tags = mapOf(
                "parent" to TagDto("parent", "health", status = "on_hold"),
                "child" to TagDto("child", "gym", parent = "parent"),
                "free" to TagDto("free", "errand"),
            ),
            projects = mapOf(
                "project" to ProjectDto("project", "Move", tags = listOf("child")),
            ),
            tasks = mapOf(
                "task" to TaskDto("task", "Pack", tags = listOf("free")),
                "projectTask" to TaskDto("projectTask", "Book van", project = "project"),
            ),
        )
        assertTrue(db.tagIsBlocked("child"))
        assertTrue(!db.tagIsBlocked("free"))
        assertEquals("health", db.tagBlocking(db.tasks.getValue("projectTask"))?.name)
        assertNull(db.tagBlocking(db.tasks.getValue("task")))
    }
}
