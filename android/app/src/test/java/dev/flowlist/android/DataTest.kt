package dev.flowlist.android

import dev.flowlist.android.data.AppPreferences
import dev.flowlist.android.data.Db
import dev.flowlist.android.data.ProjectDto
import dev.flowlist.android.data.TagDto
import dev.flowlist.android.data.TaskDto
import dev.flowlist.android.data.StampDto
import dev.flowlist.android.data.Ulid
import dev.flowlist.android.data.tagBlocking
import dev.flowlist.android.data.tagPath
import dev.flowlist.android.data.taggedTaskCount
import dev.flowlist.android.data.geotaggedTaskCount
import dev.flowlist.android.data.GeoDto
import dev.flowlist.android.data.tagIsBlocked
import dev.flowlist.android.data.PerspectiveDto
import dev.flowlist.android.ui.Family
import dev.flowlist.android.ui.Perspective
import dev.flowlist.android.ui.drawerCount
import dev.flowlist.android.ui.filterViewTasks
import dev.flowlist.android.ui.groupSections
import dev.flowlist.android.ui.ThemeSettings
import dev.flowlist.android.ui.themeColors
import dev.flowlist.android.ui.projectIconChoices
import dev.flowlist.android.ui.PerspectiveDraft
import dev.flowlist.android.ui.draftFrom
import dev.flowlist.android.ui.toFilter
import dev.flowlist.android.data.displayTitle
import dev.flowlist.android.data.normalizeBase
import dev.flowlist.android.data.projectTaskIds
import dev.flowlist.android.data.taskViews
import dev.flowlist.android.data.SettingsDto
import dev.flowlist.android.data.SidebarDto
import dev.flowlist.android.data.forecastProjectDays
import dev.flowlist.android.data.forecastTaskDays
import dev.flowlist.android.data.forecastUndated
import dev.flowlist.android.data.batchUpdateTasksCommand
import dev.flowlist.android.data.repeatNeedsClock
import dev.flowlist.android.data.repeatRowLabel
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.put
import dev.flowlist.android.data.convertTaskToProjectCommand
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
    fun the_batch_command_carries_every_selected_field() {
        val command = batchUpdateTasksCommand(
            ids = listOf("a", "b"),
            patch = kotlinx.serialization.json.buildJsonObject {
                put("priority", "high")
                put("estimate_min", 30)
            },
            addTags = listOf("Focus"),
            removeTags = listOf("Old"),
        ).toString()
        assertTrue(command.contains("\"batch_update_tasks\""))
        assertTrue(command.contains("\"ids\":[\"a\",\"b\"]"))
        assertTrue(command.contains("\"priority\":\"high\""))
        assertTrue(command.contains("\"estimate_min\":30"))
        assertTrue(command.contains("\"add_tags\":[\"Focus\"]"))
        assertTrue(command.contains("\"remove_tags\":[\"Old\"]"))
    }

    @Test
    fun the_convert_command_keeps_the_folder_optional() {
        val command = convertTaskToProjectCommand("task-1").toString()
        assertTrue(command.contains("\"convert_task_to_project\""))
        assertTrue(command.contains("\"id\":\"task-1\""))
        assertTrue(!command.contains("\"folder\""))
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
    fun header_filters_and_groups_apply_to_builtin_views() {
        val db = Db(
            projects = mapOf("p" to ProjectDto(id = "p", name = "Personal")),
            tags = mapOf("t" to TagDto(id = "t", name = "Focus")),
            tasks = mapOf(
                "a" to TaskDto(
                    id = "a",
                    title = "Call",
                    project = "p",
                    priority = "high",
                    tags = listOf("t"),
                ),
                "b" to TaskDto(id = "b", title = "Read", priority = "low"),
            ),
        )
        val views = db.taskViews()
        assertEquals(
            listOf("Call"),
            filterViewTasks(db, views, "high", availableOnly = false).map { it.title },
        )
        assertEquals(
            listOf("Personal", "No project"),
            groupSections(db, views, "project", "manual").map { it.label },
        )
        assertEquals(
            listOf("#Focus", "No tag"),
            groupSections(db, views, "tag", "manual").map { it.label },
        )
    }

    @Test
    fun nested_tag_paths_keep_their_parent() {
        val db = Db(
            tags = mapOf(
                "parent" to TagDto(id = "parent", name = "Work"),
                "child" to TagDto(id = "child", name = "Deep", parent = "parent"),
            ),
        )
        assertEquals("Work", db.tagPath("parent"))
        assertEquals("Work/Deep", db.tagPath("child"))
    }

    @Test
    fun sidebar_counts_keep_android_defaults_until_configured() {
        val untouched = SidebarDto()
        assertTrue(!untouched.isCountHidden("builtin:archive"))
        val configured = SidebarDto(
            countsConfigured = true,
            countsHidden = setOf("builtin:archive", "custom:p"),
        )
        assertTrue(configured.isCountHidden("builtin:archive"))
        assertTrue(configured.isCountHidden("custom:p"))
        assertTrue(!configured.isCountHidden("builtin:completed"))
    }

    @Test
    fun forecast_groups_due_defer_and_planned_dates() {
        val day = java.time.LocalDate.of(2026, 12, 1)
        val next = day.plusDays(1)
        fun stamp(date: java.time.LocalDate) = StampDto(
            java.time.ZonedDateTime.of(
                date,
                java.time.LocalTime.MIDNIGHT,
                java.time.ZoneId.systemDefault(),
            ).toInstant().toString(),
            false,
        )
        val db = Db(
            projects = mapOf(
                "p" to ProjectDto(id = "p", name = "Deadline", due = stamp(day)),
            ),
            tags = mapOf("t" to TagDto(id = "t", name = "Focus")),
            tasks = mapOf(
                "both" to TaskDto(
                    id = "both",
                    title = "Both",
                    due = stamp(day),
                    defer = stamp(day),
                    planned = stamp(day),
                ),
                "planned" to TaskDto(id = "planned", title = "Planned", planned = stamp(next)),
                "tagged" to TaskDto(id = "tagged", title = "Tagged", tags = listOf("t")),
            ),
            settings = SettingsDto(forecastTag = "t", forecastFlagged = true),
        )
        val all = db.forecastTaskDays(dueOnly = false, includePlanned = true)
        assertEquals(listOf("Both"), all[day]?.map { it.title })
        assertEquals(listOf("Planned"), all[next]?.map { it.title })
        val dueOnly = db.forecastTaskDays(dueOnly = true, includePlanned = true)
        assertEquals(listOf("Both"), dueOnly[day]?.map { it.title })
        assertNull(dueOnly[next])
        assertEquals(listOf("Deadline"), db.forecastProjectDays(true, true)[day]?.map { it.name })
        val (tagged, flagged) = db.forecastUndated()
        assertEquals(listOf("Tagged"), tagged.map { it.title })
        assertTrue(flagged.isEmpty())
    }

    @Test
    fun availability_filter_matches_defer_and_project_state() {
        val later = StampDto(java.time.Instant.now().plusSeconds(3600).toString())
        val db = Db(
            projects = mapOf(
                "paused" to ProjectDto(id = "paused", name = "Paused", status = "on_hold"),
            ),
            tasks = mapOf(
                "later" to TaskDto(id = "later", title = "Later", defer = later),
                "paused" to TaskDto(id = "paused", title = "Paused work", project = "paused"),
                "now" to TaskDto(id = "now", title = "Now"),
            ),
        )
        assertEquals(
            listOf("Now"),
            filterViewTasks(db, db.taskViews(), "all", availableOnly = true).map { it.title },
        )
    }

    @Test
    fun project_tasks_include_nested_subtasks() {
        val db = Db(
            projects = mapOf("p" to ProjectDto(id = "p", name = "Project")),
            tasks = mapOf(
                "root" to TaskDto(id = "root", title = "Root", project = "p"),
                "child" to TaskDto(id = "child", title = "Child", parent = "root"),
                "done" to TaskDto(id = "done", title = "Done", project = "p", completed = true),
            ),
        )
        assertEquals(setOf("root", "child"), db.projectTaskIds("p"))
        assertEquals(setOf("root", "child", "done"), db.projectTaskIds("p", includeCompleted = true))
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
    fun repeat_row_label_matches_the_mac_description() {
        fun rule(json: String) = Json.parseToJsonElement(json)
        assertEquals(
            "every 2 weeks",
            repeatRowLabel(
                rule("""{"mode":"due_again","schedule":"fixed","every":2,"unit":"weeks","end":{"kind":"never"}}"""),
            ),
        )
        assertEquals(
            "every Monday and Friday after completion (defer another) for 6 times",
            repeatRowLabel(
                rule(
                    """{"mode":"defer_another","schedule":"after_completion","every":1,"unit":"days","end":{"kind":"after_count","date":6},"weekdays":[1,5]}""",
                ),
            ),
        )
        assertEquals(
            "every month until 2026-12-01",
            repeatRowLabel(
                rule("""{"mode":"due_again","schedule":"fixed","every":1,"unit":"months","end":{"kind":"on_date","date":"2026-12-01"}}"""),
            ),
        )
        assertEquals(
            "every 2 hours",
            repeatRowLabel(
                rule("""{"mode":"due_again","schedule":"fixed","every":2,"unit":"hours","end":{"kind":"never"}}"""),
            ),
        )
        assertEquals(
            "every 30 minutes",
            repeatRowLabel(
                rule("""{"mode":"due_again","schedule":"fixed","every":30,"unit":"minutes","end":{"kind":"never"}}"""),
            ),
        )
        assertEquals(
            "the third Thursday of every month",
            repeatRowLabel(
                rule(
                    """{"mode":"due_again","schedule":"fixed","every":1,"unit":"months","end":{"kind":"never"},"pattern":{"kind":"nth_weekday","nth":3,"weekday":4}}""",
                ),
            ),
        )
        assertEquals(
            "the last weekend day of every 2 months",
            repeatRowLabel(
                rule(
                    """{"mode":"due_again","schedule":"fixed","every":2,"unit":"months","end":{"kind":"never"},"pattern":{"kind":"last_weekend_day"}}""",
                ),
            ),
        )
        assertNull(repeatRowLabel(null))
    }

    @Test
    fun sub_day_repeats_need_a_timed_anchor() {
        val dateOnly = StampDto(at = "2026-11-28T00:00:00Z", hasTime = false)
        val timed = StampDto(at = "2026-11-28T09:00:00Z", hasTime = true)
        assertTrue(repeatNeedsClock("hours", "due_again", dateOnly, null))
        assertTrue(repeatNeedsClock("minutes", "due_again", null, dateOnly))
        assertTrue(!repeatNeedsClock("hours", "due_again", timed, null))
        assertTrue(!repeatNeedsClock("days", "due_again", dateOnly, null))
        assertTrue(repeatNeedsClock("minutes", "defer_another", timed, dateOnly))
        assertTrue(!repeatNeedsClock("minutes", "defer_another", dateOnly, timed))
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
