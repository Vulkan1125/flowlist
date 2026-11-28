# Backlog

This file is the durable source of truth for outstanding work. The user-facing
pending list now lives in `flowlist.md`; this file records the engineering
detail behind it. If a conversation mentions "the pending things", it means the
open items here and in `flowlist.md`.

Status legend: `[ ]` open, `[~]` in progress, `[?]` needs a decision, `[x]`
landed and verified.

> `flowlist.md` supersedes the unreadable Things 3 database. Keep both files in
> sync when work lands; do not treat silence as "done".

## Roadmap

The phases are worked in order. Detail for each item lives in the sections
below; this list is the current map. Platform/ecosystem gaps (cloud sync, web,
Watch/Vision, widgets, Siri/Shortcuts/Spotlight, AppleScript/Omni Automation,
multiple windows, localisation, VoiceOver and alternate icon artwork) are
intentionally outside Phase 5.

### Phase 1 — Friction fixes

- [x] Inspector fields save on `Tab` and `Enter`, then move to the next field;
      validation failures keep the editor open, and `Enter` on the last field
      ends the session. A first-time task keeps its draft state (and hides the
      Delete/Complete actions) until the whole field session ends.
- [x] Per-project review interval is editable in the New Project sheet, with
      the library default pre-filled.
- [x] Nested folders verified end to end: `folder add 'Work/Client/BigCo'`,
      `project add --folder`, the sidebar tree, indentation, and folder counts.
- [x] Toast self-dismissal verified live: it retires on its own, on click, and
      when a newer toast replaces it.
- [x] Tooltips now wait three seconds instead of about five.

### Phase 2 — Safety and containment

- [x] **Attachment website links** — http(s) attachments sit beside file paths,
      with the inspector's Add link… sheet, a link-aware row badge, the shared
      Attachments view, and `--attach` / MCP fields accepting links.
- [x] **Folder delete cascade** — deleting a folder takes its nested folders,
      their projects and tasks to Trash, after a sheet that names the folder
      and counts its projects, as one undoable step.
- [x] **Folder dragging** — folder rows drag like project rows: drop on a
      folder to nest, on LIBRARY or the empty rail to return to the root, with
      self/descendant drops rejected and one undoable move.

### Phase 3 — Recurring task series

- [x] **Auto-incrementing repeat titles** — the last number in the title
      increments on completion, with the rule's end date or count stopping the
      series; a title without a number is copied unchanged.
- [x] **Series-aware deletion** — deleting a recurring task asks whether to
      remove this occurrence or every occurrence in the series. The default is
      this occurrence, Tab or the arrow keys switch the choice, and one Undo
      restores the whole chain. `remove --series` and MCP `delete_task` with
      `series: true` expose the same behaviour.

### Phase 4 — Larger product bets

- [x] **Things 3-style quick capture popup** — `⌃⌥Space` opens a compact
      620×150 floating panel in the middle of the screen. The main window keeps
      its view, selection and scroll. The panel previews the parsed title and
      destination chips, files through `Store::execute`, reuses itself with the
      old text selected, and supports `Enter`, `⇧⏎`, `⌘⏎` and `Esc`. A plain
      capture always goes to the Inbox; an explicit project or due/defer date
      sends it elsewhere, and scheduled tasks leave the Inbox. The panel floats
      above normal windows when another app is in front, and `Enter`/`Esc` hand
      focus back to the previous app. Covered by
      `scripts/gui/phase4-probe.sh` and `scripts/gui/quick-capture-probe.sh`.
- [x] **Forecast calendar** — the rich month grid lives inside Forecast:
      drag-to-reschedule, keyboard day/month steps, a fold-away header chip
      and a mouse-resizable height. There is no separate Calendar perspective.
      Covered by `scripts/gui/forecast-probe.sh`.
- [x] **Plain theme** — a neutral white/near-black family and the first-run
      default. Both modes meet the same contrast contract as the other
      families. Plain also carries nine accent preset pairs, a custom
      saturation/brightness/hue picker per appearance and a system eyedropper;
      custom tones are luminance-clamped to keep the contrast contract.
- [x] **Visual design pass** — hierarchy, spacing, motion, hover/focus and
      the glass edges, verified live in every theme. Hover-only row actions,
      one illustrated empty state, prose counts, 120 ms overlay fades,
      miniature-window theme swatches and a quieter status bar.

### Phase 5 — Advanced view parity (non-platform)

Gap audit against the reference task manager (November 2026). Platform/ecosystem
integrations are tracked separately and are not part of this phase.

- [x] **Quick Open** — `⌘O` opens one navigation palette for perspectives,
      folders, projects, tags and priorities. `↑`/`↓` move, `Return` opens,
      `Esc` closes, and typing filters without opening a second search UI.
- [x] **Planned dates** — a third date field beside Due and Defer that never
      changes availability. Inspector calendar field, `planned:` / `plan:`
      quick capture, Forecast Planned Today and planned calendar days, Today
      including planned-today tasks, a Planned dates filter in the saved-view
      editor, Planned sorting and grouping, CLI `--planned`, MCP
      `planned` / `planned_at`, Android display, and distinct
      marks for Defer, Planned and Due.
- [x] **Past plans and Running** — a planned date that slips stays in
      Forecast's Past section and in Today's Past plans group; one Hide past
      chip folds both carried-over groups. Forecast treats Due, Defer and
      Planned independently, calendar clicks include planned days, and work
      more than 30 days out folds under **Later**. Saved views
      can be pinned to the top of SAVED and given a custom rail icon. A shipped,
      pinned **Running** view lists started work that is not due yet and can be
      edited or deleted. Saved views carry an editable description, and
      projects and views share a tabbed, searchable icon card for named icons
      and emoji. Sidebar project and folder menus have Edit sheets.
