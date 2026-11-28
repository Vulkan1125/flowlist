# Changelog

## 0.10.3 — 2026-11-28

### What's new

- **Hourly and minute repeats.** Actions can repeat every 30 minutes, every 2
  hours, or any other minute/hour interval. Repeats keep the wall-clock time,
  cross midnight correctly, and after-completion repeats continue from the
  moment you finish.
- **Monthly patterns.** A repeating action can now land on a chosen weekday
  (1st Monday through 5th Monday) or on the last weekend day of the month. The
  Mac and Android repeat editors use the same choices, and the summary spells
  out the exact date.
- **A simpler repeat editor.** The Mac Repeat section now uses dropdowns for
  **Schedule the next**, **Repeat from**, **Repeat on** and **Ends**. Choosing
  **After N times** opens a typed count box instead of −/+ buttons.
- **Clearer time errors.** Hourly and minute repeats need a clock time.
  Flowlist now names the date that needs one — **Add a time to the due date
  first** or **Add a time to the defer date first** — instead of guessing.

### Bug fixes

- **Repeat dropdowns line up.** Each menu now opens directly under its own
  control and matches the control's width.
- **Long and unusual patterns stay predictable.** Monthly repeats with long
  intervals land in the correct future month, and unusual pattern values from
  older data are handled safely.

## 0.10.2 — 2026-11-28

### Android parity

- **Planned dates.** The Android task editor now edits the Mac's third date
  beside due and defer, and the task row shows **Plan …**.
- **Project completion mode.** The Android project editor toggles **Complete
  with last action**, and the shared command layer applies it on both devices.
- **Archive controls.** The Android project editor toggles **Archived**, the
  project menu archives without changing status, and Settings → Library has
  **Archive closed projects**.
- **Project dates and tags.** The Android project editor edits due, defer and
  planned dates plus project tags; the project sheet shows status, type,
  review, progress, dates and notes.
- **Project attachments.** The Android project editor lists project files and
  links, adds files through the shared upload path, adds named links, and
  removes either before saving.
- **Editor and Browse polish.** Folders fold from their chevron in Browse, the
  project icon grid stays collapsed until the current icon is tapped, tags wrap
  instead of running under the switches, and every editor switch is right
  aligned with the same accent colours.
- **Saved-view options.** The Android perspective editor now edits the pin, the
  glyph (a named icon or one emoji) and the one-line description. Pinned views
  sit above **Saved**, use a pin glyph by default and also appear in the tablet
  rail; the description shows under the view title. Settings → Library →
  **Hide empty views** hides unpinned views that match nothing.
- **Project pages.** Projects now open as full pages instead of a bottom card:
  a back arrow, the status/review/progress banner, in-project search, sort and
  grouping, **Completed** and **Available** filters, and capture that adds the
  new task to the open project.
- **Folder cells.** Tapping anywhere on a folder row folds or opens it, not
  only its chevron.
- **Today doodle.** The empty Today view now shows a steaming mug instead of
  the old sun face.
- **Header filters and grouping.** Built-in lists, saved views and project
  pages share a filter menu with **Available only** and All/High/Medium/Low
  priority, and Inbox, Today and the task-based More lists gain per-view
  grouping by project, folder, tag, due, defer, priority, flag, estimate,
  added or completed. Choices are remembered per view.
- **Convert task to project.** The Android task editor converts a task and
  its subtasks into a project in one undoable step, keeping the folder, notes,
  tags, dates, attachments and checklist as project tasks. The new project
  opens immediately, and the snackbar undoes the whole conversion.
- **Batch edit.** Select mode's pencil edits the whole selection at once:
  project (including Inbox), add or remove tags with parent paths, due, defer
  and planned dates, priority, flag and estimate. Only filled fields apply,
  the list updates immediately, and the snackbar undoes the whole batch.
- **Drag targets.** Dragging a task now places it before or after a sibling,
  including subtasks in manual order, while the middle of a row still nests.
  A drop shelf appears during the drag with Inbox, project and tag targets and
  choosers, so a task can be filed without the Mac sidebar. Moves and tag adds
  undo from the snackbar.
- **Drawer visibility.** Settings → **Drawer** controls the built-in rows and
  saved views: hide a row, hide its count, or switch the Perspectives and Saved
  sections off. Inbox, Today and Trash stay visible, and the bottom bar and
  tablet rail follow the same hidden set.
- **Forecast depth.** Forecast lists due, defer and planned dates, with
  **Due only** and **Planned** toggles. Project deadlines appear as project
  rows above each day, Settings → Library → **Forecast** chooses the undated
  tag and whether flagged actions appear, and dragging a task onto a month or
  week cell reschedules its due date while keeping its clock time.
- **Task row metadata.** Rows now draw the Mac's defer mark before an upcoming
  defer date and the repeat arrow before the rule summary, so deferred and
  repeating work is recognisable without opening the task.
- **Row metadata toggle.** Settings → Tasks → **Rows** can hide the line under
  task titles, leaving just the title, flag, pending chip and subtask chevron.
  The phone keeps this per device; the Mac's fuller row-layout setting is still
  to come.
- **Project rows match the Mac.** The Health icon now draws the Mac's
  stethoscope instead of a heart monitor, and a stalled project no longer reads
  **Done**: Done, Dropped and On Hold use the Mac's check, cross and pause
  marks, a live project shows its open count, and a project with nothing left
  open shows no badge.
- **Archive rows match the Mac.** Each archived project shows its icon, an
  **X of Y done** line, and its status with the same check, cross or pause.
  Swipe right to **Reopen** a Done or Dropped project, or to **Unarchive** an
  explicitly archived one; swipe left to **Delete** through the Browse
  confirmation. Both swipes tick the haptics.
- **Swipe haptics.** Every swipe action now gives the completion tick:
  complete, trash, restore, delete, reopen and unarchive. Settings → Tasks →
  **Haptics** turns them off with the rest of the haptics.
- **Full repeat rules.** The Android task editor now edits the interval,
  fixed versus after-completion, due-again versus defer-another, the weekday
  strip for day-based rules, and an end date or count, not just the unit.
- **Repeat ends on the Mac.** The Mac repeat section now has the same Ends
  row as the phone: Never, On date or After N. One summary sentence now folds
  in the done count and, for fixed schedules, the projected last date;
  after-completion rules say the dates follow each completion.

### Android reminders

- **No stale alarms.** The phone checks the shared dismissal list and the
  cached task before showing a full-screen reminder, so a Mac complete or
  snooze no longer leaves an old alarm to fire on an idle phone.
- **Overlay permission at the switch.** Turning on Full-screen reminders opens
  the **Display over other apps** screen when the permission is missing.
- **Locked reminders.** A locked phone still gets the full-screen reminder; a
  hidden overlay no longer swallows the next alarm.
- **Dismiss clears the Mac batch.** Dismiss or Esc closes every reminder in a
  simultaneous batch and clears them on the phone; Complete and Snooze still
  work one card at a time.
- **Dismiss and Snooze on the alarm card.** The reminder notification handles
  both without opening the full screen; snooze uses the saved length,
  reschedules the alarm and clears the other device.
- **Simultaneous reminders.** Tasks that come due together merge into one
  card that says how many are due, with a read-only View all list, Open
  Flowlist for a Due now page in the app, and a single Dismiss all. The Due
  now page keeps its list while a task's details are open; completing a task
  leaves the rest in place. Later alarms stay silent and the batch card has no
  per-task complete or snooze controls.

## 0.10.1 — 2026-11-27

### Organized project perspectives

- **Project rows with nested actions.** A saved view that matches projects can
  group those projects by folder, project, due, defer, planned or review date,
  added or changed date, and sort them by name, dates or library order. Each
  matched project keeps its own header and shows its matching actions
  underneath instead of leaving them in a separate task block.
- **Editor and CLI controls.** The shared Presentation section adds **Group
  projects by** and **Sort projects by**. `perspective add` accepts
  `--project-group` and `--project-sort`.

### Project completion

- **Complete with last action.** Projects now carry a per-project completion
  mode. With it on, completing the last open task marks the project Done. With
  it off, the project stays active and shows the existing tick badge.
  Single-action lists always stay manual. The inspector exposes the switch;
  `project add/edit --completion auto|manual`, the MCP project tools and the
  project JSON carry the same field.

### Archive

- **Separate archive action.** Projects have an archive flag beside their
  status and in the right-click menu. A folder's menu archives every project
  inside it in one undo step.
- **Auto archive setting.** Settings → Workflow → Archive moves Done and
  Dropped projects out of the active library by default. Turn it off and they
  stay visible with a tick for Done and an × for Dropped. Archived projects
  and their actions stay out of Quick Open, project autocomplete, the active
  project list, Today, Forecast, Flagged, Tags, Changed, Attachments, Nearby,
  the status bar, the rail counts and the reminder scheduler. The Archive
  still lists them.
- **CLI and MCP controls.** `project archive` (and `--undo`) archives one or
  more projects, `folder archive` archives every project in a folder, MCP adds
  `archive_projects` and `archive_folder`, and `update_project` accepts
  `archived`.
- **On Hold rail marker.** On Hold projects show a pause mark instead of a
  count; their tasks and subtasks stay dimmed and cannot be completed.

### Stalled projects

- **No stalled rail icon.** An active project with tasks but none left open
  shows no count or tick in the rail. Opening it shows the message
  **“All caught up — no active tasks.”** A pause icon marks On Hold; a tick is
  reserved for a project whose status is actually Done. Single-action lists
  keep their open count.
- **Completed section.** When a stalled project hides completed work, a
  **Completed (n)** row reveals it. Completed roots then sit under their own
  collapsible **Completed** heading instead of interleaving with open work.
  The notice reads **“All caught up — no active tasks.”**
- **On Hold locks work.** Projects on hold show a pause marker in the rail;
  their tasks and subtasks are dimmed, their checkboxes are disabled, and the
  command layer rejects completion until the project is active again.

### Small polish

- **No dates prompt.** A project with no due, planned or defer date shows a
  short prompt and an **Update project** link in the inspector’s Dates section.
- **Stalled notice.** A stalled project explains that there are no remaining
  actions and offers **Mark done** beside the Completed row.
- **Focus help.** The dimmed focus eye explains how to start focus when clicked
  and continues to restore the last focused projects and folders.
- **Blank saved views.** Empty saved perspectives no longer show a tick; their
  count appears only when non-zero.
- **Folder headings.** A folder view labels each project with its own name,
  indents its tasks under the heading and keeps the redundant per-row project
  label off. Clicking the heading also shows that project in the inspector.
- **Empty project pane.** An empty project says **Nothing here** with
  “Add the first task above”; a stalled project shows the usual centered
  icon with **All caught up — no active tasks** and “Add a new task above.”
- **Heading hit area.** The chevron target on a heading now covers the whole
  left gutter and row height, so clicks near it register. Sidebar folder and
  tag chevrons use their whole row-height slot, and subtask arrows stretch to
  the row height too.
- **Folder rows.** A folder's chevron alone unfolds its tree; clicking the
  folder name opens its view without changing the tree.
- **Saved view counts.** A saved view's badge counts its open actions once,
  ignoring the project headers that can sit beside them.
- **Task rows.** Shade and separator share a small front inset and the
  separator runs to the right edge without stopping at the padding. Grouped
  views such as Forecast, Today, Tags and grouped saved views indent tasks only
  when a heading is actually shown, and leave a gap before the next heading.
  Group None and unheaded lists such as Attachments stay flush left.
- **Project headers.** Projects headers drop the progress bar and **Open**
  link and use the bare project name everywhere. Only the chevron toggles
  tasks; clicking the heading shows the project in the inspector, and
  Cmd-click opens it. Folder projects get breathing room between them and
  task separators start at the circle.

### Android full-screen reminder

- **Immersive reminder layout.** The phone's full-screen reminder fills the
  screen with the selected theme or reminder preset, following the app's light
  or dark mode: light mode uses the pale preset and dark type, while dark mode
  uses the deep preset with white type. A bell, the task title, time and
  project sit above a compact top-right **Dismiss** and the actions. The system
  bar icons follow the same contrast.
