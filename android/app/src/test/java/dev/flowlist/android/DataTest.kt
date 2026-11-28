package dev.flowlist.android

import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskDto
import dev.flowlist.android.data.Ulid
import dev.flowlist.android.data.tagBlocking
import dev.flowlist.android.data.taggedTaskCount
import dev.flowlist.android.data.geotaggedTaskCount
import dev.flowlist.android.data.GeoDto
import dev.flowlist.android.data.tagIsBlocked
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.ui.Family
import dev.flowlist.android.ui.Perspective
import dev.flowlist.android.ui.drawerCount
import dev.flowlist.android.ui.ThemeSettings
import dev.flowlist.android.ui.themeColors
import dev.flowlist.android.ui.projectIconChoices
import dev.flowlist.android.ui.PerspectiveDraft
import dev.flowlist.android.ui.draftFrom
import dev.flowlist.android.ui.toFilter
import dev.flowlist.android.data.displayTitle
import dev.flowlist.android.data.normalizeBase
import dev.flowlist.android.data.savePerspectiveCommand
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DataTest {
    @Test
    fun each_view_keeps_its_own_sort() {
        val initial = AppPreferences(sortBy = "due")
        assertEquals("due", initial.sortFor("inbox"))
        val changed = initial.withSort("inbox", "name")
        assertEquals("name", changed.sortFor("inbox"))
        assertEquals("due", changed.sortFor("today"))
        assertEquals("manual", changed.withSort("today", "manual").sortFor("today"))
    }

    @Test
    fun trash_swipes_default_to_restore_right() {
        assertEquals("restore_right", AppPreferences().trashSwipe)
        assertEquals("restore_left", AppPreferences().copy(trashSwipe = "restore_left").trashSwipe)
        assertEquals("off", AppPreferences().copy(trashSwipe = "off").trashSwipe)
    }

    @Test
    fun project_icon_picker_matches_the_mac_set() {
        assertEquals(63, projectIconChoices.size)
        assertEquals(63, projectIconChoices.map { it.first }.toSet().size)
        assertTrue(projectIconChoices.any { it.first == "basketball" })
        assertTrue(projectIconChoices.any { it.first == "stethoscope" })
        assertTrue(projectIconChoices.any { it.first == "telescope" })
    }

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

    @Test
    fun perspective_estimate_limit_becomes_a_filter() {
        val filter = PerspectiveDraft(
            name = "Quick wins",
            estimateMax = 15,
        ).toFilter().toString()
        assertTrue(filter.contains("\"type\":\"estimate\""))
        assertTrue(filter.contains("\"max_minutes\":15"))
        assertTrue(filter.contains("\"min_minutes\":null"))
    }

    @Test
    fun perspective_multi_select_round_trips() {
        val draft = PerspectiveDraft(
            name = "Multi",
            projects = setOf("p1", "p2"),
            tags = setOf("t1", "t2"),
            priorities = setOf("high", "low"),
            dues = setOf("today", "tomorrow"),
            defers = setOf("tomorrow", "no_date"),
            inboxOnly = true,
            flaggedOnly = true,
            status = "available",
            tagChildren = false,
        )
        val back = draftFrom(PerspectiveDto(id = "p", name = "Multi", filter = draft.toFilter()))
        assertEquals(setOf("p1", "p2"), back.projects)
        assertEquals(setOf("t1", "t2"), back.tags)
        assertEquals(setOf("high", "low"), back.priorities)
        assertEquals(setOf("today", "tomorrow"), back.dues)
        assertEquals(setOf("tomorrow", "no_date"), back.defers)
        assertTrue(back.inboxOnly)
        assertTrue(back.flaggedOnly)
        assertEquals("available", back.status)
        assertEquals(false, back.tagChildren)
    }

    @Test
    fun perspective_presentation_fields_round_trip() {
        val dto = PerspectiveDto(
            id = "p",
            name = "Pinned",
            pinned = true,
            icon = "🏃",
            description = "Today's focus",
        )
        val draft = draftFrom(dto)
        assertTrue(draft.pinned)
        assertEquals("🏃", draft.icon)
        assertEquals("Today's focus", draft.description)
    }

    @Test
    fun the_save_command_carries_the_perspective_description() {
        val command = savePerspectiveCommand(
            id = "p",
            name = "Pinned",
            filter = PerspectiveDraft(name = "Pinned").toFilter(),
            groupBy = "none",
            sort = "manual",
            projectGroupBy = "none",
            projectSort = "manual",
            showCompleted = false,
            pinned = true,
            position = 3,
            icon = "star",
            description = "Today's focus",
        ).toString()
        assertTrue(command.contains("\"description\":\"Today's focus\""))
        assertTrue(command.contains("\"pinned\":true"))
        assertTrue(command.contains("\"icon\":\"star\""))
    }

    @Test
    fun legacy_todoist_theme_loads_as_tomato_and_dark_defaults_follow_light() {
        assertEquals(Family.Tomato, Family.fromSlug("todoist"))
        assertEquals(Family.Tomato, Family.fromSlug("tomato"))
        assertEquals(Family.Paper, Family.fromSlug("paper"))
        assertEquals(Family.Tomato, Family.fromSlug(null))
        assertEquals(Family.Tomato, Family.fromSlug("unknown-family"))
    }

    @Test
    fun drawer_counts_match_the_destination_rows() {
        val db = Db(
            tasks = mapOf(
                "a" to TaskDto(id = "a", title = "Inbox note"),
                "b" to TaskDto(id = "b", title = "Flagged note", flagged = true),
            ),
        )
        assertEquals(2, drawerCount(db, Perspective.Inbox))
        assertEquals(1, drawerCount(db, Perspective.Flagged))
        assertEquals(0, drawerCount(db, Perspective.Today))
        assertEquals(0, drawerCount(db, Perspective.Trash))
    }

    @Test
    fun tag_and_nearby_counts_measure_tasks_not_tags() {
        val db = Db(
            tasks = mapOf(
                "a" to TaskDto(id = "a", title = "Tagged quietly", tags = listOf("t2")),
                "b" to TaskDto(id = "b", title = "Tagged near", tags = listOf("t1", "t2")),
                "c" to TaskDto(id = "c", title = "Plain"),
            ),
            tags = mapOf(
                "t1" to TagDto(id = "t1", name = "Focus", location = GeoDto(1.0, 2.0)),
                "t2" to TagDto(id = "t2", name = "Quiet"),
            ),
        )
        assertEquals(2, db.taggedTaskCount())
        assertEquals(1, db.geotaggedTaskCount())
    }

    @Test
    fun non_tomato_danger_stays_vivid_red() {
        val light = themeColors(ThemeSettings(family = Family.Gruvbox), dark = false)
        assertEquals(androidx.compose.ui.graphics.Color(0xFFDC4C3E), light.danger)
        val dark = themeColors(ThemeSettings(darkFamily = Family.Gruvbox), dark = true)
        assertEquals(androidx.compose.ui.graphics.Color(0xFFFF7066), dark.danger)
    }

    @Test
    fun non_tomato_warnings_stay_clearly_yellow() {
        val light = themeColors(ThemeSettings(family = Family.Plain), dark = false)
        assertEquals(androidx.compose.ui.graphics.Color(0xFFC69214), light.warning)
        val dark = themeColors(ThemeSettings(darkFamily = Family.Plain), dark = true)
        assertEquals(androidx.compose.ui.graphics.Color(0xFFFACC15), dark.warning)
        val tomato = themeColors(ThemeSettings(family = Family.Tomato), dark = false)
        assertEquals(androidx.compose.ui.graphics.Color(0xFFEB8909), tomato.warning)
    }
}