- [x] **Planned inheritance** — projects carry a planned date; actions inherit
      it through parent actions, with task/parent overrides. Today, Forecast,
      filters, grouping, sorting, calendar dots, the project inspector and the
      Edit Project sheet all use the effective date. Availability is unchanged.
- [x] **Forecast count option** — a Workflow switch counts only due items in
      the Forecast badge.
- [x] **Past and Future calendar tiles** — the collapsed Forecast strip is a
      piano-key timeline with fixed Past/Future end tiles and a paged week.
- [x] **Default wording** — Sort and Group say “Default” for the view's
      natural order/grouping instead of “View”.
- [x] **Project date constraints** — Defer, Planned and Due in the project
      sheets; actions inherit defer (start no earlier) and due (finish no
      later), effective dates drive Today/Forecast/overdue/filters/groups/sorts,
      and planned stays constraint-free.
- [x] **Strict date validation** — contradictory task dates are rejected on
      save, moves show an Adjust dates sheet, and legacy libraries are clamped
      once on open.
- [x] **Inherited date labels** — the task inspector says “Defer/Planned/Due
      with project” or “with parent” when the value is inherited.
- [x] **Date warning on every path** — inspector project assignment and
      project date edits use the Adjust dates sheet; direct date edits report
      an inline error under the field or inside the edit sheet.
- [x] **Projects in Forecast** — project rows on their due/defer/planned dates,
      above the day's task rows, with status, progress, calendar pills and
      Past/Future support.
- [x] **Full notification model** — per-type Defer/Planned/Latest Start/Due
      defaults, per-item remove/restore, relative alerts, absolute custom
      notifications, project/action-group schedules and CLI/MCP access. Per-date
      floating/fixed time zones remain a date-model gap.
- [x] **Batch editing** — select several tasks and edit tags, project,
      defer/planned/due, priority, flag and estimate in the inspector as one
      undo step. Strict dates make the batch atomic.
- [x] **Focus mode** — scope every perspective to one or more projects/folders.
      Cmd-click selects rows for general multi-actions and focus; a banner names
      the focus, the eye clears/restores it, and Inbox/Trash stay in the rail.
- [x] **Forecast tag and flagged contents** — a designated tag pulls undated
      actions into Forecast, flagged undated actions can be shown, and on-hold
      project deadlines already appear. Absolute custom notifications remain
      part of the alert editor rather than Forecast dates.
- [x] **Calendar events in Forecast** — read upcoming events from macOS
      Calendar and interleave them with due, planned and deferred tasks.
      Settings → Calendar enables it, lists every calendar with a show/hide
      switch, and keeps access read-only and per-device.
- [x] **Nested perspective rules** — the shared contents tree: nested
      All / Any / None groups, the full rule catalogue, date ranges, tag and
      project status, comments, turning rules off, drag reordering, and the
      same evaluator on the Mac and the phone.
- [x] **Project-level custom perspectives** — matching projects can group by
      folder, project, review date or a project-level date (due, defer,
      planned, added, changed), with project sorting by name, dates or library
      order. Each project keeps its header and its matching actions nest
      underneath. The shared Presentation section and `perspective add`
      expose the two controls.
- [ ] **Perspective favourites and navigation** — favourites bar, reorder,
      custom icons/colours, copy-as-link and a per-perspective shortcut.
- [ ] **Configurable row layout** — per-perspective density, columns and
      visible fields, without fragmenting the shared visual language.
- [ ] **Rich-text notes** — bold, italics, lists, links and inline images
      alongside the existing plain-text fallback.
- [ ] **Advanced repeat schedules** — minute/hour intervals, multiple
      weekdays, nth weekday of a month, last weekend day and other
      calendar-pattern repeats. The inspector’s Repeat section already covers
      the day/week/month/year model and the due/defer and assigned/completion
      choices.
- [ ] **Capture integrations** — Mail Drop, Mac clippings, Mac share, and
      OmniOutliner or plain-text outline paste with hierarchy preserved.
- [ ] **Nearby map** — map view alongside the distance-sorted list.
- [ ] **Review calendar** — a calendar of upcoming review dates beside the
      review list.
- [ ] **Archive and resolved-item cleanup** — a true compacted archive
      database, a clean-up policy for completed rows, and warnings before
      deleting containers that hide completed children.

### Phase 6 — Android parity and reminder polish

- [x] **Reminder staleness on an idle phone** — the alarm and the reminder
      activity now check the shared dismissal list and the cached task before
      taking the screen, so a Mac complete or snooze can no longer leave an
      already-handled alarm to fire.
- [x] **Overlay permission onboarding** — turning on Full-screen reminders now
      opens **Display over other apps** when the permission is missing.
- [x] **Reminder notification actions** — the alarm card now carries Dismiss
      and Snooze buttons. Snooze uses the saved length, reschedules the alarm,
      dismisses the other device and closes the reminder.
- [x] **Simultaneous reminders** — tasks due together merge into one card
      that says how many are due, with a read-only **View all** list, **Open
      Flowlist** for a Due now page inside the app, and one **Dismiss all**
      button. Later alarms stay silent and there is no per-task marking on the
      batch card.
- [x] **Media pause stays FSR-only** — the Pause media switch applies to
      full-screen reminders only; regular due summaries never take audio focus.
      There is no per-reminder override.