- **Slide to complete.** The completion button is an iOS-style track: drag the
  white check to the end to finish, with a haptic tick.
- **Adjustable snooze.** Minus and plus decrease or increase the snooze in
  five-minute steps; the centre pill snoozes for the chosen length. Open
  Flowlist stays available below.
- **Cross-device dismiss.** Dismissing, snoozing or completing a full-screen
  reminder on either the Mac or the phone clears the matching reminder on the
  other device through a short-lived list on the mobile server. Both devices
  keep working locally when the server is offline.
- **Over any app.** With **Display over other apps** allowed, the reminder now
  draws above whichever app is in front, such as a video, instead of only
  opening over Flowlist or the lock screen. The theme background now reaches
  under the status and navigation bars, and the bar icons follow the reminder's
  light or dark type. The alarm notification stays as the fallback, and
  Settings → Notifications offers the permission row.
- **Pause media.** Audio and video pause while the reminder is on screen and
  resume when it closes. Settings → Notifications → **Pause media** turns this
  off for phones that should keep playing.
- **Settings child pages.** Opening Appearance, Tasks, Library, Notifications,
  Connection or About now makes that name the main heading and hides the bottom
  bar until Back returns to the Settings home.

### Currency icons

- **Nine currencies.** The project icon picker adds euro, pound, yen, won,
  ruble, bitcoin, franc and lira beside the existing dollar and rupee, with
  matching phone icons.

### Focus counts

- **Counts follow the focus.** Inbox, priority, tag, folder, project and saved
  view counts now count only tasks and projects inside the current focus. Tags
  with nothing in focus dim, so the rail no longer shows a full count over an
  empty list.

### Review stepper

- **Consistent stepper icons.** The review minus and plus controls use the
  shared icon set, so both sit cleanly inside their buttons.

### Review cadence

- **Next review follows the interval.** Changing **Review every** from the
  inspector now moves the next review to the last reviewed date plus the new
  interval (or today plus the interval when it has never been reviewed). An
  explicit next-review date still wins.

### Perspective editor polish

- **Simple fields under the tabs.** Contains text and the estimate range now sit
  inside the Simple tab instead of above the Simple/Comprehensive switch.
- **Narrower None warning.** The “No matches” hint appears only on
  Comprehensive, and only when an All/Any group contains a rule that also
  appears inside one of its None children.
- **Centred review steppers.** The review-interval minus and plus controls use
  drawn shapes, so they sit in the middle of their buttons.

### Calmer copy

- **Terser interface text.** View subtitles, empty states, settings
  descriptions, capture placeholders and dialog help are shorter and neutral;
  built-in views skip the subtitle row instead of restating their purpose.

## 0.10.0 — 2026-11-26

### Perspective rules

- **Rule tree.** Saved views now use one hierarchical contents tree:
  nested **All / Any / None** groups, every supported rule from **Status:
  Due Soon** through **Matches search terms**, comments, disabled rules, drag
  reordering and a top-level operator. New views start with **Availability:
  Remaining**. Date ranges cover due, planned, defer, completed, dropped, added
  and changed dates with On, Yesterday, Today, Tomorrow, In the past, In the
  next and Between.
- **Project rules and project rows.** Custom perspectives include matching
  projects as rows. **Is a project**, project-status, project-tag and
  project-date rules work. Projects now carry flagged, completed and dropped
  state for rules and date ranges.
- **First Available and availability accuracy.** First Available shows one
  action per parallel project and every available action in a single-actions
  list. Remaining and Due Soon no longer include tasks in completed or dropped
  projects.
- **Duration comparisons.** Estimated-duration rules include less than,
  greater than and a custom between range, with a custom-minutes field.
- **Date ranges and calendar picker.** On and Between date fields have a
  calendar picker. Open-ended Between ranges use an empty start or end date.
  Hour-based ranges compare timestamps rather than whole days.
- **Inherited tags, search and comments.** Tag rules and tag grouping/sorting
  see parent-action and project tags. Search terms match item, project, folder
  and tag text. Comments annotate without filtering.

### View editor

- **Simple and Comprehensive tabs.** Simple restores the previous checkbox and
  pop-up editor. Comprehensive is the full rule tree. A new view
  starts on Simple; advanced views open on Comprehensive so nothing is silently
  flattened.
- **Quiet contents list.** Indented rule text with code-editor
  guide lines, a right-side `⋯` menu, a hover plus, a faint **Add a new rule**,
  and a shared Presentation section. The icon background is square.
- **Resizable, scrollable sheet.** The editor resizes in both directions and
  remembers its size. Rule menus and tag/project pickers scroll inside
  themselves without moving the sheet. Cancel sits left of Save.

### Views and presentation

- **More grouping.** Group by Tags (combined), Completed, Dropped, Added,
  Changed and Next Review, alongside the existing project, folder, tag, dates,
  flagged and estimate choices.
- **More sorting.** Sort by Due & Flagged, Tag, Added, Changed, Dropped and Next
  Review.
- **Zero-match warning.** The editor calls out rule trees that cancel
  themselves out, such as Available plus None of Available.
- **View audit fixes.** Action groups remain groups after children complete,
  dropped dates inherit from dropped projects, and Between endpoints clear to
  open-ended ranges.

### Calendar and Forecast

- **Calendar events in Forecast.** Calendar has its own Settings section.
  Read-only events sit above each day's tasks; the month grid shows event chips
  and the week strip counts them.
- **Forecast due-only mode and Later headings.** Settings → Workflow → Forecast
  can show only due dates. The near horizon is **Later this month** and the far
  folded horizon is **Later**.
- **Calendar event times stay on one line.** Forecast shows the full
  `start – end` range on one row.
- **Calendar permission survives rebuilds.** Ad-hoc packages use a stable
  identifier-based designated requirement, so macOS does not treat every
  rebuild as a new Calendar app.

### Inspector and notifications

- **Collapsible inspector sections.** Title, Action, Notes & Attachments, Dates,
  Repeat, Tags and Notifications fold from their headers; the choice is
  remembered per device.
- **Repeat and Review.** Repeat has an editable interval, unit
  menu, weekday strip, anchored choices and a plain-language summary. Review
  shows Next Review, an inline cadence stepper and Last Reviewed.
- **Properties and Subtasks tabs.** A single task has separate Properties and
  Subtasks pages.
- **One Notifications section.** Notifications are a chronological list.
  Defaults can be restored, relative and absolute alerts added, and any
  notification can open full screen instead of as a banner.
- **Notification parity, except date time zones.** Defer, Planned, Latest Start
  and Due defaults can be enabled separately. Each task or project can remove
  or restore defaults and add relative or absolute notifications. Flowlist
  still schedules in the system time zone.
- **Tags.** Tags use a bordered chip box, a `+` control and an
  inline add field. Project tags use the same box without the input.
- **Empty sections start folded.** Sections open by default only when they have
  data.

### Settings and interface

- **Icons-only header by default.** A fresh session shows toolbar icons without
  the small titles. Titles remain available in Settings.
- **Settings remembers its size.** The Settings sheet reopens at its last
  dragged size.
- **Quieter empty notifications.** The full-screen hint appears only when a task
  actually has a notification.
- **Batch task editing.** Multi-selected tasks share a batch editor for project,
  tags, dates, priority, flag and estimate. Apply is one undoable command.
- **Forecast tag and flagged contents.** A chosen tag pulls undated actions into
  Forecast, and flagged undated actions can join them.
- **Custom notification schedules and rupee icon.** Per-task and per-project
  alerts accept signed offsets, and projects can use the rupee symbol.

## 0.9.51 — 2026-11-26

### What's new

- **Focus mode.** Scope every perspective, search, Forecast calendar and
  Review/Archive/Nearby list to one or more projects or folders. The sidebar
  dims outside the focused branches, and a full-width banner below the title bar
  names them. The eye beside the history arrows ends focus and restores the
  previous focus. Inbox and Trash stay global.
- **Multi-selection in the sidebar.** Cmd-click projects and folders to select
  several. Right-click a selected row to focus the selection, mark the selected
  projects Active/On Hold/Done/Dropped, or delete the selection in one undoable
  step.
- **Toolbar titles.** Header icon buttons can show small titles underneath.
  Search keeps only its placeholder. Turn the titles off in
  Settings → Workflow → Show button titles.

### Changes

- **Focus banner and controls.** The banner uses an opaque high-contrast
  palette. Workflow can hide it; the eye remains available, and a restored focus
  briefly shows the banner before it dismisses itself. Toolbar labels get a
  taller title bar, larger type and roomier spacing without changing the
  unlabeled layout.
- **Focus-aware counts.** Sidebar counts for task perspectives and project
  dashboards follow the focused branches; Inbox stays global.
- **Settings and inspector polish.** Workflow toggles are right-aligned, the
  Window controls status sits beside its main label, and the inspector no longer
  has a separate close ✕.

### Bug fixes

- **Reminder media.** Pause media now only stops playback when a full-screen
  reminder opens. It no longer starts previously paused media when the reminder
  closes.
- **Focus counts.** Badges update immediately with the active focus and no longer
  leave stale values such as an attachments count with no matching rows.
- **Focus eye.** The eye no longer shifts when its title changes between Focus
  and Unfocus.
- **Window controls switch.** The switch stays fixed when the shown/hidden text
  changes.

## 0.9.50 — 2026-11-25

- **Projects in Forecast.** A project now appears on its own due, defer and
  planned dates, above that day's tasks. The row shows the project icon, name,
  status, which dates it carries and its completion progress; clicking it opens
  the project. Completed and dropped projects stay out, the Planned chip hides
  planned-only project rows, and the calendar pills, counts, Past and Future
  tiles all include projects.
- **Project date constraints.** The Edit Project sheet now has **Defer**,
  **Planned** and **Due** fields. Actions inherit those dates: a project defer
  is a start-no-earlier gate, a project due is a finish-no-later deadline, and
  the task/parent can only make them later or earlier respectively. Today,
  Forecast, overdue, filters, grouping, sorting, badges and the task inspector
  use the effective dates. Planned remains constraint-free.
- **Strict project dates.** A task can no longer be saved with a due date later
  than its project/parent due, or a defer date earlier than its container
  defer. Dragging such a task into a project opens an **Adjust dates?** sheet;
  confirming changes the stored dates and moves in one undoable step. Cancel
  moves nothing. CLI/MCP writes that would create a contradiction are rejected.
  Existing contradictory libraries are clamped once on open.
- **Strict dates everywhere.** Typing a project into the task inspector now
  shows the same Adjust dates sheet; editing a task date outside the limits
  keeps the field open with an error; changing a project’s due/defer asks
  before clamping its existing tasks.
- **Clearer date warnings.** Refusals now stay inside the inspector or edit sheet
  instead of opening a separate error box. They read directly, for example
  “Due date cannot be later than the project’s due date”, and use a yellow
  warning tone. The Adjust dates sheet shows only each current date and its
  replacement. Same-day times are allowed when the
  calendar day matches the container. Forecast also puts tasks belonging to
  that day’s project rows first and labels the action section **Tasks**, so a
  nearby task is not mistaken for a child.
- **Inherited date labels.** The task inspector now says “Defer with project”,
  “Planned with project” or “Due with project” (and “with parent” for an action
  group) when the shown date comes from a container instead of the task.
- **Planned dates.** A third date beside Due and Defer that never changes
  availability. The inspector gets a Planned calendar field, quick capture
  accepts `planned:` / `plan:`, Forecast gains a Planned Today bucket and
  planned days on the calendar, filters/sorts/groups understand it, the CLI
  has `--planned`, and MCP reports `planned` / `planned_at`.
- **Planned dates inherit.** A project can carry a planned date; its actions
  inherit it unless the action or a parent action has its own planned date.
  The Edit Project sheet has a Planned date field, the project inspector shows
  it, and the CLI/MCP accept `planned`. Today, Forecast, the Planned filter,
  planned grouping/sorting and calendar dots all use the inherited date.
  Planned dates still never change availability.
