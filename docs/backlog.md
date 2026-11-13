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
- [ ] **Focus after delete** — deleting the selected task(s) should leave the
      keyboard cursor on the next row (or the previous row at the end of the
      list), keep the row visible, and leave `j`/`k` working immediately.
- [ ] **Tab cycles panes** — outside a text field, `Tab` should move Content →
      Inspector → Sidebar → Content (and `Shift-Tab` reverse), stopping on the
      inspector only when it is open. Inside a text field, `Tab` keeps moving
      between fields.
- [ ] **Capture field order** — in the capture bar and new-task modal, `Tab`
      should walk title → notes/when → project → tags in reading order, with
      `Shift-Tab` reversing and `Enter` submitting from any field.

## Data model and features

- [ ] **Trash** — soft-delete tasks (and containers where it makes sense) into
      a restorable Trash view with "put back" and "empty trash". Keep the
      existing undo snapshots; trash is for recovery after the undo window.
- [ ] **Priorities** — optional priority (for example High / Normal / Low) on
      tasks, with sorting and grouping, inspector and CLI/MCP support, and a
      compact row indicator that does not compete with the flag.
- [ ] **OmniFocus project types** — support parallel, sequential and
      single-action projects. Sequential projects should make only the first
      available child actionable; single-action projects should behave like
      task buckets without a project outcome.
- [ ] **Calendar view** — a dedicated Calendar perspective with a full month
      grid, day columns or an agenda, drag-to-reschedule and keyboard
      navigation. The month overview added to Forecast is a first step, not the
      whole calendar.
- [ ] **Background capture** — a system-wide shortcut that opens a small
      capture field without focusing the main window, plus a menu-bar route.
      Must work with the app in the background and hand off to the same command
      layer.

## Research and design

- [ ] **OmniFocus feature research** — compare OmniFocus 4 with Flowlist
      (project types, forecast tags, custom perspectives, review, defer
      semantics) and write a short notes page with adopt / skip / later
      decisions.
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