- [ ] **Android docs refresh** — `android/README.md` still describes the old
      week-strip reminder and calls the default theme Todoist; update it for the
      overlay, media pause, snooze and cross-device sync.
- [x] **Project completion mode** — the Android project editor now toggles
      **Complete with last action**. The shared command layer applies it, so
      the phone and Mac agree.
- [x] **Project archive controls** — the Android project editor has an
      **Archived** switch, the row action now archives instead of marking the
      project Done, and Settings → Library has the Mac's **Archive closed
      projects** choice.
- [ ] **Stalled and On Hold projects** — dim and reject completion for On Hold
      projects, mark stalled projects in the drawer, and reveal a project's
      completed roots behind a Completed (n) row.
- [ ] **Sequential blocking in the Android task list** — the task editor dims
      children after the first incomplete one and the command layer refuses the
      tick, but the main list still looks normal and only reports the refusal in
      a snackbar. Dim and lock blocked rows there too, with the blocker on the
      row.
- [ ] **Single-action list behaviour on Android** — tasks in a Single action
      project still show the Parallel / Sequential / One-off chips and subtask
      controls; hide them to match the Mac inspector, where single-action lists
      have no steps.
- [ ] **Focus mode** — scope the phone's perspectives and search to chosen
      projects and folders, with the same clear-focus banner as the Mac.
- [ ] **Calendar events in Forecast** — read the phone's own calendars (with
      permission) under the month grid, matching the Mac's EventKit events.
- [ ] **Project rows in saved views** — render matching projects as their own
      rows with nested actions, as the Mac does, instead of listing tasks only.
- [ ] **Saved-view Simple and Comprehensive tabs** — the Mac opens a new view
      on a plain checkbox and pop-up editor and switches to the rule tree on
      Comprehensive; the Android editor only offers the full form. Add both
      tabs and remember the last one used per view.
- [x] **Planned dates** — the task editor now edits the Mac's third date
      alongside due and defer, the row shows it as **Plan …**, and the value
      round-trips through `update_task`.
- [x] **Full repeat editor** — both editors now share the interval, fixed
      versus after-completion, due-again versus defer-another, the weekday
      strip for day-based rules, and the end as never, a date or a count.
      The completed count and weekdays are preserved while editing, and one
      summary line folds in the done count and the projected last date for
      fixed schedules.
- [ ] **Per-item notifications** — the Mac's task and project notification
      rules (defer, planned, latest start, due; relative and absolute) are
      missing on the phone, which only has the global due summary and
      full-screen reminders.
- [x] **Project dates, tags and dashboard** — the Android project editor now
      edits due/defer/planned dates and project tags, and the project page
      shows status, type, review, progress, dates and notes.
- [x] **Project attachments** — the Android project editor lists project
      files and links, adds files through the upload path used by the task
      editor, adds named links, and removes either before saving.
- [x] **Project pages** — projects open as full pages with a back arrow, a
      status/review/progress banner, in-project search, sort and grouping,
      **Completed** and **Available** filters, and project-aware capture.
- [x] **Folder cells** — tapping anywhere on a folder row folds or opens it,
      not only its chevron.
- [ ] **Convert task to project** — add the Mac's one-step, undoable convert.
- [ ] **Batch edit fields** — phone select mode only completes, trashes or
      purges. Add the Mac's project, tags, defer/planned/due, priority, flag
      and estimate edits as one undo step.
- [ ] **Drag into projects, tags and sibling positions** — phone drag only
      nests or moves a task to the project root. Match the Mac's drops onto
      projects, tags, parents and between siblings in manual order.
- [x] **Built-in view filters** — the header filter menu adds **Available
      only** and All/High/Medium/Low priority filters to Inbox, Today,
      Forecast, the task-based More lists, saved views and project pages.
- [x] **Grouping for built-in views** — Inbox, Today and the task-based More
      lists group by project, folder, tag, due, defer, priority, flag,
      estimate, added or completed. Project, group, priority and availability
      choices are remembered per view.
- [ ] **Forecast depth** — add the Mac's due-only mode, project rows on their
      dates, tag and flagged undated sections, and dragging month-grid chips
      to reschedule.
- [x] **Saved-view options** — the Android editor edits a view's pin, glyph
      (named icon or emoji) and description. Pinned views sit above **Saved**,
      use the pin glyph by default, and appear in the tablet rail. Settings →
      Library → **Hide empty views** hides unpinned views with no matches.
- [ ] **Drawer visibility** — the phone can reorder the drawer and pick four
      bottom-bar slots but cannot hide rows or counts. Add the Mac's Sidebar
      matrix controls.
- [ ] **Task row icons** — phone rows show due/planned text and a flag but no
      defer clock or repeat glyph. Match the Mac's drawn row metadata.
- [ ] **Separate light and dark backgrounds** — the Mac keeps a light and a
      dark wallpaper; the phone has one custom background.
- [ ] **Import and export** — add JSON/TaskPaper import and TaskPaper export
      to the phone, matching the Mac's import sheet and export shortcut.
- [?] **Play Store path** — `SYSTEM_ALERT_WINDOW` and a `specialUse` foreground
      service need a justification if the phone app is ever listed on Play.
      Sideloaded builds are unaffected.

## Interaction polish

- [x] **Toast self-dismissal** — verified in a running window: a toast retires
      on its own after about 2.6 seconds, on click, and when a newer toast
      replaces it. The replacement restarts the timer.
- [x] **Inspector Tab / Enter** — `Tab` and `Enter` save the current field and
      move to the next one. Validation keeps the field open on error, and
      `Enter` on the last field finishes the session. Fields no longer save the
      whole task and kick the focus back to Content.