- **Past plans stay visible.** A planned date that slips no longer disappears.
  Today lists carried-over plans in a **Past plans** group above Overdue, and
  Forecast gains a **Past** section. One **Hide past** chip hides both. Forecast
  now treats Due, Defer and Planned as independent dates, so a task planned
  today and due next week appears in both places, and clicking a planned day
  lists it.
- **Three dates at a glance.** Rows and inspector labels carry distinct marks
  — a turning arrow for Defer, a signpost for Planned and a clock
  on a calendar for Due — so the dates never read as the same thing. Today now
  includes tasks planned for today and its subtitle says so. Saved views gain
  a Planned dates filter (Any, Overdue, Today, Tomorrow, Next 7/30 days, No
  planned date).
- **Quick Open.** `⌘O` opens a keyboard-first palette for perspectives,
  folders, projects, tags and priorities. It starts empty, lists matches as you
  type, scrolls its results inside the panel instead of growing past the frame,
  and keeps the highlighted row in view. Use the arrows to move and Return to
  open. Import moves to `⇧⌘O`.
- **Piano-key Forecast strip.** The collapsed Forecast calendar is now a
  linear date strip with fixed **Past** and **Future** end tiles and a paged
  week of days between them. Past shows overdue work and slipped plans; Future
  shows due, deferred and planned work after the visible week. Both respect the
  Planned chip.
- **Next month and later.** Forecast moves due or planned work more than 30
  days out into a **Next month and later** group, folded by default so the
  coming week stays at the top.
- **Forecast Planned chip.** Forecast can hide everything that is only there
  because of a planned date, leaving due and deferred work. The choice is
  remembered per session, and the chip now sits beside Calendar instead of
  stacking under it.
- **Running and pinned views.** New and existing libraries get a pinned
  **Running** saved view for work that has started and is not due yet; it is an
  ordinary saved view and can be edited or deleted. Any saved view can be
  pinned to the top of **SAVED** from its context menu or the editor
  (`perspective add --pinned`), and any saved view can take a custom rail icon
  from the editor or `perspective add --icon`. Filters gain a **Later / none**
  window.
- **Searchable icon card with emoji.** Projects and saved views can use any
  named icon or an emoji. The old wrapped icon row is now a card with a search
  field, **Icons** and **Emoji** tabs, and Return to pick the first match. The
  card scrolls with the sheet instead of trapping the wheel, and opening it no
  longer jumps the sheet to the bottom. Emojis are stored as plain strings in
  `icon`, and the CLI accepts `--icon 🏃`. The retired running figure is gone;
  existing libraries are cleaned on open.
- **Edit project and folder.** The sidebar context menus now open an Edit sheet:
  projects can change name, notes, review cadence, type and icon; folders can
  be renamed without leaving the rail.
- **Descriptions on saved views.** A saved view can carry a one-line
  description under its title, in the same place Today explains itself. An
  empty description falls back to “Saved perspective”.
  `perspective add --description` sets it from the CLI (`--note` still works).
- **Delete confirmations.** Removing a project or tag from the sidebar now
  opens a confirmation sheet instead of deleting immediately. Undo still
  restores it.
- **Default wording.** Sort and Group now say **Default** where they used to
  say View: “use this screen’s natural order or grouping”. Manual still means
  your drag order, and None still means one flat list.
- **Only count due items.** A Workflow setting makes the Forecast badge count
  due work alone, ignoring planned and deferred items.
- **Folders drop the redundant prefix.** A task row inside a folder now shows
  the remaining project path (`Shopping`) instead of repeating the folder
  (`Personal / Shopping`). Other views still show the full path.
- **Inspector gutter.** The scrollbar gutter only appears when the inspector
  actually scrolls; short task sheets now use the full width.
- **No Full-screen sidebar row.** The Full-screen list is no longer offered as a
  perspective; it remains a temporary destination for a reminder's **View all**
  action.
- **Reminder focus.** Completing, snoozing or dismissing a full-screen reminder
  restores the app that owned the screen instead of bringing Flowlist forward.
- **Background terminology.** Wallpaper controls and picker text are now
  labelled Background throughout the app. A single background picker no longer
  says “light” when the same image is used in both appearances.
- **Custom background controls.** “Use background” is now **Use custom
  background** in Appearance and on the reminder. When it is off, the picker,
  separate light/dark toggle, thumbnail, opacity and notes are hidden.
- **Pause media on full-screen reminders.** A new Reminders setting pauses
  system playback when a full-screen reminder opens and resumes it when the
  reminder closes. It uses MediaRemote's transport command only when audio is
  actually playing. macOS gates this behind **Device control and data access**,
  so the enabled setting explains where to allow Flowlist.

## 0.9.49 — 2026-11-24

- **Android full-screen counts.** Saved views using the full-screen reminder
  filter now evaluate it on Android instead of ignoring it, and the count
  includes matched subtasks just like the Mac sidebar badge.
- **Wallpaper opacity fix.** A wallpaper chosen only for dark mode no longer
  makes the plain light content surface translucent.
- **Quick capture polish.** The `⌃⌥Space` shortcut now toggles the panel: a
  second press puts it away without filing. Removed the outer keycap boxes but
  kept `esc`, `⏎` and `⌘⏎` as plain text beside Add / Add & open. The panel is opaque and its
  text is pixel-aligned with a normal weight, so the input no longer looks
  soft.
- **Background polish.** The main app keeps a custom wallpaper with an
  opacity slider. The twelve ambient gradients now belong to the full-screen
  reminder, which can also use a wallpaper with its own opacity. Abstract
  styles and wallpapers have visual previews, and the picker shows a
  “selected · Save to apply” note. HEIC photos are converted with `sips`.
- **Per-reminder timing.** A full-screen reminder can follow the task's due
  date and time, or carry its own date and time. Mac's inspector and Android's
  task editor expose the choice. The picker is disabled while matching due.
- **Reminder polish.** Snooze length is 15, 30 or 60 minutes. On the full-screen
  reminder, **S** cycles the length, **Enter** snoozes with it and **⌘Enter**
  completes the task. The inspector gains a scrollbar with breathing room.
  Abstract backdrops are single crisp theme-tinted gradients, so previews fill
  their tiles instead of stopping short and no longer glow in the middle.
  JPEG XL wallpapers are converted to PNG with `sips`, and replacing a
  wallpaper now always loads the new image.
- **Full-screen views and empty-view cleanup.** The saved-view editor can
  filter on full-screen reminder marks, including marks inherited from a
  project or folder. **Settings → Sidebar → Hide empty custom views** keeps a
  saved view out of the rail until its filter matches again.
- **Light and dark wallpapers.** Appearance now stores a separate wallpaper
  for each appearance, with one opacity choice for both. Older single
  wallpapers migrate into both slots.
- **Reminders stay in the future.** The inspector refuses a reminder timestamp
  before now, greys out past days in the calendar, and rejects past text.
- **Fixed settings rail.** Scrolling a settings section no longer scrolls the
  section rail; only the pane moves.
- **Quick-entry full-screen reminders.** Type `fsr:1h30m`, `fsr:2d`, or bare
  `fsr` to follow the due time instead of using a capture button. Compact
  offsets such as `due:3d` work too. A mark without a due or explicit reminder
  time is ignored.
- **Full-screen view.** A built-in **Full-screen** view lists tasks with an
  effective reminder. When several reminders come due together, the reminder
  offers **View all** (or **V**) instead of stepping through them.
- **Optional split wallpapers.** A Settings toggle reveals separate light and
  dark pickers for the main window and for full-screen reminders; off keeps a
  single wallpaper for both.
- **Full-screen test button.** Mac opens the actual reminder window; Android
  opens the alarm screen directly instead of posting a regular notification.
- **Mac file-picker fix.** Choosing a wallpaper uses the asynchronous GPUI
  prompt, so it no longer aborts the app.

## 0.9.48 — 2026-11-24

- **Fix the Mac wallpaper picker.** Choosing a wallpaper from Settings no
  longer aborts the app. The picker now uses the same asynchronous macOS file
  prompt as attachments instead of a nested modal event loop.

## 0.9.47 — 2026-11-24

- **Mac wallpaper and reminder backdrops.** Settings → Appearance has a
  custom wallpaper behind the main window, with an opacity choice. The
  full-screen reminder has six theme-synced abstract styles (Aurora, Dunes,
  Halo, Mesh, Tide, Ember), a Match-theme default, or its own wallpaper and
  opacity. The reminder content sits on an opaque card so text stays readable.
- **Full-screen task reminders.** Tasks, projects and folders can be marked as
  full-screen reminders when they are created (or from the task inspector).
  Marking a project or folder covers its tasks. On Mac, a marked task that
  comes due while Flowlist is running opens a borderless screen-sized reminder
  with Complete, Snooze 10 minutes, Open task and Dismiss. Android posts an
  alarm-style full-screen-intent reminder with the same actions, rescheduled
  after a reboot. Nothing takes over the screen unless it carries the mark or
  inherits it from its project or folder.
- **Android wallpaper and reminder backgrounds.** The main window can show a
  custom wallpaper with its own opacity slider. The twelve ambient gradients,
  one tuned for each theme family, now belong to the full-screen reminder,
  with **Match theme**, free choice regardless of theme, and **None**. The
  reminder can also use the wallpaper with a separate opacity. Cards, sheets
  and menus stay opaque, with one scrim under the main content, so text does
  not bleed through the background. The Full-screen reminders card is first in
  Settings → Notifications, and the capture sheet shows the per-task toggle
  above its chips.
- **Cleaner Mac login item.** The mobile server agent is associated with the
  Flowlist app bundle, so Login Items lists one **Flowlist** row with the app
  icon. Installing from a bare build now points the agent at the packaged app
  instead of `target/release/flowlist`, which previously left an anonymous
  duplicate behind.

## 0.9.46 — 2026-11-23

- **Mac Forecast week strip.** The Calendar chip now only hides and restores
  the calendar, remembering whether it was the week strip or the full month,
  and hiding clears any picked day so the list always returns to the full
  agenda. A chevron beside the month name folds the full month to a
  Monday-first current-week strip with day counts, forward-only week arrows or
  horizontal scroll, and a selected day that clears on a second click. The
  week strip's chevron opens the unchanged full month, which keeps its resize
  grip. Taller strip cells stop the day number and count clipping, and the
  header count reads “1 task for 23 Nov” while a day is picked.
- **Mac inspector.** Text fields gained a thin overlay caret, click placement,
  double-click word selection, triple-click select-all and drag selection.
  Step modes show only the active list, dim blocked children, and confirm the
  destructive one-off conversion; each child gets its own separate and delete
  controls.
- **Mac task rows and colours.** The subtask chevron hit area now includes the
  gap before the flag, priority circles use vivid red and yellow with thicker
  rings, and the row icon cluster has more breathing room.
- **Mac window and library.** Quit, titlebar zoom and Dock reopen behave, and
  Attachments rows are fully selectable.
- **Android Forecast.** Added the forward-only current-week strip with a month
  chevron, a draggable bottom pill, animated expand/collapse, scrolling full
  month, and the same day-filter header count.
- **Android trash, sorting and capture.** Trash uses right-swipe restore and
  left-swipe delete with a settings choice, each view keeps its own sort, quick
  add opens in a separate task affinity, and the widget refreshes its theme on
  stop.
- **Faster sorting.** Case-insensitive list, forecast, tag and selector sorts
  precompute their keys once per row instead of once per comparison. Android
  Forecast reuses its derived day counts, sorted day list and date formatters
  across recompositions. The 10,000-task `list` check in
  `scripts/perf-check.sh` dropped from about 160 ms to about 155 ms, with the
  largest gains on larger libraries.

## 0.9.45 — 2026-11-23

