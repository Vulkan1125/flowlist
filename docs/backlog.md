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

The four phases are worked in order. Detail for each item lives in the sections
below; this list is the current map.

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

- [ ] **Attachment website links** — http(s) attachments alongside file paths,
      with inspector, row badge and open behaviour.
- [ ] **Folder delete cascade** — deleting a folder also deletes the projects
      inside it, after a clear warning, as one undoable step.

### Phase 3 — Recurring task series

- [ ] **Auto-incrementing repeat titles** — an optional number in the title
      increments on completion, with an end date or “forever”.
- [ ] **Series-aware deletion** — deleting a recurring task asks whether to
      remove this occurrence or the whole series.

### Phase 4 — Larger product bets

- [ ] **Things 3-style quick capture popup** — a small floating capture window
      that leaves the current workspace alone.
- [ ] **Calendar perspective** — a real month/day/agenda view with
      drag-to-reschedule.
- [ ] **Visual design pass** — hierarchy, spacing, motion, hover/focus and the
      glass edges, verified live in every theme.

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
- [x] **OmniFocus project types** — parallel, sequential and single-action
      projects, chosen in the New Project sheet and cycled from the project
      banner. Legacy `sequential` files still map to the sequential kind.
- [x] **Container-type polish** — task subtrees now carry a
      parallel/sequential type, exposed as chips in the inspector.
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
- [x] **Forecast calendar display** — the calendar header has a Months chip
      (Auto / 1 / 2, `M`) and a Width chip (fixed cells / full pane, `W`).
      Both are stored in `Settings`, so the layout survives a relaunch.
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
- [x] **Inline subtasks** — outlines are expanded by default, so children sit
      under their parent without a first click. The disclosure previously
      disagreed with the render default, which made its first click a no-op;
      both now share one default. `za`/`zo`/`zc` fold the selected task, and
      the GUI test state lists the painted rows and disclosure boxes.
- [ ] **Calendar view** — a dedicated Calendar perspective with a full month
      grid, day columns or an agenda, drag-to-reschedule and keyboard
      navigation. The Forecast month calendar is a first step, not the whole
      calendar.
- [x] **Background capture** — `⌃⌥Space` is registered with Carbon's
      `RegisterEventHotKey`, so it fires while another app is in front.
      Flowlist activates, falls back to Inbox when the current view has no
      capture bar, and focuses capture with any previous text selected. It
      hands off to the same quick-entry parser and command layer. A separate
      menu-bar capture window is still open.

## Research and design

- [x] **OmniFocus feature research** — first pass completed against the
      OmniFocus 4.3 reference manual (`support.omnigroup.com/documentation/
      omnifocus/universal/4.3.3/en/`). Findings and adopt/later decisions are
      below.
- [ ] **"Absolutely stunning UI design"** — a deliberate visual pass, not a
      bug fix: hierarchy and type scale, spacing rhythm, hover and focus
      states, empty states, motion, and the glass edges against busy
      wallpapers. Verify every change in a live window and keep the contrast
      contract in `theme.rs` intact.

## Landed in the current UI round

- [x] macOS traffic lights removed from the app window.
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
      window width, centered on the content pane with a × to clear the query;
      the title-bar theme icon was removed.
- [x] Sidebar rows give their full width to the name until hovered. The
      hovered row alone makes room for the pencil and × and fades its long name
      at the edge; idle long project, folder, tag, perspective and review-card
      names end with an ellipsis instead of stopping mid-word. The inspector
      wraps the full project name so it is never cut off. Library project rows
      use the perspective-style boxed count and align the name with the icon.
- [x] Deleting a project from the sidebar asks for confirmation in a sheet
      that names the project; Enter confirms and Esc cancels.
- [x] Added a title-bar moon/sun button for light/dark, left of the inspector
      toggle. A manual switch turns off follow-system appearance so it sticks.
- [x] The Forecast calendar shows two consecutive months once the content pane
      is wide enough, keeping calendar-sized cells on large windows and full
      screen. Narrow panes keep one month.
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
- [x] Settings has a Sidebar group for section visibility and count badges;
      counts default quiet outside the Library, while Today always shows its
      day count.

## OmniFocus gap research (first pass, November 2026)

Source: the OmniFocus 4.3.3 reference manual and glossary. This is a product
comparison, not a claim of feature parity.

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

- **Forecast Tag (OmniFocus Pro)** — a designated tag marks items for Forecast
  even without a due or defer date. This is a clean fit for our tag model; add
  a Settings field that names the tag and include it in the Forecast query.
- **Favorite perspectives and a perspectives bar** — OmniFocus lets users
  reorder and hide perspectives, and assigns keyboard shortcuts per
  perspective. Our sidebar order is fixed. Add favorites, drag reorder and a
  per-view shortcut field.
- **Quick Open** — one palette that finds perspectives, folders, projects,
  tags and plug-ins from anywhere. Flowlist search covers tasks and entities;
  a command palette would be the next step, not a second search UI.
- **Clean up resolved items** — OmniFocus decides when completed rows leave the
  list, and warns before deleting a container that hides completed children.
  Add a cleanup policy setting and a pre-delete warning for hidden subtrees.
- **Review sidebar calendar** — the Review perspective pairs a calendar of
  upcoming review dates with the project list. Add a mini calendar to our
  review dashboard.
- **Latest Start Date** — derived from due date minus estimated duration.
  Useful in Forecast and as a sort key; depends on reliable estimates.
- **Custom layouts / compact rows** — per-perspective density, columns and row
  fields. Keep this behind the design pass so it does not fragment the one
  visual language.
- **Tag status affecting availability** — an On Hold or Dropped tag can make an
  action unavailable in OmniFocus. We model tag status but do not yet let it
  gate availability.
- **Convert inbox item to project** — a one-command promotion with the item's
  notes and dates carried across, matching OmniFocus's Convert to Project.
- **System capture surfaces** — Quick Entry, Mail Drop, clippings and
  share-sheet capture. Background capture is already in the backlog; treat
  Mail Drop as a separate import path.
- **Notifications and due reminders** — OmniFocus alerts on due/defer times.
  Flowlist is local-first and has no notification scheduler yet.

### Explicitly skipped for now

- OmniFocus sync, Omni Accounts, web access and Omni Automation plug-ins: they
  conflict with the single local library and the shared Rust command layer.
- Custom fluid/column layouts per device: Flowlist is a single desktop app and
  should keep one adaptive layout until the visual pass is finished.