- [x] **Per-project review interval** — the New Project sheet carries a Review
      every (days) field, pre-filled from `Settings.default_review_days`, and
      `CreateProject` stores it per project. The inspector already shows each
      project's cadence.
- [x] **Nested folders** — `folder add 'Work/Client/BigCo'` and
      `project add --folder 'Work/Client/BigCo'` round-trip through the CLI,
      and the GUI indents folders and projects one step per level. Verified in
      a live window at three levels.
- [x] **Project completion tick** — a sidebar project with tasks but no open
      ones shows a tick instead of 0, and any open task restores the count.
      A new project keeps its 0 until it has a task.
- [x] **Rename text selection** — rename sheets select the whole name on
      open, so typing replaces it and one key clears it. Sheet fields accept a
      click for the caret and a drag for a selection, with the layout mapped
      from a transparent copy of the field text.
- [x] **Focus after delete** — deleting selected task(s) selects the row that
      took their place, or the previous row at the end of the list. Works in
      Trash too, where delete is permanent.

## Data model and features

- [x] **Trash** — tasks are soft-deleted into a ninth built-in perspective,
      restored with `r`/Restore, purged with Delete forever, and auto-emptied
      after `trash_retention_days` (0 keeps forever). See Settings.
- [x] **Priorities** — optional High / Medium / Low on tasks, with a coloured
      row mark, inspector chips, a global priority filter in every view,
      sidebar rows, and CLI/MCP support.
- [x] **Project types** — parallel, sequential and single-action
      projects, chosen in the New Project sheet and cycled from the project
      banner. Legacy `sequential` files still map to the sequential kind.
- [x] **Container-type polish** — task subtrees now carry a
      parallel/sequential type, exposed as chips in the inspector.
- [x] **Inbox means unscheduled** — a root task with no project and no due or
      defer date is in the Inbox. Adding a date moves it to Today or Forecast,
      clearing the dates files it back, and the sidebar badge and `stats`
      counter both treat a scheduled root as out of the Inbox. Quick captures
      no longer inherit the last view's project, tag, priority or forecast
      date.