- **Mac sidebar and Settings rebuilt.** Saved perspectives show a count or a
  bare tick for an empty view. Settings → Sidebar is a collapsible matrix of
  every perspective, priority, folder, project, saved view and tag: Hide and
  Hide count sit side by side, hidden rows are struck through, counts grey out
  with the row, and Inbox/Today/Trash have no Hide box at all. The Settings
  sheet is resizable and uses a Zed-style scrollbar that tracks and jumps
  correctly.
- **Android drawer counts.** Every row shows its task count; Tags and Nearby
  count tasks rather than tags, zero counts stay hidden, and an empty saved
  perspective shows a bare tick.
- **Perspective editor parity.** Mac and Android now offer the same filters:
  text, Inbox only, Flagged only, status, projects, tags with child tags,
  priorities, due and defer windows, and an estimate range. Grouping covers
  project, folder, tag, due, defer, flagged and estimate; sorting adds priority.
  Multi-select menus use checkboxes, and estimate min/max sit side by side.
- **Android themes.** Light and dark families are chosen independently. The old
  Todoist family is now Tomato and existing saves migrate automatically.
- **Android lists and Forecast.** Attachment headings use priority colours and
  the theme accent only there, with filled badges for No priority and missing
  dates that invert in dark mode. Forecast keeps Today and the arrows under the
  month name only in Month mode, uses larger touch targets, and scrolls the
  calendar with the day’s tasks. Select mode toggles off on a second press.
- **Smaller fixes.** Completing a Mac task clears the inspector; Android
  favicons retry common paths and decode PNG-in-ICO files; due/priority section
  headings and attachment spacing are tidier.

## 0.9.44 — 2026-11-23

- **Geotag reminders use their own Android notification channel.** “Geotag
  reminders” now appears separately from “Due tasks” in Android Settings →
  Apps → Flowlist → Notifications, so its sound and behaviour can be changed
  there.
- **No separate app Settings page.** Geotag reminders stay in the app’s
  Notifications section, with a direct link to the Android channel settings.

## 0.9.43 — 2026-11-23

- **Widget theme follows the device instantly.** Glance now carries day and
  night colour pairs in the RemoteViews, so Android can switch the widget
  without waking Flowlist. Verified with the app force-stopped.
- **Widget icon and preview.** The header uses the selected launcher icon
  colour, and the widget picker shows a real Today preview.
- **Geotags settings.** Android Settings has a separate **Geotags** category
  for arrival reminders, permissions and the test reminder.
- **Mac tag groups.** The sidebar TAGS section is split into **General** and
  **Geotags**, preserving each tag tree.
- **Dropped from the roadmap:** custom geotag notification sounds.

## 0.9.42 — 2026-11-23

- **Widget follows the app theme.** “Follow app” now uses the app’s light/dark
  choice instead of only the system night mode, and the widget repaints when the
  app process sees a configuration change.
- **Widget icon matches the launcher icon.** The header uses the currently
  selected app-icon colour and refreshes when the icon changes in Settings.
- **Widget picker preview.** The picker now shows a real Today-widget preview
  instead of the blank loading layout.

## 0.9.41 — 2026-11-23

- **Task rows are joined again.** The thin drag zone that sat between every
  top-level row is gone, so consecutive rows touch as before. One drop target
  remains before the first row of each group, and rows still accept a drop
  after themselves. Metadata keeps its normal gaps and still wraps.

## 0.9.40 — 2026-11-23

- **Mac metadata cells sit together again.** Project, tags, estimate, due and
  the other row details use the tighter spacing from before, while still
  wrapping onto a second line when the row runs out of room.

## 0.9.39 — 2026-11-22

- **Empty widget illustration.** An empty widget now shows a light/dark 2D
  checklist drawing instead of only the capture hint.
- **Translucent bottom bar.** The phone navigation bar uses a translucent
  surface with a hairline divider, so it reads as a floating bar rather than a
  solid block.

## 0.9.38 — 2026-11-22

- **Removed the widget chevron.** The inline Today / Inbox / Upcoming switcher
  was too small and could freeze on the chooser. The widget view is chosen in
  its settings again; the header keeps the app icon, title, count and plus.
- **Quick add stays on the home screen.** The floating capture card from 0.9.37
  remains the widget’s **+** action.
- **Widget settings rebuilt.** Options are grouped into Show, Layout, Appearance
  and Content cards with a sticky Save/Cancel bar. The new **Blur** slider adds
  a frosted veil; Android widgets cannot sample the wallpaper, so this is a
  translucent frost rather than a true backdrop blur.
- **Centred empty state.** An empty widget puts “Tap + to capture” in the middle
  of the card.

## 0.9.37 — 2026-11-22

- **Widget quick add stays on the home screen.** The widget’s **+** now opens a
  small floating capture card instead of the full app. It parses the same quick
  entry, adds to Inbox and refreshes the widget.
- **Switch views from the widget.** A chevron beside the header title opens an
  inline Today / Inbox / Upcoming chooser, so changing the widget’s view no
  longer means reopening its settings.

## 0.9.36 — 2026-11-22

- **Independent widget header size.** The title bar has its own Compact,
  Default, Large and Extra large choice, separate from the task text, and the
  default is smaller than the oversized header in 0.9.35.
- **Widget layout closer to the reference.** The header now uses the app icon,
  a compact title and a count pill, with the date and **Overdue** as section
  headings. Each row puts the due date under the title and the project on the
  right.
- **Widget rows are calmer.** The **Tmrw** reschedule action is gone; the row
  is just checkbox, title and details again. Quick add and in-place completion
  remain.

## 0.9.35 — 2026-11-22

- **Configurable Android widgets.** Each Flowlist widget can show Today, Inbox
  or Upcoming, with its own theme, row count, project labels, completed rows and
  quick-add button. The widget is resizable and opens its configuration sheet
  when placed or reconfigured.
- **Widget actions.** Checking a row completes the task, **Tmrw** moves it to
  tomorrow, and **+** opens the capture sheet without navigating through the
  app.
- **Widget appearance.** Each widget has a text-size choice, a background
  opacity slider and a frosted tint for a softer glass look. Android widgets
  cannot blur the wallpaper behind them, so frosted is a translucent tint
  rather than a real backdrop blur.
- **Add from Settings.** Settings → Tasks → Home-screen widgets opens the
  launcher's pin flow without finding the widget picker.

## 0.9.34 — 2026-11-22

- **Android tablet layout.** Large screens swap the phone bottom bar for a
  navigation rail with the chosen perspectives and Settings. The drawer still
  holds the other perspectives, selection actions keep their bottom bar, and
  edge-swipe drawer gestures are disabled on tablets so lists scroll freely.
- **Planned next.** Todoist-inspired configurable widgets, widget actions and
  per-geotag notification sounds are recorded in the plan and backlog.

## 0.9.33 — 2026-11-22

- **Android history is folded away.** Undo and Redo sit under a **History**
  heading in the drawer and stay collapsed until tapped, so a stray touch near
  the bottom of the drawer cannot change anything.
- **Mac title editing wraps.** A long task title now edits in a wrapping field
  the same way it reads, keeping the caret visible instead of pushing it past
  a single-line field. A GUI probe checks the editor's painted height.

## 0.9.32 — 2026-11-22

- **Android tag status.** Tags can be Available, Blocked or On Hold, and the
  status is inherited by child tags. Blocked and on-hold tags are marked in the
  Tags list and tag sheet, and tasks carrying them show **Blocked by #tag** in
  the list. Perspective status filters now treat blocked tags as unavailable.

## 0.9.31 — 2026-11-22

- **Android project icons.** The project editor carries the Mac’s ten-icon set
  (project, folder, star, briefcase, home, heart, book, code, rocket, coffee)
  as a scrollable picker. Browse rows use the chosen icon instead of a status
  dot, tinted by whether the project still has open work. The default project
  icon is a clipboard, distinct from the folder icon.

## 0.9.30 — 2026-11-22

- **Undo and redo on Android.** The drawer’s history rows name the action that
  Undo or Redo will apply. Pending phone changes are flushed first, then the
  Mac’s shared history performs the step and returns a fresh snapshot. The rows
  disable while offline.
- **The mobile API exposes history.** `POST /api/v1/undo` and `/redo` return
  the outcome label, the new snapshot and the next undo/redo labels; the
  snapshot response carries the same history summary.

## 0.9.29 — 2026-11-22

- **Mac link favicons.** Link attachments in the inspector show the site's own
  favicon instead of the chain icon. Icons are fetched from the linked host
  (never a third-party favicon service), cached per host for the session, and
  fall back to the chain when a site has none. Project links get the same
  treatment.

## 0.9.28 — 2026-11-22

- **Long task titles wrap in the Mac inspector.** The title field grows to two
  lines instead of overflowing its box; every other inspector field stays one
  line tall.

## 0.9.27 — 2026-11-22

- **Cleaner geotagging.** The tag editor no longer shows latitude and longitude
  fields. Search a town or address, pick a suggestion, and the chosen place
  appears as a compact card with its coordinates, a Maps action and a clear
  button. “Use my location” sits below it. The “No places found” hint only
  appears after a search that returned nothing.

## 0.9.26 — 2026-11-22

- **Search for a place when geotagging.** The tag editor now has a place field:
  type a town or address (for example “Guntur”), pick a suggestion, and its
  latitude/longitude fill in. Results come from the phone's geocoder, so no API
  key or map SDK is needed. Manual coordinates and **Use my location** remain.

## 0.9.25 — 2026-11-22

- **Nearby uses the phone's location.** The Nearby perspective asks for location
  access and measures from the phone, falling back to the library location, and
  shows which source it is using.
- **Geotags on tags.** The tag editor carries latitude/longitude fields, a
  **Use my location** button, **Maps** to open the coordinates in Google Maps or
  another maps app, and **Clear**. Geotagged tags show a pin in the task
  editor's tag list.
- **Arrival reminders.** A geofence is registered for every geotagged tag with
  open tasks (250 m). Arriving posts a notification naming the tag and its
  tasks. Settings → Tasks → Nearby reminders has the switch, the location and
  background-location prompts, and a test reminder.

## 0.9.24 — 2026-11-22

- **Bulk actions on search results.** A checklist button in the search sheet
  selects matching tasks; Complete or Trash applies to the whole selection in
  one go. The sheet closes so the Undo toast stays visible.

## 0.9.23 — 2026-11-22

- **Search covers the whole library.** Results now include projects, folders,
  tags and saved perspectives alongside tasks. A **This view / Library** switch
  keeps task results inside the current list or searches everything; tag
  results open the tasks carrying that tag.
- **Nearby comes to Android.** A new Nearby perspective lists the tags with a
  location inside the library's radius, sorted by distance, with task counts.
  Tapping a tag opens its tasks. The library location and radius come from the
  Mac's Settings → Nearby.

## 0.9.22 — 2026-11-22

- **Android attachment previews.** Tapping an image or PDF in the task editor
  opens a preview card instead of leaving the app. Images scale to fit; PDFs
  render one page at a time with previous/next controls and a page count.
  **Open externally** and **Close** remain on the card. The phone uses a saved
  offline copy when present and otherwise fetches the file from the Mac.

## 0.9.21 — 2026-11-22

- **Android project editing matches the Mac.** The project sheet now carries
  the project **type** (Parallel, Sequential, Single action), **notes** and the
  **review interval**, alongside name, folder and status. New projects start
  with the library's default review interval.

## 0.9.20 — 2026-11-22

- **The whole Android task row is the touch target.** Tapping anywhere in a
  row now shows the press ripple across the full cell instead of only under the
  title text. The circle, chevron, flag and selection controls keep their own
  actions.

## 0.9.19 — 2026-11-22

- **Sharing asks where it goes.** A shared link or file now offers **New task**
  or **Add to existing task**. The picker searches active tasks and attaches
  the link or the uploaded file in place, without creating a duplicate.
- **Link favicons.** Link rows in the Android task editor show the site's own
  favicon, falling back to the chain icon when the site has none. The icon is
  fetched from the link's host, not a third-party favicon service.

## 0.9.18 — 2026-11-22

- **Sharing asks for a name first.** Sharing a link or file now opens the
  capture sheet with the page title or file name prefilled and an attachment
  chip. Rename the task, then Add to Inbox; the link or uploaded file is
  attached to that task. Previously the task was created silently with a name
  you could not choose.

