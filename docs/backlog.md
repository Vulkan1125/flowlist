# Backlog

This file is the durable source of truth for outstanding work. It exists because
the original list lived in a Things 3 project, and a context reset lost it more
than once. If a conversation mentions "the pending things", it means the open
items here.

Status legend: `[ ]` open, `[~]` in progress, `[?]` needs a decision, `[x]`
landed and verified.

> The Things 3 database on this Mac is not readable (`things` reports "Things
> database not found" and the container has no database file), so this list was
> reconstructed from the previous working notes. Reconcile it with Things when
> the database is available again; do not treat silence as "done".

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
- [ ] **Container-type polish** — OmniFocus also gives action groups their own
      parallel/sequential type. Apply `ProjectKind` semantics to task subtrees,
      not just projects.
- [x] **Forecast date selection** — clicking a calendar date shows that day's
      due/deferred tasks; click again or press `\` to clear. `[`/`]` step a day,
      `{`/`}` step a month.
- [ ] **Calendar view** — a dedicated Calendar perspective with a full month
      grid, day columns or an agenda, drag-to-reschedule and keyboard
      navigation. The Forecast month calendar is a first step, not the whole
      calendar.
- [ ] **Background capture** — a system-wide shortcut that opens a small
      capture field without focusing the main window, plus a menu-bar route.
      Must work with the app in the background and hand off to the same command
      layer.

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

## OmniFocus gap research (first pass, November 2026)

Source: the OmniFocus 4.3.3 reference manual and glossary. This is a product
comparison, not a claim of feature parity.

### Adopt next

- **Changed perspective** — OmniFocus pairs Completed with a Changed view for
  recently edited items. We have nine built-ins now, including Trash, but no
  Changed view. Add it with a configurable window and reuse the history
  timestamps.
- **Action-group types** — parallel/sequential applies to task subtrees in
  OmniFocus, not only projects. Make `ProjectKind` do double duty on parent
  tasks and show the blocked reason per subtree.
- **First Available view option** — OmniFocus can show only the next action in
  a sequential project or group. Add an availability option next to the
  existing Completed and priority filters.
- **Per-perspective view options** — OmniFocus stores availability, structure
  and layout per perspective. Flowlist currently keeps sort/group globally in
  the session. Persist a view-options record on custom perspectives and a
  built-in default for the rest.

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