- [x] **Forecast date selection** — clicking a calendar date shows that day's
      due/deferred tasks; click again or press `\` to clear. `[`/`]` step a day,
      `{`/`}` step a month.
- [x] **Due/Defer date pickers** — the inspector's date fields carry a
      calendar icon that opens a month grid. A picked day is applied through
      `UpdateTask`; Today, Clear and keyboard navigation are included.
- [x] **Time picker** — the date popover has a Time row: All day toggles a
      time on or off at the library default, −/+ step 15 minutes, and `A`,
      `[`/`]`, `{`/`}` do the same from the keyboard. A picked day writes the
      staged time, or a date-only stamp when the row is all-day.
- [x] **Forecast calendar display** — one full-width month, no month-count or
      width choices. The content header carries a Calendar chip that folds the
      grid away, and the bottom grip (or `-` / `=`) resizes it; the fold and
      height live in the session, so the layout survives a relaunch.
- [x] **Project drag into folders** — project rows drag onto folder rows, or
      onto the LIBRARY heading or empty space under the tree to return to the
      root. The hovered folder or heading highlights during the drag, and a
      post-drop background tick forces the repaint that mouse-up misses. A
      drop onto the current folder or root is a no-op. Drops use
      `Command::MoveProject`, so they are undoable and shared with the CLI.
      The GUI test state exposes every sidebar row's painted box, its name
      start, the empty-folder note and the live drag target for the probe.
- [x] **Nested sidebar indentation** — folder contents step right once per
      level, including projects and subfolders, and an empty folder's note
      says “Empty” aligned with the names instead of reading like a heading.
- [x] **Changed hides completed tasks** — the query and the sidebar badge both
      filter out completed tasks; undo brings the row and the count back.
- [x] **Android client** — a Compose remote for the Mac library with an offline
      outbox, full task editing, themes, a home-screen widget and due reminders.
      See `android/README.md`.
- [x] **QR pairing** — Settings → Mobile draws a QR carrying every Mac address
      and the live pairing code; the phone scans it and tries each address in
      order, so a LAN address can fall through to Tailscale.
- [x] **Android share-sheet capture** — text opens the capture sheet
      prefilled; a shared URL or file asks whether to start a new task or join
      an existing one, then attaches the link or upload. Link rows show the
      site's favicon.
- [x] **Android row touch target** — the whole task cell shows the press
      ripple; the circle, chevron and selection controls keep their actions.
- [x] **Android project editing** — type, notes and review interval join name,
      folder and status, matching the Mac inspector and New Project sheet.
- [x] **Android attachment previews** — images and PDFs open in a card; PDFs
      page with previous/next, and Open externally remains.
- [x] **Mac link favicons** — link rows use the host's own favicon, cached per
      session, with the chain icon as fallback.
- [x] **Android undo/redo** — the drawer shows the next action for Undo and
      Redo, backed by the Mac's shared history through `/api/v1/undo` and
      `/redo`.
- [x] **Android project icons** — the Mac's ten-icon set is selectable in the
      project editor and painted on Browse rows.
- [x] **Android tag status** — Available, Blocked and On Hold are editable in
      the tag sheet, inherit to child tags, and mark affected tasks as blocked.
- [x] **Perspective estimate filter and header sort** — Mac and Android
      perspective editors accept minimum and maximum estimate minutes. Every
      Android task perspective also has a header sort menu covering Manual,
      Due, Defer, Name, Priority, Flagged, Estimate, Created and Completed.
      Date sorts show Today/Yesterday/date section headings.
- [x] **Android attachment and favicon parity** — Attachments only lists open
      tasks. Date sorts show section headings. Link favicons try the common
      paths with a browser user agent and decode PNG entries inside ICO files,
      which Android's BitmapFactory cannot read directly.
- [x] **Android multi-select perspectives** — Projects, tags, priorities and
      due windows can each be OR-ed as multi-select sets.
- [x] **Priority section headings** — sorting by priority groups Android task
      lists under High/Medium/Low/No priority headings.
- [x] **Project completion mode** — a project can complete with its last open
      task or stay active for a manual Done; single-action lists stay manual.
      The inspector, CLI, MCP and project JSON carry the setting.
- [x] **Focus-aware rail counts** — Inbox, priority, tag, folder, project and
      saved-view counts respect the active focus, and empty tags dim.
- [x] **Stalled projects and completed work** — an active project with no open
      tasks shows no rail badge; the project pane says “All caught up — no
      active tasks.” Completed roots can be revealed with a Completed (n) row
      under their own heading.
- [x] **On Hold lockout** — an On Hold project shows a pause marker, dims its
      tasks and subtasks, disables their checkboxes and rejects completion
      until the project becomes active.
- [x] **Project archive** — a separate archive flag with an inspector icon and
      right-click actions. Folder menus archive every project inside them.
      Settings → Workflow controls automatic archiving of Done and Dropped
      projects; when off they stay visible with a tick or ×. CLI
      `project archive` / `folder archive` and MCP `archive_projects` /
      `archive_folder` / `update_project.archived` share the same flag.
- [x] **Currency project icons** — dollar, rupee, euro, pound, yen, won, ruble,
      bitcoin, franc and lira, shared with the phone picker.
- [x] **Mac completion clears the inspector** — completing a task clears the
      selection and closes the field editor.
- [x] **Mac multi-select perspective filters** — projects, tags, due and defer
      values toggle with checkboxes, and estimate min/max sit side by side.
- [x] **Checkbox multi-select on Android** — perspective multi-select menus use
      checkboxes rather than single check marks.
- [x] **Independent light and dark themes on Android** — Appearance shows a
      light-theme row and a dark-theme row, each with its own family. Tomato is
      the new name for the former Todoist palette, and old saves that still say
      `todoist` load as Tomato.
- [x] **Counts on every Android drawer row** — built-in perspectives and saved
      views show their task count when it is non-zero; zero counts stay hidden.
      Tags and Nearby count tasks, not the tags themselves, and an empty saved
      view leaves the slot blank.
- [x] **Mac saved perspective badges** — the rail shows a saved view's open
      count only when it is non-zero; empty views leave the slot blank.
- [x] **Mac sidebar matrix** — Settings → Sidebar is a collapsible table of
      every perspective, priority, folder, project, saved view and tag, with
      side-by-side Hide and Hide count boxes and item names indented under each
      heading. Every heading starts collapsed; opening one closes the rest. A
      tick means hidden, hiding a row greys out its count box, and the hidden
      name is struck through. Inbox, Today and Trash have no Hide box at all.
      Count choices materialise the old quiet defaults on the first visit, so
      upgrading does not flood the rail with numbers.
- [x] **Resizable Settings sheet** — the sheet opens at a fixed size, can be
      resized from a bottom-right grip, and shows a Zed-style thin scrollbar
      when the body no longer fits. The thumb tracks the pointer exactly, the
      track jumps to a click, and the corner grip uses the correct diagonal
      resize cursor.
- [x] **Non-Tomato warning yellow** — Plain, Paper, Mist, Catppuccin, Nord,
      Gruvbox, Dracula, Tokyo Night, Vesper, Graphite and Midnight use a clear
      gold (#C69214) in light mode and bright yellow (#FACC15) in dark mode for
      warnings and medium priority. Tomato keeps its orange.
- [x] **Non-Tomato danger red** — the same families use a vivid red (#DC4C3E
      light, #FF7066 dark) for high priority and destructive actions instead of
      their muted per-family reds.
- [x] **Mac priority signals** — high and medium task circles use the same
      vivid red/yellow pair across every Mac theme, and priority rings are 2 px
      instead of 1 px so the colour reads more clearly.
- [x] **Mac Forecast week strip** — the Calendar chip hides and restores the
      calendar, remembering whether it was the week strip or the full month.
      Hiding clears any picked day, so the list always returns to the full
      agenda. In the strip, clicking the selected day again also clears the
      filter. The current-week strip remembers its week, moves one Monday-first
      week at a time (forward only), and switches to the unchanged full month
      from the month chevron; the full month's chevron folds back to the strip.
      Taller day cells keep the number and count clear of the edges, and the
      full month keeps its resize grip.
- [x] **Todoist-style Forecast calendar** — the month view opens on a
      forward-only week strip starting with the current week, with breathing
      room below the month name and its chevron. The bottom pill drags to
      expand/collapse the full month grid with a smooth height/fade animation.
      The full grid scrolls with the task list, keeps Today and the month
      arrows, and allows moving backwards.
- [x] **Trash swipe actions** — trash rows use right-swipe to restore and
      left-swipe to delete instead of two buttons. Settings → Tasks → Trash
      swipes can swap the sides or turn swipes off, which restores the
      buttons. Permanent delete still asks first.
- [x] **Android sort per view** — Inbox, Today, Forecast and every sortable
      destination keep their own sort instead of sharing one global order.
- [x] **Widget quick add stays floating** — the widget’s + button opens its
      capture card in its own task, so a background app is not dragged to the
      foreground underneath it.
- [x] **Expanded project icons** — the Mac and Android pickers share 54
      project icons: the original ten plus the Reddit/Todoist-style set
      (travel, sport, food, study, health, home life, media, people, tools and
      nature).
- [x] **Chevron hit target** — the one-circle gap between a task's subtask
      chevron and its flag is part of the expand/collapse target, so the small
      chevron is easier to hit.
- [x] **No-project heading** — grouping a saved perspective by project calls
      projectless tasks "No project" instead of pretending they belong to an
      Inbox group.
- [x] **Android widget theme refresh** — changing the app theme now updates
      every placed widget immediately; it no longer waits for the widget
      options sheet to be opened and saved.
- [x] **Mac and Android step modes** — Parallel and Sequential list only
      child tasks; One-off lists only checklist steps. Sequential dims every
      child after the first incomplete one and refuses to tick it. Switching
      to one-off warns before converting children into steps and dropping their
      task-only details; switching back restores the steps as subtasks. Every
      Mac child row keeps Separate and a Close (×) delete control.
- [x] **Mac task-row icon cluster** — the order is circle, title, subtask
      chevron, flag. The title starts just after the circle, the chevron sits
      immediately beside the title, and the flag sits one circle width after
      the chevron (or after the title when there is no chevron).
- [x] **Inspector subtask spacing** — the Parallel/Sequential/One-off chips,
      the subtask or one-off rows, and the add field now have extra breathing
      room between them.
- [x] **Inspector text editing** — Title, Notes, the add-subtask field and the
      sheet fields accept mouse placement, double-click word selection,
      triple-click select-all and drag selection. The caret is a thin overlay
      instead of an inserted block glyph, so it no longer leaves a wide blank
      cell to its right or shifts the text after it.
- [x] **Mac window lifecycle** — ⌘Q is bound to Quit (and appears in a small
      Flowlist app menu), double-clicking the drag handle zooms the window, and
      closing the red button then clicking the Dock icon rebuilds the window
      instead of leaving a windowless app behind.
- [x] **Attachment row selection** — the whole Attachments row selects,
      including its padding and empty space, and Attachments supports the same
      bulk select mode as the other task lists.
- [x] **Attachment headings** — only Attachments uses the coloured headings:
      date headings in the theme accent and priorities in red/amber/blue. No
      priority or a missing date is a filled badge, which inverts to white with
      grey text in dark mode. Other views keep quiet grey headings, and groups
      leave a clear gap between sections.
- [x] **Forecast month controls** — the month heading and its Today/arrow
      controls appear only in Month mode, underneath the heading. Today aligns
      with the month name, and the arrows have larger, separated touch targets.
      Agenda hides them.
- [x] **Select-mode toggle** — the checklist button enters select mode and
      becomes a close button; pressing it again leaves select mode.
- [x] **Forecast pill spacing** — the Month/Agenda pills have roomier padding
      and a wider selected pill, with more space between the selector and the
      month name. Selected day cells are slightly taller so the date pill has
      breathing room.
- [x] **Scrollable Forecast month** — the month grid, weekday header and the
      selected day's tasks share one scroll, so the calendar can move up and
      leave the task list the rest of the screen.
- [x] **Perspective sheet inset** — the New perspective card itself starts
      below the Android status bar, not just its contents.
- [x] **Perspective editor parity** — Mac and Android now offer the same
      filters (text, Inbox only, Flagged only, status, projects, tags with
      child tags, priorities, due/defer windows and estimate), the same
      grouping options, and the same sort list including priority.
- [x] **Android history disclosure** — Undo and Redo fold under a History
      heading and stay collapsed by default.
- [x] **Mac wrapping title editor** — the inspector title edits in a wrapping
      field so a long title keeps its caret visible. Covered by
      `scripts/gui/title-editor-probe.sh`.
- [x] **Empty widget illustration** — a light/dark 2D checklist drawing replaces
      the plain empty-state text.
- [x] **Translucent phone bottom bar** — a translucent surface and hairline
      divider replace the solid bar.
- [x] **Joined Mac task rows** — the per-row drag spacer is gone, so consecutive
      top-level rows touch; metadata keeps normal gaps and still wraps.
- [x] **Widget theme, icon and picker preview** — “Follow app” tracks the app’s
      light/dark choice, the header icon follows the launcher icon colour, and
      the widget picker shows a real preview.
- [x] **Widget day/night colours** — RemoteViews carries both palettes, so
      device theme changes apply without waking the app.
- [x] **Geotag notification channel** — geotag arrival reminders have a
      separate Android notification channel, with a link from the app’s
      Notifications section to Android notification settings.
- [x] **Full-screen task reminders** — tasks, projects and folders opt in at
      creation (and tasks from the inspector). A marked task opens a
      screen-sized reminder on Mac or an alarm-style full-screen-intent
      reminder on Android, with Complete, Snooze ten minutes, Open and Dismiss.
      Marks inherit down through parent tasks, projects and folders.
- [x] **Full-screen reminder polish** — quick capture accepts `fsr:1h30m`,
      `fsr:2d` and bare `fsr`; simultaneous reminders offer **View all** into a
      built-in Full-screen view; marks with no due or explicit time are
      ignored; and Settings can keep one wallpaper or separate light/dark
      wallpapers for both the main window and the reminder.
- [x] **Android reminders over other apps** — with **Display over other apps**
      granted, the phone draws the full-screen reminder above whatever app is
      in front, keeps the alarm notification as the fallback, pauses media
      while it is open (with a Settings switch), and now clears the Mac
      reminder on snooze and complete as well as dismiss.
- [x] **Settings and saved-view polish** — the Settings rail stays fixed while
      a section scrolls, full-screen reminder times earlier than now are
      refused, saved views can filter on full-screen reminder marks (including
      inherited ones), empty saved views can hide from the rail until they
      match again, and Appearance stores separate light and dark wallpapers.
- [x] **Mac tag groups** — the sidebar TAGS section splits into General and
      Geotags, with the group labels indented under the section heading. The
      review-reminder dot is gone from project rows, the Review count stays
      visible like Today, and the General/Geotags headings appear only when
      both kinds of tags exist.
- [x] **Android search and Nearby** — search covers projects, folders, tags and
      perspectives with a view/library scope; Nearby lists tags within the
      library radius, sorted by distance. Search results also support bulk
      complete/trash.
- [x] **Android geotags and arrival reminders** — tags carry a place from a
      search (with a selected-place card), the current location or Maps; Nearby
      measures from the phone, and geofenced notifications fire when arriving
      near a geotagged tag with open tasks.
- [x] **Android multi-select** — a header checklist opens select mode; bulk
      complete/reopen, bulk trash with one Undo, and Trash select-all plus a
      confirmed delete-forever.
- [x] **Android tablet layout** — a navigation rail carries the chosen
      perspectives and Settings on large screens, with the phone bottom bar
      hidden and content sized for the extra width.
- [x] **Todoist-inspired Android widgets** — Today, Inbox and Upcoming
      home-screen widgets with an app-icon header, count, Overdue/date
      sections, and configuration for theme, list length, text size,
      independent header size, background opacity, frosted tint, project
      labels, completed rows and quick add. The widget can be added from
      Settings → Tasks.
- [x] **Widget actions** — complete in place. The **+** opens a floating
      capture card over the home screen. The inline chevron switcher was
      removed after it proved unreliable; the view is chosen in widget
      settings. The row-level Tmrw reschedule action was removed after
      feedback. Android widgets cannot blur the wallpaper itself, so the
      frosted option is a translucent tint.
- [-] **Custom geotag notification sounds** — dropped. Geotag arrival
      reminders now live in their own Android Settings category instead.
- [x] **Inline subtasks** — outlines are expanded by default, so children sit
      under their parent without a first click. The disclosure previously
      disagreed with the render default, which made its first click a no-op;
      both now share one default. `za`/`zo`/`zc` fold the selected task, and
      the GUI test state lists the painted rows and disclosure boxes.
- [x] **Calendar view** — the rich month calendar lives inside Forecast now:
      a full month grid with event pills, drag-to-reschedule, keyboard day and
      month navigation, a collapsible header chip and a mouse-adjustable
      height. The duplicate Calendar perspective was removed, so there is one
      calendar surface rather than two.
- [x] **Background capture** — `⌃⌥Space` is registered with Carbon's
      `RegisterEventHotKey`, so it fires while another app is in front. It
      opens the floating quick-capture panel and hands off to the same
      quick-entry parser and command layer. The panel sits at the floating
      window level, so it stays above normal windows after focus moves away,
      and `Enter`/`Esc` hand focus back to the previous app. See Phase 4.

## Research and design

- [x] **Task-manager feature research** — first pass completed against a
      reference manual. Findings and adopt/later decisions are below.
- [x] **"Absolutely stunning UI design"** — the deliberate visual pass:
      checkboxes stay visible on a white surface, row actions wait for hover,
      empty views share one illustrated halo, counts read as prose, menus and
      sheets fade in over 120 ms, theme swatches are miniature windows, and
      the status bar keeps the revision and library path in a tooltip. The
      contrast contract in `theme.rs` is unchanged: the automated palette
      audit covers all eleven families in both modes, and the live window was
      checked over a busy desktop.

## Landed in the current UI round

- [x] macOS traffic lights kept in the header by default, with a Settings →
      Appearance toggle to hide them (`W` while the section is open).
- [x] Title bar reduced to the app name only — no app badge, no
      `Flowlist / view`.
- [x] Inspector toggle uses an info (`i`) icon instead of the sidebar icon.
- [x] Forecast opens with a Monday-first month calendar; dates with a task get
      an accent bar, and prev/next/Today navigation is available.
- [x] Header and status chrome use the flat glass tint instead of a
      left-to-right gradient ramp; the sidebar and inspector keep their
      vertical wash.
- [x] Sort and Group menus open under their chip, paint above the list, close
      on an outside click and accept clicks on their items. The scrim closes
      on mouse-up so the item under the pointer is not removed before its
      click is delivered.
- [x] Every sidebar row has a right-click menu (Open, Rename, Delete, New
      child, status, tag location, Empty Trash).
- [x] Sidebar rows reveal edit/delete on hover, including the selected row,
      which no longer pins the icons visible.
- [x] Undoable project deletion in the GUI, CLI and MCP; tasks move to Trash.
- [x] Typing an unknown project path asks before creating the project, and the
      project picker stays closed until the user types.
- [x] New projects use the project icon by default; Folder remains one of the
      ten choices. Tasks converted to projects take the same default.
- [x] Sort, Group and Priority chips all open anchored option menus; Escape,
      an outside click or the same chip closes them, and only one is open at a
      time.
- [x] Project creation sheets omit the corner close control because the footer
      already offers Cancel.
- [x] Tooltips stay hidden for three seconds, and a click mutes the button's
      tooltip until the pointer leaves. Hovering again later restores the
      normal behaviour.
- [x] Trash has a **Select all** button and `⌘A`, so the whole bin can be
      restored or purged in one action.
- [x] The **Add task** action moved to the title bar as a labelled plus button
      that works from any view, so the content header keeps its full width for
      the view controls.
- [x] The content header keeps the view title and Sort/Group/Filter chips on
      one row, with the subtitle on its own row so it is never squeezed. A view
      name that cannot fit beside the controls is omitted rather than clipped
      to a few letters. Search moved to the title bar as a full field at every
      window width, centered on the content pane with the inspector's column
      always reserved so the field never moves or collides with the actions,
      and a × to clear the query; the title-bar theme icon was removed.
- [x] Sidebar rows give their full width to the name. Rename, delete, status
      and tag location live in the right-click menu, which keeps every row
      uncluttered; long project, folder, tag, perspective and review-card names
      end with an ellipsis instead of stopping mid-word. The inspector wraps
      the full project name so it is never cut off. Library project rows use
      the perspective-style boxed count and align the name with the icon; a
      finished project's tick paints no chip behind it.
- [x] Deleting a project from the sidebar asks for confirmation in a sheet
      that names the project; Enter confirms and Esc cancels.
- [x] Added a title-bar moon/sun button for light/dark, left of the inspector
      toggle. A manual switch turns off follow-system appearance so it sticks.
- [x] Forecast and Calendar merged into one perspective: a single full-width
      month grid with macOS-style separators, event pills, a fold-away chip in
      the content header and a drag grip for its height. The separate Calendar
      perspective is gone.
- [x] Dropdown menus and sheet backdrops occlude the window underneath, so
      choosing an option no longer selects a row behind it.
- [x] The Priority filter is stored per view; sidebar priority rows no longer
      leave their filter on the next perspective.
- [x] Sidebar badges and the status bar exclude tasks in Trash.
- [x] Sidebar tags sort by live usage, then case-insensitive name, with the id
      as a stable tie-break; children follow the same order in their parent.
- [x] Folder badges count only open projects that still have work left, so a
      project showing a tick or a closed project no longer inflates the count.
      Nested folders roll their live projects into the parent's badge.
- [x] Rail sections fold from their headings, folders launch folded, the title
      bar walks view history with back/forward arrows, and Archive rows can be
      deleted with confirmation and Undo.
- [x] Settings is a sectioned sheet (Appearance, Sidebar, Workflow, Nearby,
      Trash, About) with `⌘1`–`⌘6` navigation. It carries section visibility and
      count badges; counts default quiet outside the Library, while Today
      always shows its day count.

## Task-manager gap research (first pass, November 2026)

Source: a widely used task manager's reference manual and glossary. This is a
product comparison, not a claim of feature parity.

### Adopt next

- [x] **Changed perspective** — tenth built-in, `0` shortcut, configurable
  `changed_days` window, newest edits first.
- [x] **Action-group types** — parallel/sequential applies to task subtrees,
  shown as chips in the inspector.
- [x] **First Available view option** — header toggle that keeps only
  currently available rows.
- [x] **Per-perspective view options** — `Session.view_options` stores sort,
  grouping, completed and availability per view.

### Later / evaluate

- **Forecast Tag** — a designated tag marks items for Forecast even without a
  due or defer date. This is a clean fit for our tag model; add
  a Settings field that names the tag and include it in the Forecast query.
- **Phone project grouping** — the Mac and CLI group matching projects by
  folder or date with nested actions. The phone preserves the new fields but
  its editor and list still show a flat Projects block.
- **Favorite perspectives and a perspectives bar** — the reference app lets
  users reorder and hide perspectives, and assigns keyboard shortcuts per
  perspective. Our sidebar order is fixed. Add favorites, drag reorder and a
  per-view shortcut field.
- **Quick Open** — one palette that finds perspectives, folders, projects,
  tags and plug-ins from anywhere. Flowlist search covers tasks and entities;
  a command palette would be the next step, not a second search UI.
- **Clean up resolved items** — the reference app decides when completed rows
  leave the list, and warns before deleting a container that hides completed
  children.
  Add a cleanup policy setting and a pre-delete warning for hidden subtrees.
- **Review sidebar calendar** — the Review perspective pairs a calendar of
  upcoming review dates with the project list. Add a mini calendar to our
  review dashboard.
- **Latest Start in Forecast and sorting** — the notification already fires at
  due minus estimate. The same derived date is not yet a Forecast filter or a
  sort key.
- **Custom layouts / compact rows** — per-perspective density, columns and row
  fields. Keep this behind the design pass so it does not fragment the one
  visual language.
- **Tag status affecting availability** — an On Hold or Dropped tag can make an
  action unavailable in the reference app. We model tag status but do not yet
  let it gate availability.
- **Convert inbox item to project** — a one-command promotion with the item's
  notes and dates carried across, matching the reference app's Convert to
  Project.
- **System capture surfaces** — Quick Entry, Mail Drop, clippings and
  share-sheet capture. Background capture is already in the backlog; treat
  Mail Drop as a separate import path.
- **Per-date time zones** — dates can float with the wall clock or be
  fixed to a time zone. Flowlist stores one instant and schedules in the system
  time zone. This needs a date-model and sync change across macOS and Android,
  not only a notification change.

### Explicitly skipped for now

- Hosted sync, web accounts, web access and automation plug-ins: they conflict
  with the single local library and the shared Rust command layer.
- Custom fluid/column layouts per device: Flowlist is a single desktop app and
  should keep one adaptive layout until the visual pass is finished.