## 0.9.17 — 2026-11-22

- **Mac: one row, aligned.** The circle, chevron, flag and task name share one
  line; attachments, project, tags, dates and the subtask badge drop to a second
  line only when they would touch the name. The badge reads `[done/total]`.
- **Keep completed subtasks.** A new per-device setting on Mac (Settings →
  Workflow) and Android (Settings → Tasks) leaves finished subtasks under their
  parent, struck through and ticked, instead of hiding them.
- **Mac: Completed and Trash show their tasks again.** The expansion pass was
  re-filtering rows the view had already selected.
- **QR pairing.** Settings → Mobile draws a QR carrying every Mac address and
  the live pairing code. The Android Pair screen scans it, fills the fields and
  tries each address in order (LAN, then Tailscale).
- **Share to Flowlist on Android.** Plain text opens the capture sheet
  prefilled. A shared link becomes a task with the URL as its attachment, and a
  shared file is uploaded to the Mac and attached to a new task.
- **Multi-select on Android.** The header checklist opens select mode: bulk
  complete or reopen, bulk trash with one Undo, and Trash select-all plus a
  confirmed delete-forever.
- **Priority while capturing.** Quick entry accepts `@1`, `@2` and `@3` for
  high, medium and low; the Android capture sheet has an `@1 high` chip.
- **Android toasts swipe away.** A new message replaces the visible toast
  instead of queueing, and a left or right swipe dismisses it.
- **Android task and capture sheets sit below the status bar.**
- **Subtasks tick immediately.** Optimistic updates no longer wait for the
  server round-trip, and the completion toggle reopens a finished task instead
  of completing it again.

## 0.9.16 — 2026-11-22

- **Mac: the vertical priority stripe is gone.** The task circle already carries
  the priority colour, so the extra line was noise.
- **Mac: the subtask chevron works on the first click.** Collapsed branches now
  use the same default as the disclosure, so one click opens them.
- **Easier drag-out on both platforms.** On Mac, dropping a subtask on the
  lower third of a root row moves it out to the project root; the whole row
  tints to show the target. On Android the root-out zone is the bottom 45% and
  the dragged and target rows highlight as full cells, not just the text.
- **Android drag grips sit at the far right** of subtask and one-off rows. Mac
  keeps the grips where they were.

## 0.9.15 — 2026-11-22

- **One step type per task.** The task editor now has a single **Parallel /
  Sequential / One-off** selector instead of two separate toggles. Only one is
  active, and the mode is saved with the task. Parallel and Sequential show
  child tasks; One-off shows checklist-style steps.
- **Links can be named and renamed on Mac and Android.** The Add link dialog
  has an optional **Name** field; leaving it empty keeps the current display.
  Existing links get a **Rename** action, and the name is stored with the link.

## 0.9.14 — 2026-11-22

- **Adding a subtask on Android works.** The create command sent both the
  parent and the parent's project, which the engine rejects; the child now
  inherits the project from its parent.
- **The Android editor's top row is the task title itself.** The "Task" label
  is gone, the circle sits beside the title, and the row starts a little lower.
- **Parallel and Sequential chips** are in the Android task editor for a
  task's own subtasks.
- **One-off steps are a second subtask type on Mac and Android.** Choose
  **One-off** above the add field; one-offs tick, add inline, drag to reorder
  and delete like checklist steps, and are stored with the task rather than
  appearing as separate tasks.
- **Links no longer show a download button.** The offline-copy control only
  appears for file attachments; links open in the browser.

## 0.9.13 — 2026-11-22

- **Checklists are gone from the Mac and the phone; subtasks replace them.**
  The checklist panel and the phone's checklist section are removed. Existing
  checklist data stays in the library file but is no longer shown.
- **The Mac inspector shows subtasks like the old checklist.** Rows tick in
  place, click to select, drag to reorder, and **Separate** moves one out from
  under its parent. The field at the bottom adds a subtask without leaving the
  inspector; `⌘L` focuses it.
- **The Android task editor reorders subtasks by dragging** the grip, with the
  other rows moving aside and the list settling into place on release.
- **Drag tasks in the Android list.** Long-press a task and drop it on another
  task to nest it; drop it on the lower third of a root row to move it back out
  to the project or Inbox at that position.
- **Android can attach links.** **Add link** sits under **Add attachment**;
  links open in the browser and sync through the same attachment list.

## 0.9.12 — 2026-11-22

- **Root-level drop gaps work in every list.** Dragging a subtask out of its
  parent now works in Today, Flagged, Tags and saved perspectives too, not only
  in projects and the Inbox. The task files beside the row below the gap.
- **Subtasks are collapsed by default on Mac and the phone.** A chevron on the
  parent row opens the children; Mac keeps `za` / `zo` / `zc` for the keyboard.
- **Mac task circles use priority colours:** red for high, yellow for medium
  and blue for low, matching the phone. No priority keeps the neutral ring, and
  completed tasks keep the filled accent check.

## 0.9.11 — 2026-11-22

- **The phone drawer settles smoothly on release.** The dragged row now slides
  into the gap instead of jumping when the saved order is applied.
- **The Mac inspector lists subtasks.** Each child has a **Separate** action
  that moves it out from under the parent to the end of the project or Inbox.
- **Drag a subtask out of its parent in the Mac middle pane.** While dragging,
  a line appears in the gaps between root-level tasks; dropping there moves the
  task out and places it at that position.

## 0.9.10 — 2026-11-22

- **Flagged is a perspective on the phone.** It sits next to Forecast in the
  drawer, lists every flagged task, and can be pinned to one of the four
  bottom-bar slots from Settings → Bottom bar.
- **Priorities has its own icon** instead of sharing the flag with Flagged,
  and **Review** now shows the same coffee cup as the Mac.
- **Drag perspectives in the phone drawer to reorder them.** The dragged row
  lifts with an accent outline, the rows it passes slide aside to open a gap,
  and the order is remembered after the app restarts.
- **The Mac inspector has a + Add subtask button.** It creates the task under
  the selected one and puts the cursor in its title.

## 0.9.9 — 2026-11-22

- **Clearing a priority now sticks on the phone.** An explicit `null` in a
  task patch clears the field instead of being read as "unchanged". The same
  fix makes clearing a due date, defer date, project, estimate, repeat or
  attachment list work from the phone. Partial patches are also accepted now.
- **Changing the priority of a repeating task asks for a scope.** Choose
  **This task** for one occurrence or **All repeats** for the whole series,
  on both the Mac and the phone. Future occurrences inherit the series
  priority.
- **A defer date can no longer fall after the due date.** The engine rejects
  the pair for tasks and projects; the Mac keeps its date picker open with a
  toast, and the phone's task editor explains before saving.
- **Phone priority pills are H, M and L**, and the selected pill takes the
  priority colour: red, yellow or blue. Task circles use the same three
  colours, so low priority is blue instead of the theme's red accent.

## 0.9.8 — 2026-11-21

- **Task circles show priority:** red for high, yellow for medium, and the
  theme accent for low. Completed tasks keep the accent-filled check.
- Priority no longer draws its own flag; the flag on the right means the task
  is flagged.

## 0.9.7 — 2026-11-21

- **Forecast remembers Month or Agenda** across app restarts.
- **Create a project or tag from the task editor.** The project menu is the
  same width as the project row, and its last item is **Create new project**.
  Tags gain a trailing **+** chip. Both auto-select the new item on the task.
- **Tag chips wrap onto new lines** instead of running off the screen.
- **Flagged tasks show their flag at the far right** of the task row.

## 0.9.6 — 2026-11-21

- **Settings → Library → Offline files** has a **Save all attachments** toggle.
  Turning it on saves every attachment in the library, including project
  attachments, and keeps new ones saved as the library syncs.
- **Remove all offline copies** sits below it, with a confirmation warning;
  removing them also turns the toggle off.
- Fixed large file downloads truncating because accepted sockets inherited the
  listener's non-blocking mode. Transfers now complete for 20 MB+ files.

## 0.9.5 — 2026-11-21

- **Attachments can be saved for offline use.** The download button on each
  attachment copies it into the app's private files, marks it as saved, and
  opening it no longer needs the Mac. Tapping the saved button again removes
  the offline copy. Removing an attachment also removes its saved copy.

## 0.9.4 — 2026-11-21

- **Attachments on the phone.** Pick any file from the task editor; it is
  uploaded to `attachments/` inside the library and added to the task.
  Tap an attachment to download and open it, or remove it.
- **File endpoints on the Mac server:** authenticated `POST /api/v1/files`
  and `GET /api/v1/files`, limited to 40 MB and to files inside the
  attachments folder or already listed on a task/project.
- The Settings header no longer repeats a subtitle, but keeps its line of
  vertical space.

## 0.9.3 — 2026-11-21

- Perspective creation uses labelled dropdowns for project, tag, priority,
  due, grouping and sorting instead of horizontal pill rows. Long lists
  scroll inside the menu.

## 0.9.2 — 2026-11-21

- **Forecast has a month grid.** Days show how many tasks are due, with
  previous/next month and Today controls; tapping a day lists its tasks.
- **Agenda** keeps the original rolling two-week list one tap away.
- **Saved perspectives** live in the drawer. Build one from text, project,
  tag, priority and due filters, with grouping, sorting and show-completed.
  They use the Mac's `save_perspective` command and can be edited or deleted.

## 0.9.1 — 2026-11-21

- **Checklist steps can be added, reordered and deleted** in the task editor.
  Up and down arrows move a step one place; the delete button removes it.
- **Subtasks can be moved under another top-level task** from the task editor,
  with a searchable picker.
- Per-task reminders are not part of the core model yet, so they remain a
  Mac-side due-time notification.

## 0.9.0 — 2026-11-21

- **Library management on the phone.** Browse now groups projects under
  folders, with New project and New folder actions. Project menus edit the
  name, folder and status, archive, or delete with its tasks. Folder menus
  rename, nest, add a project or folder inside, or delete the tree.
- **Tags can be created, renamed, re-parented and deleted** from the Tags
  perspective, using the same hierarchy rules as the Mac.
- **Moving a root tag under another tag** no longer leaves it duplicated in
  the root list; fixed in `flowlist-core` with a regression test.
- Removed the dead `SettingsSheet` composable left over from 0.8.

## 0.8.9 — 2026-11-21

- Tag counts now match the tag view: active tasks only, including child tags,
  and never completed or trashed tasks.

## 0.8.8 — 2026-11-21

- Back inside a tag now returns to the Tags list instead of leaving the
  perspective entirely. A second back leaves Tags.

## 0.8.7 — 2026-11-21

- The drawer gained a **Perspectives** heading above its list.

## 0.8.6 — 2026-11-21

- The drawer lists only the perspectives that are not already on the bottom
  bar. Library and Settings are no longer repeated there; the connection
  status stays pinned at the bottom.

## 0.8.5 — 2026-11-21

- The phone heading and its subtitle now start on the same left edge.
- Settings → Appearance → **Bottom bar** chooses the four perspectives shown
  in the bottom bar; Settings is always the fifth, on the far right.
- Review, Completed, Archive, Trash, Tags, Priorities, Changed and Attachments
  open as full destinations instead of modal sheets.

## 0.8.4 — 2026-11-21

- The perspective heading now has a little more air below the search row.

## 0.8.3 — 2026-11-21

- Removed a duplicated status-bar inset: the header and Settings were being
  padded once by the Scaffold and again by their own `statusBarsPadding`, which
  made them look unchanged after 0.8.2. Both now start directly under the
  system bar.

## 0.8.2 — 2026-11-21

- The phone header sits closer to the top: no extra top padding, and the
  perspective title follows the search row sooner.

## 0.8.1 — 2026-11-21

- The phone header is a TickTick-style two-row header: hamburger and a full
  search field on the first line, the perspective title and date underneath.
- The drawer is square-edged, 310 dp wide and starts straight at **Library**;
  the app name and logo header are gone. Settings and connection status sit at
  the bottom.
