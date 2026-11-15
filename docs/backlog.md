# Backlog

This file is the durable source of truth for outstanding work. The user-facing
pending list now lives in `flowlist.md`; this file records the engineering
detail behind it. If a conversation mentions "the pending things", it means the
open items here and in `flowlist.md`.

Status legend: `[ ]` open, `[~]` in progress, `[?]` needs a decision, `[x]`
landed and verified.

> `flowlist.md` supersedes the unreadable Things 3 database. Keep both files in
> sync when work lands; do not treat silence as "done".

## Interaction polish

- [ ] **Toast self-dismissal** — verify in a running window that a toast
      disappears on its own after a few seconds, on click, and when another
      toast replaces it. The timer exists; this is a visual confirmation, not a
      new feature.
- [?] **`o` view switch** — the pending note asked for a view switch on `o`,
      but the current grammar uses `o`/`O` to add a task below/above. Decide
      which wins, keep the other on an unused key, and update the help card,
      README and `docs/features.md` together. Do not silently keep both.
- [x] **Focus after delete** — deleting selected task(s) selects the row that
      took their place, or the previous row at the end of the list. Works in
      Trash too, where delete is permanent.
- [ ] **Tab cycles panes** — outside a text field, `Tab` should move Content →
      Inspector → Sidebar → Content (and `Shift-Tab` reverse), stopping on the
      inspector only when it is open. Inside a text field, `Tab` keeps moving
      between fields.
- [ ] **Capture field order** — in the capture bar and new-task modal, `Tab`
      should walk title → notes/when → project → tags in reading order, with
      `Shift-Tab` reversing and `Enter` submitting from any field.

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
- [x] Sidebar rows reveal edit/delete on hover, including the active row.
- [x] Undoable project deletion in the GUI, CLI and MCP; tasks move to Trash.
- [x] Typing an unknown project path asks before creating the project, and the
      project picker stays closed until the user types.
- [x] New projects use the folder icon by default.
- [x] Sort, Group and Priority chips all open anchored option menus; Escape,
      an outside click or the same chip closes them, and only one is open at a
      time.
- [x] Project creation sheets omit the corner close control because the footer
      already offers Cancel.
- [x] Tooltips stay hidden for about five seconds, and a click mutes the
      button's tooltip until the pointer leaves. Hovering again later restores
      the normal behaviour.
- [x] Trash has a **Select all** button and `⌘A`, so the whole bin can be
      restored or purged in one action.
- [x] The **Add task** action moved to the title bar as a labelled plus button
      that works from any view. Removing it from the content header lets the
      search field shrink so long Sort/Group labels no longer clip the right
      chip under the inspector.

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