- The Settings home no longer repeats the title.

## 0.8.0 — 2026-11-21

- **Settings is a destination, not a sheet.** The fifth tab opens a settings
  home with Appearance, Tasks, Library, Notifications, Connection and About,
  each on its own page with a back arrow. The gear is gone from every header.
- **A hamburger drawer replaces More.** Inbox, Today, Forecast and Browse sit
  above Review, Completed, Archive, Trash, Tags, Priorities, Changed and
  Attachments in a left drawer.
- **Library defaults** now live in the library: review interval, changed
  window, completed visibility and trash retention. Changing them on the phone
  updates the Mac too.
- **Repeat rules on the phone:** None, Daily, Weekly, Monthly or Yearly, plus
  “Delete whole series” for repeating tasks.
- **Startup view:** Settings → Tasks → Open on chooses Inbox, Today, Forecast
  or Browse for a normal launch.

## 0.7.0 — 2026-11-21

- **More on the phone.** A fifth destination collects Review, Completed,
  Archive, Trash, Tags, Priorities, Changed and Attachments. Trash can restore
  or permanently delete, Review marks projects reviewed, Archive and project
  rows open their tasks, and Tags lists counts with a tag filter.
- **Settings only in More.** The gear is removed from Inbox, Today, Forecast
  and Browse; search stays everywhere.
- **Startup destination.** Settings → Tasks → **Open on** chooses Inbox, Today,
  Forecast or Browse for normal launches.
- The update check, subtasks, search, checklists and sectioned settings from
  0.6.x remain.

## 0.6.1 — 2026-11-21

- **Mac empty states return to the quiet halo.** All hand-drawn illustrations
  are gone, including the Today sun and Forecast calendar; the perspective
  titles remain.
- The phone's **Add subtask** field focuses itself when opened, so typing
  starts immediately after tapping Add subtask.
- 0.6.0 remains in this release: indented subtasks, subtask creation, the
  in-app update check, search and checklist steps.

## 0.6.0 — 2026-11-21

- **Subtasks on the phone.** Child tasks sit indented under their parent in
  every list, and the task editor shows them with completion circles plus an
  **Add subtask** field. New subtasks go through `create_task` with a parent and
  queue offline like everything else.
- **Check for updates.** Settings → About asks GitHub for the latest release,
  compares versions and offers a **Download** button for the APK. The current
  version is shown beside it.
- The phone search and checklist steps from 0.5.0 remain.

## 0.5.0 — 2026-11-21

- **Mac empty states stay focused.** Today keeps its sun and cloud doodle and
  Forecast its calendar, both theme-coloured and animated. The other
  perspectives keep the quiet halo instead of a drawing.
- The perspective empty titles are specific now: Nothing flagged, No tags yet,
  Nothing nearby, No archived projects, Nothing changed recently, No
  attachments, Trash is empty.
- **Phone search.** A search button in the header filters the cached library by
  task, note, project or tag, with results you can complete or open.
- **Checklist toggles on the phone.** Checklist steps show in the task editor
  and tick through `update_checklist_item`, optimistic and offline-safe like
  every other edit.

## 0.4.0 — 2026-11-21

- **Sectioned phone settings.** Appearance, Tasks, Notifications, Connection
  and About are separate cards: theme, accent and app icon; week start, default
  time, haptics and swipe actions; due reminders with a test; the Mac address,
  live status, last sync and unpair.
- **A quieter top bar.** The sync badge is gone. If the phone loses the Mac for
  more than a minute, a slim “Not connected to your Mac · Tap to retry” line
  appears under the heading and disappears as soon as sync succeeds.
- **Mac Dock icons.** Settings → Appearance now has six Dock icon colours.
  They swap the running app's Dock and app-switcher icon and are remembered;
  Finder keeps the bundle artwork.
- **Animated Mac empty states.** Inbox, Today, Forecast, Projects and
  Completed use the same hand-drawn check, sun, calendar and folder doodles as
  the phone. Every stroke comes from the active theme, with a slow bob, drifting
  cloud, floating page and an orbiting accent dot.
- The Mac empty Inbox now says **Inbox zero** instead of “Nothing here”.

## 0.3.1 — 2026-11-21

- **Compact due/defer picker.** The huge Material date dialog is gone. A
  bottom sheet now shows Today / Tomorrow / Next week / No date chips, a
  month calendar with today and the chosen day highlighted, and time chips
  (All day, 09:00, 13:00, 18:00, Custom) in the app's accent colour. Custom
  opens a compact keyboard time input.
- The picker follows the active theme and accent, including wallpaper and
  custom tones, instead of Android's green system accent.
- Empty-state doodles stay at their original size; the earlier reduction was
  reverted.
- The Settings sheet keeps its space below the status bar.

## 0.3.0 — 2026-11-21

- **A Todoist-shaped app.** Red accent by default, a round capture button, an
  inline **Add task** row, coloured due dates, priority flags and swipe-to-
  complete / swipe-to-trash with undo.
- **Animated empty states.** Hand-drawn check, sun, calendar and folder doodles
  draw themselves in and move gently, instead of a plain text block.
- **Themes from the Mac.** Todoist, Plain, Paper, Mist, Catppuccin, Nord,
  Gruvbox, Dracula, Tokyo Night, Vesper, Graphite and Midnight, each with light
  and dark palettes.
- **Accent modes.** Theme accent, a Material You **Wallpaper** colour on
  Android 12+, or eleven custom presets with matching light and dark tones. No
  colour picker or eyedropper.
- **Six launcher icons** (red, blue, green, orange, purple, graphite) switched
  from Settings without reinstalling. The icon is a normal-sized version of the
  Mac mark.
- **Full task editing** on the phone: title, notes, due and defer dates with
  Android pickers, project, priority, flag, estimate and tags. Saves go through
  `update_task` and `set_task_tags`, and queue offline like every other change.
- **Home-screen widget** with today and Inbox counts.
- **Due reminders** with a notification permission flow and a periodic summary
  for today and overdue tasks.
- The Settings sheet starts below the status bar instead of crowding it.

## 0.2.0 — 2026-11-21

- **Flowlist for Android.** A native Compose app that uses the Mac library
  through a new local server. Inbox, Today, Forecast, Browse, quick capture with
  the same syntax, task detail, complete, trash, undo, light and dark Plain
  themes, and an offline outbox that replays when the Mac is reachable.
- **Settings → Mobile.** Turn the server on, see its LAN and Tailscale
  addresses, pair with a six-digit code, and revoke phones. **Serve to my
  phone** installs a login agent that keeps syncing after the window closes; the
  uninstall path removes it.
- `flowlist serve` exposes `info`, `pair`, `snapshot`, `ops` and `events`.
  Operations are `flowlist-core` commands, capture text is parsed by the same
  quick-entry parser, client-generated ULIDs keep ids stable offline, and op ids
  are deduplicated so retries never double-capture.
- The phone's **Unpair this phone** revokes its token on the Mac.
- Command types are serialisable for the wire, and the smartphone icon joins the
  built-in set.
- Release assets now include a signed `Flowlist-Android-0.2.0.apk` beside the
  DMG and zip.

## 0.1.5 — 2026-11-21

- **Remove command** in Settings → About takes the `flowlist` link off `PATH`
  without touching the app or the library. `flowlist uninstall-cli` does the
  same from a terminal.
- **Uninstall** lives in a red **Danger zone** card at the end of About. The
  sheet asks whether to **Keep my library** or **Delete the app and all data**,
  then quits and removes the app, the command and macOS support files. A log is
  written to `~/Library/Logs/Flowlist-uninstall.log` if a path cannot be
  deleted.
- `flowlist uninstall --keep-data|--purge-data` mirrors the sheet, with
  `--dry-run` to print the paths and `--yes` for scripts. Without `--yes` it
  asks you to type `uninstall`.
- The uninstall sheet is keyboard-driven: Tab or the arrow keys switch the
  choice, Return confirms and Escape cancels.

## 0.1.4 — 2026-11-21

- The app can install its own command-line tool. **Settings → About → Command
  line → Install command-line tool** puts a `flowlist` link in the first
  writable folder already on `PATH`, preferring `~/.local/bin`, so no admin
  password is needed. If the folder is not on `PATH`, the card shows the exact
  `export PATH=…` line and offers a one-click `~/.zshrc` update.
- `flowlist install-cli` does the same from a terminal, for a build run from
  its full path.
- After moving the app, running the installer again repairs the link. The full
  path remains a fallback.

## 0.1.3 — 2026-11-21

- Toasts and CLI results read as statements now instead of the command names
  that produced them: **Settings updated**, **Task “X” updated**, **Completed
  task “X”**, **Moved 1 task to Trash**, **Priority set to high for 2 tasks**,
  **Checklist step “X” added**, **Project “X” added**, **Folder “X” renamed**,
  **Tag “X” deleted**, **Perspective “X” saved**, and so on.
- Proper singular/plural counts replace `task(s)` / `project(s)` throughout
  command results; deleting a project or folder with nothing inside no longer
  claims it moved zero tasks to Trash. Empty completions say **Nothing to
  change**.
- The CLI no longer prints a verb followed by an imperative label
  (“Completed Complete task …”) and uses the same messages as the toasts.

## 0.1.2 — 2026-11-21

- Boolean settings are switches now instead of tick-mark buttons: Vim mode,
  Follow system, Window controls, Due reminders, the Sidebar section/count
  rows and Import's merge choice. The labels stay put; the switch carries the
  state.
- The Reminders status follows the toggle: **Off** when reminders are off, and
  only **On**, **Denied**, **Blocked** or **Waiting** while they are on.
  Previously it showed the macOS permission ("Allowed") even beside an off
  toggle.

## 0.1.1 — 2026-11-21

### Notifications and distribution

- Due reminders on macOS: **Settings → Workflow → Reminders** schedules a
  notification for every task due in the next 30 days. Date-only tasks remind
  at 09:00 local, timestamped tasks at their exact time. The system permission
  is requested when the toggle is saved, **Send test** schedules a one-off
  banner, and the settings sheet reports the live authorization state.
  Foreground banners are shown through a `UserNotifications` delegate.
- macOS only grants notifications to a signed app. The downloadable build is
  ad-hoc signed, so the sheet reports the signing block instead of failing
  silently; package with `FLOWLIST_SIGN_IDENTITY` set to an Apple Development
  or Developer ID identity to enable reminders.
- Releases now include a drag-to-Applications **DMG** beside the updater's
  zip. `scripts/package.sh` builds both, and the signing identity is
  configurable for notifications.

### Performance and build

- The content header reuses the task groups the list already queried instead
  of running the whole view a second time to count rows.
- Search normalises the needle once per query and sorts by precomputed titles,
  removing a lowercase allocation per task and per comparison.
- `Store::open` no longer rewrites the undo history on every launch.
- Removed unused `owo-colors` and `unicode-segmentation` dependencies and the
  unused workspace `authors` field.

## 0.1.0 — 2026-11-21

### Interface

- Settings gains an **About** section (`⌘6`) with the version, the library
  folder and GitHub/wiki links, plus a real update check against the latest
  published GitHub release. A newer release downloads its zip, stages the
  `.app` beside the running bundle, swaps it in and offers **Restart
  Flowlist**; a build that is not inside a bundle falls back to the release
  page, and offline checks fail with a sentence rather than a spinner.

- Settings is now a sectioned sheet: a fixed rail (Appearance, Sidebar,
  Workflow, Nearby, Trash) beside one scrolling pane, so library-wide
  preferences have a home instead of lengthening one nameless list. `⌘1`–`⌘5`
  jump between sections, and `⇥` only walks the fields in the section on
  screen.
- The title-bar search keeps the inspector's column reserved when the
  inspector is closed, so it no longer moves between inspector states or slides
  under the **Add task** cluster.
- The macOS close, minimise and zoom buttons now sit over the header by
  default instead of being hidden, and Settings → Appearance → Window controls
  hides them again (`W` while the section is open). The header reserves the
  buttons' space while they show, the choice previews live, Cancel restores the
  state the sheet opened with, and the choice is saved with the session.
- The Settings sheet sizes itself to the window instead of a fixed height, so
  the theme grid, accent picker and window controls fit at once on a tall
  display and the pane scrolls on a short one.
- Rounded and bordered quads now snap to the device pixel grid when painted,
  so borders that used to land on half pixels render as solid one-pixel lines
  instead of two washed-out rows.
- Plain gains accent colours. Nine curated pairs each carry a light and a dark
  tone (Ink, Blue, Teal, Green, Amber, Orange, Rose, Pink, Violet), and a
  saturation/brightness square plus hue strip tune a separate custom accent
  for each appearance. Keys `1`–`9` choose a preset while Appearance is open.
  A system eyedropper samples any pixel — wallpaper included — and the result
  is luminance-clamped so accent text and primary-button labels stay at least
  4.5:1 on Plain's chrome. Accents are saved per appearance with the session;
  Cancel restores the pair the sheet opened with.
- Sidebar rows no longer show a pencil or × on hover. Rename, delete, project
  status and tag location live in the right-click menu, so names keep the full
  row width. A finished project's tick no longer paints a chip behind it.
- Visual design pass: task checkboxes stay visible on a white surface, row
  actions wait for hover, every empty view shares one illustrated halo,
  counts read as prose (`1 task`, `6 tasks`), menus, the toast and sheets fade
  in over 120 ms, and theme swatches preview each family as a miniature
  window. The status bar keeps the revision and library path in a tooltip.
- A new Plain theme family adds a pure white light palette and a near-black
  dark palette with a neutral ink accent. Its sidebar, inspector and header
  use a lighter Dia-style translucent wash, so the desktop shows through
  without tinting the glass. It is the first-run default for a library with no
  saved theme; existing saved themes are untouched.
- `⌃⌥Space` now opens a Things 3-style floating capture panel in the middle
  of the screen instead of bringing the whole window forward. The panel is a
  compact, square-cornered card that stays above normal windows when another
  app comes forward. It previews the parsed title and destination, always files
  a plain capture into the Inbox, honours an explicit project or due/defer
  date (so scheduled tasks leave the Inbox), closes on `Enter`, stays open on
  `⇧⏎`, selects the new task on `⌘⏎`, and every capture is undoable from the
  main window. `Enter` and `Esc` hand focus back to the app that was in front
  before capture.
- Inbox now means unscheduled: a task with a due or defer date leaves the
  Inbox for Today or Forecast, and clearing its dates files it back. Captures
  no longer inherit the last view's project, tag, priority or forecast date.
- The inspector's inline fields now save on `Tab` and `Enter` and move to the
  next field. A validation error keeps the current field open, and `Enter` on
  the last field finishes editing. A first-time task stays a draft until the
  whole session ends, so Delete and Complete no longer appear as soon as the
  title is saved.
- The New Project sheet has a Review every (days) field, pre-filled from the
  library default, so each project can keep its own review cadence. A project
  keeps that cadence editable afterwards: its review line in the banner and in
  the inspector is a button that opens a **Review interval** sheet, so changing
  the rhythm no longer means recreating the project. Undo restores the previous
  interval and the next review date moves with it.
- Task rows read their right-edge metadata in one fixed order — attachments,
  project, tags, estimate, due, deferred, repeat, checklist — instead of the
  order the fields happened to be added. Tags now print with a leading `#`, so
  a tag chip can never be mistaken for a project name.
- The × on an attachment row now asks first, naming the file or link and
  noting that nothing on disk is touched. Confirming unlinks it, and Undo puts
  the attachment back.
- The content header no longer repeats the last action and its Undo sentence;
  the status bar still carries both. The Settings sheet drops its
  library-wide subtitle, since the section rail already names what it holds.
- Image and PDF attachments can preview inside the window: clicking a file's
  name opens a card over the list instead of handing it to another app. Images
  fit the card; a PDF opens on its first page and steps with ←/→ or the footer
  arrows. `o` still opens the file in its default app, `Esc` closes the card,
  and links and unknown file types keep the old open behaviour.
- Search is fuzzy now: missing letters still find a task (`bcf` finds “Buy
  coffee filters”), and the fields behind a row — title, notes, checklist,
  tags, project path — all count, with the closest match first. A scope chip
  under the view title switches between this view and the whole library
  (`⌘⇧F`); library results ignore Trash and keep the project and tag metadata
  so every hit says where it lives. The empty state names the scope and how to
  switch it.
- The toast now sits in the bottom-center of the middle pane. It follows the
  pane that is actually on screen, so closing the inspector re-centres it under
  the wider list instead of leaving it left of middle.
- The **Add task** button opens the floating capture panel over the Flowlist
  window, while the global `⌃⌥Space` shortcut keeps its screen-centered spot.
  A panel opened from the button now lands where the user is looking.
- Inspector boxes read as solid objects instead of pale outlines on the glass:
  the readonly title, notes, date, project and attachment controls carry the
  same surface as the other chips, use the visible border tone, and take an
  8px radius, so their corners stay rounded rather than washed out.
- A Plain accent chosen in one appearance now carries into the other instead
  of resetting to ink: the unset side inherits the chosen hue, clamped by the
  theme builder so text and button labels stay readable there. The accent
  panel shows the inherited tone, and setting a tone for that appearance (a
  preset, the picker or the eyedropper) still overrides it. Toggling the theme
  or switching family no longer drops the accent either.
- Icon tooltips now wait three seconds instead of about five.
- Nested folder rows show just the folder's own name; indentation carries the
  path instead of printing `Parent / Child` on every row.
- Completing a repeating task now increments the last number in its title, so
  “Week 3 review” spawns “Week 4 review”. The rule's end date or count still
  stops the series.
- Deleting a repeating task asks whether to trash just this occurrence or the
  whole series. The sheet defaults to this occurrence, Tab or the arrow keys
  switch choices, and one Undo restores every trashed occurrence. `remove
  --series` and the MCP `delete_task` tool's `series` flag do the same.
- Attachments now accept http(s) website links next to file paths. The
  inspector's **Add link…** button takes a pasted address, a link row uses the
  chain icon, and a task whose attachments are all links shows a link badge
  instead of the paperclip. `--attach` and the MCP attachment fields take
  links too.
- Deleting a folder now takes its nested folders, their projects and every
  task inside to Trash as one undoable step. The sheet names the folder and
  says how many projects go with it.
- Folder rows drag like project rows. Drop one on another folder to nest it,
  or on the LIBRARY heading or empty rail space to move it back to the root;
  a folder never lands inside itself or its own subtree.
- Sidebar tags now sort by live task count, busiest first, then by name
  ignoring case. Nested tags follow the same order inside their parent, and
  completed tasks no longer lift a tag up the list.
- A folder's count now reports live projects instead of everything filed under
  it. A project whose tasks are all ticked, or a closed project, no longer
  inflates the badge; nested folders roll their live projects up.
- The title bar now has back and forward arrows beside the app name, with
  `⌘[` and `⌘]` as their keyboard routes. They walk the views the window
  came through, so Archive → project → back returns to the Archive; a new view
  drops the forward trail and deleted projects are skipped.
- Rail sections are calmer. Folders start folded on every launch, Priorities,
  Library, Saved and Tags fold from their headings, and a section's chevron
  waits at the right edge until the pointer rests on that heading.
  Perspectives stay fixed as the backbone.
- Settings has a Sidebar group that chooses which rail sections appear and
  which carry count badges. Counts default quiet outside the Library, while
  Today always keeps its day count. The choice is saved with the library.
- Archive rows can now be deleted, not just reopened. Delete asks in the usual
  confirmation sheet and runs through the normal command layer, so Undo
  restores the project and its folder group.
- The Settings sheet puts the Sidebar controls with Appearance so the new
  choices are visible without scrolling past the numeric defaults.
- Added an Archive to the sidebar, after Completed. Finished and dropped
  projects wait in collapsible folder sections with a project count, each row
  opens the project, and a Reopen button sets it back to Active through the
  ordinary command, so Undo puts it straight back. An empty Archive shows a
  calm “No finished projects” state.
- The bottom View sheet is now a settings editor instead of a row of cycling
  chips. Every condition has a labelled row that opens a menu of all its
  values, so Status, Due, Defer, Group and Sort are one click each, and the
  open menu takes `j`/`k`, arrows, Home/End, Enter and Esc. The sheet previews
  how many tasks the view matches as it is edited, Clear filters drops every
  condition, and conditions written by the CLI are named and kept on save.
- Forecast and Calendar are now one perspective. The rich month calendar
  moved above the Forecast list: one full-width month with a continuous grid,
  an accent circle for today, tinted event pills, `+N more` overflow and
  drag-to-reschedule. The duplicate Calendar perspective is gone.
- A Calendar chip in the content header folds the Forecast month grid away,
  and the grip under it resizes the calendar with the mouse (`-` / `=` from
  the keyboard). Both choices are saved with the session, and the grip is
  clamped so the day cells stay readable and the list keeps its room.
- Added a Today perspective, second in the sidebar: overdue work in its own
  group, then everything due today plus tasks deferred to today. A Hide
  overdue chip (remembered with the view) drops the overdue group, and a clear
  day shows “Nothing due today” instead of an empty list.
- Theme and appearance choices in Settings now preview on the whole window as
  soon as they are clicked, including `⌘T` inside the sheet. Cancel or Esc
  restores the theme the sheet opened with, and Save keeps the new one.
- Deleting a folder now asks first, in a sheet that names it and explains that
  its projects move up a level. The × button, the right-click menu and the
  sidebar's `⌘⌫` / `dd` all use the same confirmation, and Undo still restores
  the folder in one step.
- Task rows use the app's drawn icons for their metadata instead of emoji: a
  clock marks a future defer date, a tick marks checklist progress, and the
  repeat glyph marks a recurrence. The defer date no longer shows an
  hourglass that sat apart from every other mark.
- Clicking a sidebar folder opens it and unfolds its tree; clicking the same,
  already-open folder folds the tree again without leaving the folder view.
  The chevron keeps toggling just the tree, and `Enter` on the row matches the
  mouse.
- The Due and Defer date pickers are now a two-step surface. A proper month
  calendar replaces the cramped date-plus-stepper card, and choosing a timed
  day opens an Android-style clock face: pick the hour, it moves on to
  minutes, then Done writes the stamp. All-day picks still commit in one
  click, so a plain date stays quick. The clock shows the date being set, an
  AM/PM pair and a Back button, and every control has a tooltip and a keyboard
  route.
- The inspector no longer repeats the completion circle beside the task status;
  the Complete / Reopen button at the bottom of the panel is the one control
  for that. The status text and flag stay at the top.
- Subtasks now sit indented under their parent in every list instead of being
  hidden until the first click. The disclosure chevron collapses a whole
  branch in one click (the first click used to do nothing), and `za`, `zo` and
  `zc` fold the selected task from the keyboard.
- Projects can now be dragged between folders in the sidebar. The folder row
  or LIBRARY heading under the pointer highlights before the drop, empty
  sidebar space under the tree also returns a project to the root, and the
  list repaints immediately instead of waiting for the toast. Dropping a
  project where it already sits is a silent no-op, so it adds no undo entry.
  Every real move goes through the normal move command, so `⌘Z` restores the
  previous placement, and task drags onto projects keep working.
- Folder contents now indent under their folder, one step per nesting level,
  including projects and subfolders. An empty folder says “Empty” instead of
  the root’s “No projects yet”, and both notes line up with the names.
- Changed no longer lists completed tasks; completing a task removes it from
  the view and lowers the sidebar count, and undoing the completion puts it
  back. The filter also excludes tasks in Trash as before.
- New projects and tasks converted to projects use the Project icon by default
  again instead of the Folder glyph; Folder is still one of the ten choices.
- A sidebar project with tasks but nothing left open now shows a tick in place
  of its count. A brand-new project still shows 0, and adding any open task
  brings the number back.
- Removed the macOS close/miniaturize/zoom buttons from the app window.
- Reduced the title bar to the app name. The app badge and the
  `Flowlist / view` suffix are gone.
- Replaced the inspector toggle's sidebar glyph with an info (`i`) icon, in
  both the header and the inspector panel.
- Added a moon/sun button to the title bar, left of the inspector toggle. It
  switches light/dark in the current theme family and shows the mode it will
  switch to; `⌘T` still does the same. A manual switch turns off follow-system
  appearance so the choice sticks.
- Added a Monday-first month calendar to the top of Forecast, with previous /
  Today / next controls, an accent circle for today and event pills on dates
  that carry a task. The existing forecast buckets sit below it unchanged, and
  the header chip folds the grid away when the list needs the whole pane.
- Rename sheets now open with the whole name selected: typing replaces it,
  one Backspace or Delete clears it, and arrows or a click place the caret.
  Sheet text fields also accept a click or drag, so a range of a name can be
  selected with the mouse instead of erased key by key.
- The inspector's Due and Defer fields gained a calendar icon that opens a
  small month picker right under the field. Clicking a day applies it as a
  normal undoable edit, keeping an existing time of day; Today, Clear, month
  arrows and keyboard navigation are included, and Esc closes without change.
- The picker's Time row adds or removes a time of day. All day toggles between
  a date-only stamp and the library's default due/defer time, −/+ step 15
  minutes (an hour with `{`/`}`), and the chosen time is written with the day.
- Header and status chrome now use a flat glass tint; only the sidebar and
  inspector keep the vertical tint wash. This removes the unintended
  left-to-right gradient ramp under the title bar.
- Added `docs/backlog.md` and `AGENTS.md` so outstanding work survives a
  context reset; `flowlist.md` is now the durable pending list.
- Deleting tasks now selects the row that took their place, so `j`/`k` keep
  working and the list never loses the cursor.
- Clicking a Forecast calendar date filters the list to that day. Click again
  or press `\` to clear; `[`/`]` step a day and `{`/`}` step a month.
- Project types: parallel, sequential and single-action projects, chosen in the
  New Project sheet and cycled from the project banner.
- Sort and Group header chips now open a list of every option instead of
  cycling blindly. `Esc` closes the menu.
- The Priority header chip now opens the same kind of menu (All priorities,
  High, Medium, Low) instead of cycling through the filter.
- Project creation sheets no longer show a second close control in the top
  corner; their footer already has Cancel.
- Tooltips wait about five seconds before appearing, and clicking a button
  keeps its tooltip quiet until the pointer leaves the button. Hovering again
  later restores the normal behaviour.
- The **Add task** action moved to the title bar as a labelled plus button,
  so it is available in every view and the content header keeps its full width
  for the view controls.
- The content header keeps only the view title and the Sort/Group/Filter chips
  on its first row, with the subtitle on its own row so it is never squeezed.
  If the view name cannot fit beside the controls it is left out entirely
  instead of being clipped to a few letters.
- Search moved into the title bar as a full field at every window width, so it
  no longer competes with the view title and chips. The field is centered on
  the content pane, not on the whole window, and a × at its right edge clears
  the query in one click. The title-bar theme (palette) icon is gone; `⌘T` and
  `⇧⌘T` / `⌥⌘T` still change the theme.
- Sidebar rows spend their width on the name until the pointer arrives. Only
  the hovered row makes room for its pencil and ×, and its long name then fades
  out at the edge instead of adding an ellipsis. At rest long project, folder,
  tag, perspective and review-card names still end with an ellipsis (…). The
  inspector shows the project name in full and wraps it instead of cutting it.
- Dropdown menus and sheets now block the window underneath, so clicking a
  Sort, Group or Priority option no longer also selects or highlights the row
  beneath the menu, and a dimmed sheet backdrop swallows clicks instead of
  passing them to the list.
- The Priority filter is remembered per view. Opening a sidebar priority row
  no longer leaves that filter on the next perspective, and each perspective
  keeps its own choice.
- Sidebar Forecast, Flagged and Completed badges, plus the status bar's open
  and available totals, no longer count tasks sitting in Trash.
- Project rows in the Library use the same small boxed count as perspectives,
  and their names line up with the folder icon and count instead of sitting a
  couple of pixels low.
- Sort and Group menus open directly under their chip, stay above the task
  list, and close on any click outside. The outside-click scrim now closes on
  mouse-up, so the menu item under the pointer still receives its click.
- Right-clicking any sidebar row opens a menu for that row: open, rename,
  delete, new child, project status, tag location and Empty Trash where they
  apply. The menu opens under the pointer and `Esc` closes it.
- Sidebar rows reveal their edit and delete buttons on hover, including the
  selected row, which no longer pins them visible. Project rows gained a
  delete button.
- Project deletion is an undoable command in the GUI, CLI (`project delete`)
  and MCP (`delete_project`). Deleting a project moves its tasks to Trash and
  detaches the project from its folder. The sidebar's × now opens a Delete
  project sheet that names the project, so one click cannot remove it by
  accident; Enter confirms and Esc cancels.
- Typing an unknown `Folder / Project` path in the task inspector now asks
  before creating the project. Dismissing the prompt leaves the task alone.
- The inspector's project picker only opens once you start typing, instead of
  covering the panel with every project as soon as the field is focused.
- New projects default to the folder icon instead of the generic project
  glyph.
- Added a **First Available** header toggle that hides blocked rows.
- Sort, grouping, completed visibility and availability are remembered per
  perspective.
- Task inspector: parallel/sequential chips for action groups, and an
  attachments section that can add file paths, open them, and remove them.
- **Convert to project** turns a task and its top-level subtasks into a project
  in one undoable step, then opens that project. Attachments move to the
  project and leftover checklist items become tasks, so nothing is lost.
- Project icons: a fixed set of ten glyphs, chosen in New Project and the
  project inspector, rendered in the sidebar and dashboard. The grid stays
  folded behind the current icon until it is clicked.
- Project and tag fields complete paths as you type: prefix matches first,
  `↑`/`↓` to highlight, `Tab` or click to accept. Spaces around `/` are
  optional.
- The system-wide `⌃⌥Space` shortcut opens capture while another app is in
  front. Flowlist activates, switches to Inbox if the current view has no
  capture bar, and selects any previous capture text so typing starts fresh.
- Follow-system appearance keeps the chosen theme family and takes light/dark
  from macOS.
- Changing the theme now shows a toast naming the family and appearance.
- Notes wrap in the inspector instead of clipping.
- Review uses a coffee-cup icon, and Mark reviewed sits on the title row.
- Dark palettes raise selection, hover and accent-soft contrast, and the glass
  chrome is a little more opaque so text stays readable.
- Project rows show open-task counts; folder rows count open projects.
- Long view headings no longer wrap into the toolbar.

### Library

- **Trash** is a ninth built-in perspective. Delete moves tasks and their
  subtrees to Trash; `r` restores them, Delete forever purges them, and
  Settings controls auto-delete after a retention period (`0` keeps forever).
  The automatic sweep runs as a normal command and is undoable. The Trash
  header has a **Select all** button, and `⌘A` selects every trashed task, so
  the whole bin can be restored or purged in one action.
- **Priorities**: optional High / Medium / Low, shown as a coloured mark on
  rows, editable in the inspector, filterable from every view, and available
  as sidebar rows plus CLI (`--priority`) and MCP fields.
- **Project kinds** are persisted alongside the legacy `sequential` flag, so
  older libraries keep their sequential behaviour.
- **Changed** is the tenth built-in perspective, reached with `0`. Introspection
  and editing times feed a configurable `changed_days` window.
- **Attachments** store referenced file paths on tasks and projects; files are
  never copied into the library. Rows show a paperclip count and the
  **Attachments** perspective lists every open task that has one. CLI:
  `--attach`, `--clear-attachments` and `list --attachments`.
- CLI: new `trash` and `convert` commands, `--kind`/`--icon` on projects and
  `--changed`/`--attachments` on `list`. MCP: trash tools,
  `convert_task_to_project`, attachment fields and `icon` on project tools,
  and an `attachments` view for `list_tasks`.
- Added a task-manager comparison and the next adopted gaps to
  `docs/backlog.md`.

## 0.1.0-beta.1 — 2026-11-13

First complete release of Flowlist: one Rust library and three front ends.

### Highlights

- **Native desktop app** built with GPUI 0.2.2 — no web view, Tauri or Electron.
- **Keyboard-first interaction**: app-wide vim keybindings, macOS shortcuts,
  delayed tooltips on icon-only controls and a searchable help card.
- **Scriptable CLI** and **MCP server** over the same domain layer, so every
  front end shares validation, repeat rules, search and undo.
- **One local library**: atomic JSON writes, backups, a cross-process lock and
  automatic reload when another process writes.
- **Full task model**: projects, nested folders, tags, due/defer dates,
  estimates, repeat rules, flags, subtasks, checklists with progress, review
  scheduling and nearby tasks based on tag locations.
- **Saved perspectives** with a rich editor for common conditions, grouping and
  sort; unknown filter nodes survive editing unchanged.
- **Everything is undoable**: every GUI, CLI and MCP mutation goes through one
  command engine and one history.

### Interface

- **Ten theme families** — Paper, Mist, Catppuccin, Nord, Gruvbox, Dracula,
  Tokyo Night, Vesper, Graphite and Midnight — each with light and dark
  palettes, chosen in Settings or with ⌘T / ⇧⌘T / ⌥⌘T.
- **Glass chrome**: the sidebar, inspector, header and status bar use GPUI's
  native macOS blur backdrop, while the task list keeps an opaque reading
  surface for contrast. The blur view is pinned to the palette's light/dark
  appearance and the `.underWindowBackground` material, and each pane runs a
  subtle tint gradient instead of a flat fill.
- **Full-height window**: the native titlebar is hidden, traffic lights sit in
  the Flowlist header, and the header doubles as the drag region.
- **One icon language**: embedded SVG icons share a 24px grid, 1.75px stroke
  and round joins, replacing inconsistent text glyphs throughout the app.

### Packaging

```sh
scripts/package.sh
open dist/Flowlist.app
```

`dist/Flowlist.app` bundles the GUI, CLI and MCP server in one executable. The
bundle is ad-hoc signed for local use; it is not Developer ID signed or
notarized. See [docs/packaging.md](docs/packaging.md).

### Quality checks

- `cargo test --workspace` covers the domain, undo/redo, import/export, query
  behaviour, the CLI and cross-process reload with the real binary.
- `scripts/perf-check.sh` exercises 1k, 10k and 30k task libraries; a
  10,000-task library answers a perspective in under 200 ms including process
  start-up. See [docs/performance.md](docs/performance.md).
- `scripts/gui-smoke.sh` drives the mouse-only New project and perspective
  pencil flows with real CoreGraphics events. See
  [docs/gui-automation.md](docs/gui-automation.md).
- Accessibility: every icon-only control has a tooltip and a keyboard route
  where one is possible. GPUI 0.2.2 does not expose an accessibility tree, so
  VoiceOver cannot inspect internal controls yet; that limitation is documented
  in [docs/accessibility.md](docs/accessibility.md).

### Known limitations

- Ad-hoc signing only; distribution to other Macs needs a Developer ID
  certificate and notarization.
- The GUI smoke test depends on the current sidebar geometry and fails loudly
  if that geometry changes.
- Tag removal from the inspector has no dedicated keyboard shortcut yet; use
  the inspector Tag field.
- GPUI 0.2.2 lacks an accessibility tree, so screen-reader support is limited to
  the window title; the CLI and MCP server are the fallback for assistive
  workflows.

### Documentation

[README](README.md) · [CLI](docs/cli.md) · [MCP](docs/mcp.md) ·
[Architecture](docs/architecture.md) · [Features](docs/features.md) ·
[Checklists](docs/checklists.md) · [Packaging](docs/packaging.md) ·
[Performance](docs/performance.md) · [Accessibility](docs/accessibility.md) ·
[GUI smoke test](docs/gui-automation.md)
