//! A hand-rolled Model Context Protocol server over stdio.
//!
//! Supports the modern `2026-07-28` lifecycle (`server/discover`, per-request
//! `_meta`, `resultType`) and the legacy `initialize` handshake used by older
//! clients. stdout carries JSON-RPC lines only; diagnostics go to stderr.

use std::io::{BufRead, Write};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};

use flowlist_core::command::{
    Command as CoreCommand, ImportMode, NewProject, NewTask, ProjectPatch, TagPatch, TaskPatch,
    TaskPlacement,
};
use flowlist_core::db::{ContainerPosition, Db, Position};
use flowlist_core::ids::{ChecklistId, ContainerId, FolderId, ProjectId, TagId, TaskId};
use flowlist_core::model::{
    BuiltinPerspective, Condition, DefaultNotification, FilterNode, Geo, GroupBy, IconSpec,
    Perspective, Priority, ProjectKind, ProjectStatus, SortBy, TagStatus, TaskAlert, TaskStatus,
};
use flowlist_core::parse::{parse_duration, parse_repeat, parse_stamp};
use flowlist_core::paths::DataPaths;
use flowlist_core::persist;
use flowlist_core::query::{self, Group};
use flowlist_core::store::Store;
use flowlist_core::time::{Stamp, local_date, now};
use flowlist_core::{export, search as search_engine, stats, taskpaper};

use crate::render;

pub const SERVER_NAME: &str = "flowlist";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MODERN_PROTOCOL: &str = "2026-07-28";
pub const SUPPORTED_PROTOCOLS: [&str; 4] =
    [MODERN_PROTOCOL, "2025-11-25", "2025-06-18", "2024-11-05"];

pub fn serve(paths: DataPaths) -> Result<()> {
    let store = Store::open(paths).context("opening the library")?;
    if let Some(warning) = store.warning() {
        eprintln!("warning: {warning}");
    }
    let mut server = Server {
        store,
        protocol: SUPPORTED_PROTOCOLS[1].to_string(),
        shutting_down: false,
    };

    let stdin = std::io::stdin();
    let mut stdout = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line.context("reading stdin")?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => server.handle(request),
            Err(error) => Some(error_response(
                Value::Null,
                -32700,
                &format!("parse error: {error}"),
            )),
        };
        if let Some(response) = response {
            serde_json::to_writer(&mut stdout, &response).context("writing response")?;
            stdout.write_all(b"\n").context("writing newline")?;
            stdout.flush().context("flushing stdout")?;
        }
        if server.shutting_down {
            break;
        }
    }
    Ok(())
}

struct Server {
    store: Store,
    protocol: String,
    shutting_down: bool,
}

impl Server {
    fn handle(&mut self, request: Value) -> Option<Value> {
        let id = request.get("id").cloned();
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let modern = is_modern(&params) || self.protocol == MODERN_PROTOCOL;

        if id.is_none() {
            // Notifications need no response.
            return None;
        }
        let id = id.unwrap();

        let result: Result<Value> = match method {
            "initialize" => Ok(self.initialize(&params)),
            "server/discover" => Ok(discover()),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tool_definitions()})),
            "tools/call" => self.call_tool(&params),
            "resources/list" => Ok(json!({"resources": resources_list()})),
            "resources/read" => self.read_resource(&params),
            "prompts/list" => Ok(json!({"prompts": []})),
            "logging/setLevel" => Ok(json!({})),
            "shutdown" => {
                self.shutting_down = true;
                Ok(json!({}))
            }
            other => {
                return Some(error_response(
                    id,
                    -32601,
                    &format!("method not found: {other}"),
                ));
            }
        };

        match result {
            Ok(mut value) => {
                if modern {
                    if let Some(object) = value.as_object_mut() {
                        object.insert("resultType".into(), json!("complete"));
                    }
                }
                Some(json!({"jsonrpc": "2.0", "id": id, "result": value}))
            }
            Err(error) => Some(error_response(id, -32000, &format!("{error:#}"))),
        }
    }

    fn initialize(&mut self, params: &Value) -> Value {
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(MODERN_PROTOCOL);
        self.protocol = if SUPPORTED_PROTOCOLS.contains(&requested) {
            requested.to_string()
        } else {
            MODERN_PROTOCOL.to_string()
        };
        json!({
            "protocolVersion": self.protocol,
            "capabilities": capabilities(),
            "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
            "instructions": "Flowlist is a local-first task manager. Use list_tasks with a view or filter, then add_task/update_task/complete_task. Every mutation is undoable with undo.",
        })
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("tools/call requires a tool name"))?;
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        if let Err(error) = self.store.refresh() {
            eprintln!("warning: could not refresh library: {error}");
        }
        match self.dispatch(name, &args) {
            Ok(value) => {
                let text = if let Some(text) = value.as_str() {
                    text.to_string()
                } else {
                    serde_json::to_string_pretty(&value).unwrap_or_default()
                };
                Ok(json!({
                    "content": [{"type": "text", "text": text}],
                    "structuredContent": value,
                    "isError": false,
                }))
            }
            Err(error) => Ok(json!({
                "content": [{"type": "text", "text": format!("Error: {error:#}")}],
                "isError": true,
            })),
        }
    }

    fn dispatch(&mut self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "capture" | "add_task" => self.tool_add_task(args),
            "get_task" => self.tool_get_task(args),
            "update_task" => self.tool_update_task(args),
            "complete_task" => self.tool_complete_task(args),
            "delete_task" => self.tool_delete_task(args),
            "list_trash" => self.tool_list_trash(args),
            "restore_tasks" => self.tool_restore_tasks(args),
            "purge_tasks" => self.tool_purge_tasks(args),
            "empty_trash" => self.tool_empty_trash(args),
            "move_task" => self.tool_move_task(args),
            "convert_task_to_project" => self.tool_convert_task(args),
            "set_task_tags" => self.tool_set_task_tags(args),
            "list_checklist" => self.tool_list_checklist(args),
            "add_checklist_item" => self.tool_add_checklist_item(args),
            "update_checklist_item" => self.tool_update_checklist_item(args),
            "delete_checklist_item" => self.tool_delete_checklist_item(args),
            "move_checklist_item" => self.tool_move_checklist_item(args),
            "list_tasks" => self.tool_list_tasks(args),
            "search" => self.tool_search(args),
            "list_projects" => self.tool_list_projects(args),
            "create_project" => self.tool_create_project(args),
            "update_project" => self.tool_update_project(args),
            "set_project_status" => self.tool_set_project_status(args),
            "archive_projects" => self.tool_archive_projects(args),
            "archive_folder" => self.tool_archive_folder(args),
            "delete_project" => self.tool_delete_project(args),
            "list_tags" => self.tool_list_tags(args),
            "create_tag" => self.tool_create_tag(args),
            "delete_tag" => self.tool_delete_tag(args),
            "list_folders" => self.tool_list_folders(args),
            "create_folder" => self.tool_create_folder(args),
            "review_list" => self.tool_review_list(args),
            "complete_review" => self.tool_complete_review(args),
            "stats" => self.tool_stats(args),
            "undo" => self.tool_undo(args, false),
            "redo" => self.tool_undo(args, true),
            "export_library" => self.tool_export(args),
            "import_library" => self.tool_import(args),
            "library_info" => self.tool_library_info(args),
            other => bail!("unknown tool: {other}"),
        }
    }

    // ---------------------------------------------------------------- tasks

    fn tool_add_task(&mut self, args: &Value) -> Result<Value> {
        let title = require_string(args, "title")?;
        let reference = now();
        let mut new_task = NewTask::new(title);
        if let Some(notes) = string_arg(args, "notes") {
            new_task.notes = notes;
        }
        if let Some(project) = string_arg(args, "project") {
            new_task.project_path = Some(project);
        }
        if let Some(parent) = string_arg(args, "parent") {
            new_task.parent = Some(self.store.db().resolve_task(&parent)?);
        }
        new_task.tag_paths = string_array(args, "tags");
        if let Some(due) = string_arg(args, "due") {
            new_task.due = Some(parse_stamp(&due, reference)?);
        }
        if let Some(defer) = string_arg(args, "defer") {
            new_task.defer = Some(parse_stamp(&defer, reference)?);
        }
        if let Some(planned) = string_arg(args, "planned") {
            new_task.planned = Some(parse_stamp(&planned, reference)?);
        }
        if let Some(estimate) = args.get("estimate_min").and_then(Value::as_u64) {
            new_task.estimate_min = Some(estimate as u32);
        } else if let Some(estimate) = string_arg(args, "estimate") {
            new_task.estimate_min = Some(parse_duration(&estimate)?);
        }
        if bool_arg(args, "flagged").unwrap_or(false) {
            new_task.flagged = true;
        }
        if let Some(priority) = string_arg(args, "priority") {
            new_task.priority = Some(parse_priority(&priority)?);
        }
        if let Some(alerts) = string_arg(args, "alerts") {
            new_task.alerts = if alerts.eq_ignore_ascii_case("none") {
                Vec::new()
            } else {
                TaskAlert::parse_list(&alerts).map_err(|error| anyhow!(error))?
            };
        }
        if let Some(values) = args.get("removed_notifications").and_then(Value::as_array) {
            for value in values {
                let slug = value
                    .as_str()
                    .ok_or_else(|| anyhow!("removed_notifications values must be strings"))?;
                let kind = DefaultNotification::from_slug(slug)
                    .ok_or_else(|| anyhow!("Unknown notification type: {slug}"))?;
                new_task.removed_notifications.insert(kind);
            }
        }
        if let Some(values) = args.get("custom_notifications").and_then(Value::as_array) {
            for value in values {
                let text = value
                    .as_str()
                    .ok_or_else(|| anyhow!("custom_notifications values must be strings"))?;
                new_task
                    .custom_notifications
                    .push(parse_stamp(text, reference)?);
            }
            new_task.custom_notifications.sort_by_key(|stamp| stamp.at);
        }
        if let Some(repeat) = string_arg(args, "repeat") {
            new_task.repeat = Some(parse_repeat(&repeat, reference)?);
        }
        if bool_arg(args, "sequential").unwrap_or(false) {
            new_task.sequential = true;
        }
        new_task.attachments = string_array(args, "attachments");
        let outcome = self.store.execute(CoreCommand::CreateTask(new_task))?;
        let created = outcome
            .created
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("task was not created"))?;
        let id = self.store.db().resolve_task(&created)?;
        Ok(render::task_value(self.store.db(), &id))
    }

    fn tool_get_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let value = render::task_value(self.store.db(), &id);
        if value.is_null() {
            bail!("task not found: {id}");
        }
        Ok(value)
    }

    fn tool_update_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let reference = now();
        let mut patch = TaskPatch::default();
        if let Some(title) = string_arg(args, "title") {
            patch.title = Some(title);
        }
        if let Some(notes) = string_arg(args, "notes") {
            patch.notes = Some(notes);
        }
        patch.due = stamp_patch(args, "due", reference)?;
        patch.defer = stamp_patch(args, "defer", reference)?;
        patch.planned = stamp_patch(args, "planned", reference)?;
        patch.estimate_min = estimate_patch(args)?;
        if let Some(flagged) = bool_arg(args, "flagged") {
            patch.flagged = Some(flagged);
        }
        if let Some(priority) = string_arg(args, "priority") {
            patch.priority = if priority.eq_ignore_ascii_case("none") {
                Some(None)
            } else {
                Some(Some(parse_priority(&priority)?))
            };
        }
        if let Some(alerts) = string_arg(args, "alerts") {
            patch.alerts = Some(if alerts.eq_ignore_ascii_case("none") {
                Vec::new()
            } else {
                TaskAlert::parse_list(&alerts).map_err(|error| anyhow!(error))?
            });
        }
        if let Some(values) = args.get("removed_notifications").and_then(Value::as_array) {
            let mut removed = std::collections::BTreeSet::new();
            for value in values {
                let slug = value
                    .as_str()
                    .ok_or_else(|| anyhow!("removed_notifications values must be strings"))?;
                let kind = DefaultNotification::from_slug(slug)
                    .ok_or_else(|| anyhow!("Unknown notification type: {slug}"))?;
                removed.insert(kind);
            }
            patch.removed_notifications = Some(removed);
        }
        if let Some(values) = args.get("custom_notifications").and_then(Value::as_array) {
            let mut custom = Vec::new();
            for value in values {
                let text = value
                    .as_str()
                    .ok_or_else(|| anyhow!("custom_notifications values must be strings"))?;
                custom.push(parse_stamp(text, reference)?);
            }
            custom.sort_by_key(|stamp| stamp.at);
            patch.custom_notifications = Some(custom);
        }
        if let Some(repeat) = repeat_patch(args, reference)? {
            patch.repeat = Some(repeat);
        }
        if let Some(project) = project_patch(args)? {
            patch.project = Some(project);
        }
        if let Some(parent) = parent_patch(args)? {
            patch.parent = Some(parent);
        }
        if let Some(sequential) = bool_arg(args, "sequential") {
            patch.sequential = Some(sequential);
        }
        if bool_arg(args, "clear_attachments").unwrap_or(false) {
            patch.attachments = Some(Vec::new());
        } else if args.get("attachments").is_some() {
            patch.attachments = Some(string_array(args, "attachments"));
        }
        self.store.execute(CoreCommand::UpdateTask {
            id: id.clone(),
            patch,
        })?;
        Ok(render::task_value(self.store.db(), &id))
    }

    fn tool_complete_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let completed = bool_arg(args, "completed").unwrap_or(true);
        let outcome = self.store.execute(CoreCommand::CompleteTasks {
            ids: vec![id.clone()],
            completed,
        })?;
        Ok(json!({
            "task": render::task_value(self.store.db(), &id),
            "created": outcome.created,
            "completed": completed,
        }))
    }

    fn tool_delete_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let series = bool_arg(args, "series").unwrap_or(false);
        if series {
            self.store.execute(CoreCommand::DeleteTaskSeries {
                ids: vec![id.clone()],
            })?;
        } else {
            self.store.execute(CoreCommand::DeleteTasks {
                ids: vec![id.clone()],
            })?;
        }
        Ok(json!({"deleted": id.to_string(), "series": series}))
    }

    fn tool_list_trash(&mut self, _args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let at = now();
        let groups = query::view(self.store.db(), BuiltinPerspective::Trash, at);
        Ok(render::groups_value(self.store.db(), &groups))
    }

    fn tool_restore_tasks(&mut self, args: &Value) -> Result<Value> {
        let ids = self.resolve_id_list(args)?;
        let outcome = self
            .store
            .execute(CoreCommand::RestoreTasks { ids: ids.clone() })?;
        Ok(json!({
            "restored": ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "message": outcome.message,
        }))
    }

    fn tool_purge_tasks(&mut self, args: &Value) -> Result<Value> {
        let ids = self.resolve_id_list(args)?;
        let outcome = self
            .store
            .execute(CoreCommand::PurgeTasks { ids: ids.clone() })?;
        Ok(json!({
            "purged": ids.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "message": outcome.message,
        }))
    }

    fn tool_empty_trash(&mut self, _args: &Value) -> Result<Value> {
        let outcome = self
            .store
            .execute(CoreCommand::PurgeExpiredTrash { before: now() })?;
        Ok(json!({"message": outcome.message}))
    }

    fn resolve_id_list(&self, args: &Value) -> Result<Vec<TaskId>> {
        let raw = id_list(args, "ids").or_else(|_| id_list(args, "id"))?;
        if raw.is_empty() {
            bail!("provide `ids` (or `id`) with at least one task");
        }
        let mut ids = Vec::with_capacity(raw.len());
        for spec in &raw {
            ids.push(self.store.db().resolve_task(spec)?);
        }
        Ok(ids)
    }

    fn tool_move_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let position = if let Some(before) = string_arg(args, "before") {
            Position::Before(self.store.db().resolve_task(&before)?)
        } else if let Some(after) = string_arg(args, "after") {
            Position::After(self.store.db().resolve_task(&after)?)
        } else {
            Position::End
        };
        let placement = if bool_arg(args, "inbox").unwrap_or(false) {
            TaskPlacement::Inbox(position)
        } else if let Some(parent) = string_arg(args, "parent") {
            TaskPlacement::Parent {
                parent: self.store.db().resolve_task(&parent)?,
                position,
            }
        } else if let Some(project) = string_arg(args, "project") {
            let project = self
                .store
                .db()
                .find_project_by_path(&project)
                .or_else(|| self.store.db().find_project_by_name(&project))
                .ok_or_else(|| anyhow!("project not found: {project}"))?;
            TaskPlacement::Project { project, position }
        } else {
            bail!("move_task needs --project, --parent or inbox");
        };
        self.store.execute(CoreCommand::MoveTasks {
            ids: vec![id.clone()],
            placement,
        })?;
        Ok(render::task_value(self.store.db(), &id))
    }

    fn tool_set_task_tags(&mut self, args: &Value) -> Result<Value> {
        let specs = id_list(args, "id")?;
        let ids = specs
            .iter()
            .map(|spec| {
                self.store
                    .db()
                    .resolve_task(spec)
                    .map_err(|error| anyhow!("{error}"))
            })
            .collect::<Result<Vec<_>>>()?;
        let add = string_array(args, "add");
        let remove = string_array(args, "remove");
        let mut add_ids = Vec::new();
        for path in add {
            add_ids.push(self.ensure_tag_chain(&path)?);
        }
        let mut remove_ids = Vec::new();
        for spec in remove {
            remove_ids.push(resolve_tag(self.store.db(), &spec)?);
        }
        self.store.execute(CoreCommand::SetTaskTags {
            ids: ids.clone(),
            add: add_ids,
            remove: remove_ids,
        })?;
        Ok(json!({
            "tasks": ids.iter().map(|id| render::task_value(self.store.db(), id)).collect::<Vec<_>>()
        }))
    }

    fn tool_list_checklist(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let task_id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let task = self
            .store
            .db()
            .tasks
            .get(&task_id)
            .ok_or_else(|| anyhow!("task not found"))?;
        Ok(json!({
            "task_id": task_id.to_string(),
            "task_title": task.title,
            "items": task.checklist.iter().map(|item| json!({
                "id": item.id.to_string(),
                "short_id": item.id.short(),
                "title": item.title,
                "completed": item.completed,
            })).collect::<Vec<_>>(),
            "completed": task.checklist_progress().0,
            "total": task.checklist_progress().1,
        }))
    }

    fn resolve_checklist_item(&self, task: &TaskId, spec: &str) -> Result<ChecklistId> {
        let task_entity = self
            .store
            .db()
            .tasks
            .get(task)
            .ok_or_else(|| anyhow!("task not found"))?;
        let matches: Vec<&ChecklistId> = task_entity
            .checklist
            .iter()
            .filter(|item| item.id.matches_spec(spec))
            .map(|item| &item.id)
            .collect();
        match matches.as_slice() {
            [] => bail!("checklist item not found: {spec}"),
            [item] => Ok((*item).clone()),
            _ => bail!("checklist item id is ambiguous: {spec}"),
        }
    }

    fn tool_add_checklist_item(&mut self, args: &Value) -> Result<Value> {
        let task = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let title = require_string(args, "title")?;
        let position = args
            .get("position")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let outcome = self.store.execute(CoreCommand::AddChecklistItem {
            task: task.clone(),
            title,
            position,
        })?;
        let created = outcome.created.first().cloned().unwrap_or_default();
        Ok(json!({
            "task": render::task_value(self.store.db(), &task),
            "item_id": created,
        }))
    }

    fn tool_update_checklist_item(&mut self, args: &Value) -> Result<Value> {
        let task = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let item = self.resolve_checklist_item(&task, &require_string(args, "item")?)?;
        let title = string_arg(args, "title").unwrap_or_else(|| {
            self.store
                .db()
                .tasks
                .get(&task)
                .and_then(|task| task.checklist_item(&item))
                .map(|item| item.title.clone())
                .unwrap_or_default()
        });
        let completed = args.get("completed").and_then(Value::as_bool);
        self.store.execute(CoreCommand::UpdateChecklistItem {
            task: task.clone(),
            item,
            title,
            completed,
        })?;
        Ok(render::task_value(self.store.db(), &task))
    }

    fn tool_delete_checklist_item(&mut self, args: &Value) -> Result<Value> {
        let task = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let item = self.resolve_checklist_item(&task, &require_string(args, "item")?)?;
        self.store.execute(CoreCommand::DeleteChecklistItem {
            task: task.clone(),
            item,
        })?;
        Ok(render::task_value(self.store.db(), &task))
    }

    fn tool_move_checklist_item(&mut self, args: &Value) -> Result<Value> {
        let task = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let item = self.resolve_checklist_item(&task, &require_string(args, "item")?)?;
        let to = args
            .get("to")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("move_checklist_item requires `to`"))? as usize;
        self.store.execute(CoreCommand::MoveChecklistItem {
            task: task.clone(),
            item,
            to,
        })?;
        Ok(render::task_value(self.store.db(), &task))
    }

    fn tool_convert_task(&mut self, args: &Value) -> Result<Value> {
        let id = self.store.db().resolve_task(&require_string(args, "id")?)?;
        let folder = match string_arg(args, "folder") {
            Some(path) => Some(self.ensure_folder(&path)?),
            None => None,
        };
        let outcome = self
            .store
            .execute(CoreCommand::ConvertTaskToProject { id, folder })?;
        let created = outcome.created.first().cloned().unwrap_or_default();
        let project = self.store.db().resolve_project(&created)?;
        Ok(render::project_value(self.store.db(), &project))
    }

    fn tool_list_tasks(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let at = now();
        let db = self.store.db();
        let view = string_arg(args, "view").unwrap_or_else(|| "all".to_string());
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize);

        let mut groups: Vec<Group> = match view.to_lowercase().as_str() {
            "inbox" => query::view(db, BuiltinPerspective::Inbox, at),
            "today" => query::view(db, BuiltinPerspective::Today, at),
            "projects" => query::view(db, BuiltinPerspective::Projects, at),
            "forecast" => query::view(db, BuiltinPerspective::Forecast, at),
            "flagged" => query::view(db, BuiltinPerspective::Flagged, at),
            "tags" => query::view(db, BuiltinPerspective::Tags, at),
            "completed" => query::view(db, BuiltinPerspective::Completed, at),
            "archive" => query::view(db, BuiltinPerspective::Archive, at),
            "nearby" => query::view(db, BuiltinPerspective::Nearby, at),
            "changed" => query::view(db, BuiltinPerspective::Changed, at),
            "attachments" => query::view(db, BuiltinPerspective::Attachments, at),
            "fullscreen" => query::view(db, BuiltinPerspective::Fullscreen, at),
            "trash" => query::view(db, BuiltinPerspective::Trash, at),
            "review" => Vec::new(),
            "custom" | "perspective" => {
                let spec = string_arg(args, "perspective")
                    .ok_or_else(|| anyhow!("view=custom requires perspective"))?;
                let perspective = find_perspective(db, &spec)?;
                let perspective: Perspective = db.perspectives[&perspective].clone();
                query::custom(db, &perspective, at)
            }
            _ => {
                let mut conditions: Vec<Condition> = Vec::new();
                let mut show_completed = bool_arg(args, "show_completed").unwrap_or(false);
                if bool_arg(args, "flagged").unwrap_or(false) {
                    conditions.push(Condition::Flagged { value: true });
                }
                if bool_arg(args, "completed").unwrap_or(false) {
                    conditions.push(Condition::Status {
                        value: TaskStatus::Completed,
                    });
                    show_completed = true;
                }
                if bool_arg(args, "inbox").unwrap_or(false) {
                    conditions.push(Condition::InInbox);
                }
                if bool_arg(args, "available").unwrap_or(false) {
                    conditions.push(Condition::Status {
                        value: TaskStatus::Available,
                    });
                }
                if let Some(project) = string_arg(args, "project") {
                    let id = db
                        .find_project_by_path(&project)
                        .or_else(|| db.find_project_by_name(&project))
                        .ok_or_else(|| anyhow!("project not found: {project}"))?;
                    conditions.push(Condition::Project { id });
                }
                if let Some(folder) = string_arg(args, "folder") {
                    let id = db
                        .find_folder_path(&folder)
                        .ok_or_else(|| anyhow!("folder not found: {folder}"))?;
                    conditions.push(Condition::Folder {
                        id,
                        include_children: true,
                    });
                }
                for tag in string_array(args, "tags") {
                    conditions.push(Condition::Tag {
                        id: resolve_tag(db, &tag)?,
                        include_children: true,
                    });
                }
                if let Some(due) = string_arg(args, "due") {
                    conditions.push(Condition::Due {
                        window: due_window(&due, at)?,
                    });
                }
                if let Some(priority) = string_arg(args, "priority") {
                    conditions.push(Condition::Priority {
                        value: parse_priority(&priority)?,
                    });
                }
                let filter = FilterNode::all_conditions(conditions);
                let sort = string_arg(args, "sort")
                    .map(|value| parse_sort(&value))
                    .transpose()?
                    .unwrap_or(SortBy::Manual);
                let group_by = string_arg(args, "group")
                    .map(|value| parse_group(&value))
                    .transpose()?
                    .unwrap_or(GroupBy::Project);
                query::run(db, &filter, sort, group_by, show_completed, at)
            }
        };

        if view.eq_ignore_ascii_case("review") {
            let projects = query::review_projects(db, at);
            return Ok(json!({
                "projects": projects.iter().map(|id| render::project_value(db, id)).collect::<Vec<_>>(),
            }));
        }

        if let Some(query_text) = string_arg(args, "search") {
            let wanted: std::collections::HashSet<TaskId> =
                search_engine::task_ids(db, &query_text, 1000)
                    .into_iter()
                    .collect();
            groups = groups
                .into_iter()
                .map(|mut group| {
                    group.rows.retain(|row| wanted.contains(&row.id));
                    group
                })
                .filter(|group| !group.rows.is_empty())
                .collect();
        }
        if let Some(limit) = limit {
            let mut remaining = limit;
            groups = groups
                .into_iter()
                .filter_map(|mut group| {
                    if remaining == 0 {
                        return None;
                    }
                    group.rows.truncate(remaining);
                    remaining -= group.rows.len();
                    (!group.rows.is_empty()).then_some(group)
                })
                .collect();
        }
        Ok(render::groups_value(db, &groups))
    }

    fn tool_search(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let query = require_string(args, "query")?;
        let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(25) as usize;
        let hits = search_engine::search(self.store.db(), &query, limit);
        Ok(render::search_value(self.store.db(), &hits))
    }

    // ------------------------------------------------------------- projects

    fn tool_list_projects(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let include_closed = bool_arg(args, "include_closed").unwrap_or(false);
        let db = self.store.db();
        let ids: Vec<ProjectId> = db
            .flatten_containers()
            .into_iter()
            .filter_map(|row| match row.container {
                ContainerId::Project(id) => db
                    .projects
                    .get(&id)
                    .filter(|project| {
                        (include_closed || !project.is_archived())
                            && (include_closed || project.status.is_open())
                    })
                    .map(|_| id),
                ContainerId::Folder(_) => None,
            })
            .collect();
        Ok(json!({
            "projects": ids.iter().map(|id| render::project_value(db, id)).collect::<Vec<_>>(),
        }))
    }

    fn tool_create_project(&mut self, args: &Value) -> Result<Value> {
        let name = require_string(args, "name")?;
        let reference = now();
        let folder = match string_arg(args, "folder") {
            Some(path) => Some(self.ensure_folder(&path)?),
            None => None,
        };
        let mut project = NewProject::new(name);
        project.folder = folder;
        project.notes = string_arg(args, "notes").unwrap_or_default();
        project.due = optional_stamp(args, "due", reference)?;
        project.defer = optional_stamp(args, "defer", reference)?;
        project.planned = optional_stamp(args, "planned", reference)?;
        project.review_interval_days = args
            .get("review_interval_days")
            .and_then(Value::as_u64)
            .map(|value| value as u32);
        project.kind = string_arg(args, "kind")
            .as_deref()
            .map(parse_project_kind)
            .transpose()?
            .unwrap_or(ProjectKind::Parallel);
        project.sequential = bool_arg(args, "sequential").unwrap_or(false)
            || project.kind == ProjectKind::Sequential;
        project.complete_with_last_action =
            bool_arg(args, "complete_with_last_action").unwrap_or(true);
        if let Some(icon) = string_arg(args, "icon") {
            project.icon = parse_project_icon(&icon)?;
        }
        project.attachments = string_array(args, "attachments");
        let outcome = self.store.execute(CoreCommand::CreateProject(project))?;
        let created = outcome.created.first().cloned().unwrap_or_default();
        let id = self.store.db().resolve_project(&created)?;
        Ok(render::project_value(self.store.db(), &id))
    }

    fn tool_update_project(&mut self, args: &Value) -> Result<Value> {
        let id = self
            .store
            .db()
            .resolve_project(&require_string(args, "id")?)?;
        let reference = now();
        let mut patch = ProjectPatch::default();
        if let Some(name) = string_arg(args, "name") {
            patch.name = Some(name);
        }
        if let Some(notes) = string_arg(args, "notes") {
            patch.notes = Some(notes);
        }
        if let Some(folder) = folder_patch(args)? {
            match folder {
                Some(path) => patch.folder = Some(Some(self.ensure_folder(&path)?)),
                None => patch.folder = Some(None),
            }
        }
        patch.due = stamp_patch(args, "due", reference)?;
        patch.defer = stamp_patch(args, "defer", reference)?;
        patch.planned = stamp_patch(args, "planned", reference)?;
        if let Some(days) = args.get("review_interval_days").and_then(Value::as_u64) {
            patch.review_interval_days = Some(days as u32);
        }
        if let Some(next) = string_arg(args, "next_review") {
            patch.next_review = Some(flowlist_core::parse::parse_date(&next, reference)?);
        }
        if let Some(kind) = string_arg(args, "kind") {
            patch.kind = Some(parse_project_kind(&kind)?);
        } else if let Some(sequential) = bool_arg(args, "sequential") {
            patch.sequential = Some(sequential);
        }
        if let Some(complete) = bool_arg(args, "complete_with_last_action") {
            patch.complete_with_last_action = Some(complete);
        }
        if let Some(archived) = bool_arg(args, "archived") {
            patch.archived = Some(archived);
        }
        if let Some(icon) = string_arg(args, "icon") {
            patch.icon = Some(parse_project_icon(&icon)?);
        }
        if bool_arg(args, "clear_attachments").unwrap_or(false) {
            patch.attachments = Some(Vec::new());
        } else if args.get("attachments").is_some() {
            patch.attachments = Some(string_array(args, "attachments"));
        }
        if let Some(alerts) = string_arg(args, "alerts") {
            patch.alerts = Some(if alerts.eq_ignore_ascii_case("none") {
                Vec::new()
            } else {
                TaskAlert::parse_list(&alerts).map_err(|error| anyhow!(error))?
            });
        }
        if let Some(values) = args.get("removed_notifications").and_then(Value::as_array) {
            let mut removed = std::collections::BTreeSet::new();
            for value in values {
                let slug = value
                    .as_str()
                    .ok_or_else(|| anyhow!("removed_notifications values must be strings"))?;
                let kind = DefaultNotification::from_slug(slug)
                    .ok_or_else(|| anyhow!("Unknown notification type: {slug}"))?;
                removed.insert(kind);
            }
            patch.removed_notifications = Some(removed);
        }
        if let Some(values) = args.get("custom_notifications").and_then(Value::as_array) {
            let mut custom = Vec::new();
            for value in values {
                let text = value
                    .as_str()
                    .ok_or_else(|| anyhow!("custom_notifications values must be strings"))?;
                custom.push(parse_stamp(text, reference)?);
            }
            custom.sort_by_key(|stamp| stamp.at);
            patch.custom_notifications = Some(custom);
        }
        self.store.execute(CoreCommand::UpdateProject {
            id: id.clone(),
            patch,
        })?;
        Ok(render::project_value(self.store.db(), &id))
    }

    fn tool_set_project_status(&mut self, args: &Value) -> Result<Value> {
        let id = self
            .store
            .db()
            .resolve_project(&require_string(args, "id")?)?;
        let status = parse_project_status(&require_string(args, "status")?)?;
        self.store.execute(CoreCommand::SetProjectStatus {
            id: id.clone(),
            status,
        })?;
        Ok(render::project_value(self.store.db(), &id))
    }

    fn tool_archive_projects(&mut self, args: &Value) -> Result<Value> {
        let specs = id_list(args, "ids")?;
        if specs.is_empty() {
            bail!("provide at least one project id");
        }
        let mut ids = Vec::new();
        for spec in specs {
            ids.push(self.store.db().resolve_project(&spec)?);
        }
        let archived = bool_arg(args, "archived").unwrap_or(true);
        let outcome = self.store.execute(CoreCommand::ArchiveProjects {
            ids: ids.clone(),
            archived,
        })?;
        Ok(json!({
            "archived": archived,
            "projects": ids.iter().map(|id| render::project_value(self.store.db(), id)).collect::<Vec<_>>(),
            "message": outcome.message,
        }))
    }

    fn tool_archive_folder(&mut self, args: &Value) -> Result<Value> {
        let folder = resolve_folder(self.store.db(), &require_string(args, "folder")?)?;
        let ids: Vec<ProjectId> = self
            .store
            .db()
            .projects
            .values()
            .filter(|project| {
                self.store
                    .db()
                    .project_is_in_folder(&project.id, &folder, true)
            })
            .map(|project| project.id.clone())
            .collect();
        let archived = bool_arg(args, "archived").unwrap_or(true);
        let outcome = self.store.execute(CoreCommand::ArchiveProjects {
            ids: ids.clone(),
            archived,
        })?;
        Ok(json!({
            "folder": self.store.db().folder_path(&folder),
            "archived": archived,
            "projects": ids.iter().map(|id| render::project_value(self.store.db(), id)).collect::<Vec<_>>(),
            "message": outcome.message,
        }))
    }

    fn tool_delete_project(&mut self, args: &Value) -> Result<Value> {
        let id = self
            .store
            .db()
            .resolve_project(&require_string(args, "id")?)?;
        self.store
            .execute(CoreCommand::DeleteProject { id: id.clone() })?;
        Ok(json!({ "deleted": id.to_string() }))
    }

    // ----------------------------------------------------------------- tags
    fn tool_list_tags(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let db = self.store.db();
        let ids: Vec<TagId> = db.flatten_tags().into_iter().map(|row| row.tag).collect();
        let _ = args;
        Ok(json!({
            "tags": ids.iter().map(|id| render::tag_value(db, id)).collect::<Vec<_>>(),
        }))
    }

    fn tool_create_tag(&mut self, args: &Value) -> Result<Value> {
        let path = require_string(args, "path")?;
        let id = self.ensure_tag_chain(&path)?;
        let mut patch = TagPatch::default();
        if let Some(status) = string_arg(args, "status") {
            patch.status = Some(parse_tag_status(&status)?);
        }
        if let (Some(lat), Some(lon)) = (
            args.get("lat").and_then(Value::as_f64),
            args.get("lon").and_then(Value::as_f64),
        ) {
            patch.location = Some(Some(Geo { lat, lon }));
        }
        if patch.name.is_some()
            || patch.parent.is_some()
            || patch.status.is_some()
            || patch.location.is_some()
        {
            self.store.execute(CoreCommand::UpdateTag {
                id: id.clone(),
                patch,
            })?;
        }
        Ok(render::tag_value(self.store.db(), &id))
    }

    fn tool_delete_tag(&mut self, args: &Value) -> Result<Value> {
        let id = resolve_tag(self.store.db(), &require_string(args, "id")?)?;
        self.store
            .execute(CoreCommand::DeleteTag { id: id.clone() })?;
        Ok(json!({"deleted": id.to_string()}))
    }

    // -------------------------------------------------------------- folders

    fn tool_list_folders(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let _ = args;
        let db = self.store.db();
        let folders: Vec<Value> = db
            .flatten_containers()
            .into_iter()
            .filter_map(|row| match &row.container {
                ContainerId::Folder(id) => Some(json!({
                    "id": id.to_string(),
                    "short_id": id.short(),
                    "path": db.folder_path(id),
                    "depth": row.depth,
                    "projects": db.project_count(id),
                })),
                ContainerId::Project(_) => None,
            })
            .collect();
        Ok(json!({"folders": folders}))
    }

    fn tool_create_folder(&mut self, args: &Value) -> Result<Value> {
        let path = require_string(args, "path")?;
        let id = self.ensure_folder(&path)?;
        Ok(json!({
            "id": id.to_string(),
            "short_id": id.short(),
            "path": self.store.db().folder_path(&id),
        }))
    }

    // --------------------------------------------------------------- review

    fn tool_review_list(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let _ = args;
        let db = self.store.db();
        let ids = query::review_projects(db, now());
        Ok(json!({
            "projects": ids.iter().map(|id| render::project_value(db, id)).collect::<Vec<_>>(),
        }))
    }

    fn tool_complete_review(&mut self, args: &Value) -> Result<Value> {
        let id = self
            .store
            .db()
            .resolve_project(&require_string(args, "id")?)?;
        let reviewed_on = match string_arg(args, "on") {
            Some(value) => flowlist_core::parse::parse_date(&value, now())?,
            None => local_date(now()),
        };
        self.store.execute(CoreCommand::CompleteReview {
            id: id.clone(),
            reviewed_on,
        })?;
        Ok(render::project_value(self.store.db(), &id))
    }

    // ------------------------------------------------------------- library

    fn tool_stats(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let _ = args;
        let counts = stats::compute(self.store.db(), now());
        Ok(render::stats_value(&counts))
    }

    fn tool_undo(&mut self, args: &Value, redo: bool) -> Result<Value> {
        let steps = args
            .get("steps")
            .and_then(Value::as_u64)
            .unwrap_or(1)
            .max(1) as usize;
        let mut labels = Vec::new();
        for _ in 0..steps {
            let outcome = if redo {
                self.store.redo()?
            } else {
                self.store.undo()?
            };
            labels.push(outcome.label);
        }
        Ok(json!({"steps": labels}))
    }

    fn tool_export(&mut self, args: &Value) -> Result<Value> {
        let format = string_arg(args, "format").unwrap_or_else(|| "json".to_string());
        let content = match format.to_lowercase().as_str() {
            "json" => export::to_json(self.store.db())?,
            "taskpaper" | "tp" => taskpaper::export(self.store.db()),
            other => bail!("unknown export format: {other}"),
        };
        Ok(json!({"format": format, "content": content}))
    }

    fn tool_import(&mut self, args: &Value) -> Result<Value> {
        let data = require_string(args, "data")?;
        let format = string_arg(args, "format").unwrap_or_else(|| "auto".to_string());
        let normalised = format.to_lowercase();
        let resolved = match normalised.as_str() {
            "auto" => {
                if data.trim_start().starts_with('{') {
                    "json"
                } else {
                    "taskpaper"
                }
            }
            other => other,
        };
        let db = match resolved {
            "json" => export::from_json(&data)?,
            "taskpaper" | "tp" => taskpaper::import(&data)?,
            other => bail!("unknown import format: {other}"),
        };
        let mode = if bool_arg(args, "merge").unwrap_or(false) {
            ImportMode::Merge
        } else {
            ImportMode::Replace
        };
        let outcome = self.store.execute(CoreCommand::Import { db, mode })?;
        Ok(json!({
            "label": outcome.label,
            "message": outcome.message,
            "tasks": self.store.db().tasks.len(),
            "projects": self.store.db().projects.len(),
        }))
    }

    fn tool_library_info(&mut self, args: &Value) -> Result<Value> {
        self.store.refresh()?;
        let _ = args;
        let db = self.store.db();
        let counts = stats::compute(db, now());
        let paths = self.store.paths();
        Ok(json!({
            "name": "Flowlist",
            "version": SERVER_VERSION,
            "schema_version": persist::SCHEMA_VERSION,
            "revision": self.store.revision(),
            "root": paths.root.display().to_string(),
            "data_file": paths.data().display().to_string(),
            "history_file": paths.history().display().to_string(),
            "can_undo": self.store.can_undo(),
            "can_redo": self.store.can_redo(),
            "undo_label": self.store.undo_label(),
            "redo_label": self.store.redo_label(),
            "stats": render::stats_value(&counts),
            "builtin_perspectives": BuiltinPerspective::ALL.iter().map(|perspective| json!({
                "slug": perspective.slug(),
                "name": perspective.label(),
            })).collect::<Vec<_>>(),
        }))
    }

    // -------------------------------------------------------------- helpers

    fn ensure_folder(&mut self, path: &str) -> Result<FolderId> {
        let mut parent: Option<FolderId> = None;
        for part in path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            let siblings = self.store.db().container_children(parent.clone()).to_vec();
            let existing = siblings.into_iter().find_map(|container| match container {
                ContainerId::Folder(id) => self
                    .store
                    .db()
                    .folders
                    .get(&id)
                    .filter(|folder| folder.name.eq_ignore_ascii_case(part))
                    .map(|_| id),
                ContainerId::Project(_) => None,
            });
            parent = Some(match existing {
                Some(id) => id,
                None => {
                    let outcome = self.store.execute(CoreCommand::CreateFolder {
                        name: part.to_string(),
                        parent: parent.clone(),
                        position: ContainerPosition::End,
                        fullscreen_reminder: false,
                    })?;
                    let created = outcome
                        .created
                        .first()
                        .cloned()
                        .ok_or_else(|| anyhow!("folder was not created"))?;
                    self.store.db().resolve_folder(&created)?
                }
            });
        }
        parent.ok_or_else(|| anyhow!("empty folder path"))
    }

    fn ensure_tag_chain(&mut self, path: &str) -> Result<TagId> {
        let mut parent: Option<TagId> = None;
        for part in path
            .split('/')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            let siblings: Vec<TagId> = match &parent {
                Some(id) => self.store.db().tag_children(id).to_vec(),
                None => self.store.db().tag_roots().to_vec(),
            };
            let existing = siblings.into_iter().find(|id| {
                self.store
                    .db()
                    .tags
                    .get(id)
                    .is_some_and(|tag| tag.name.eq_ignore_ascii_case(part))
            });
            parent = Some(match existing {
                Some(id) => id,
                None => {
                    let outcome = self.store.execute(CoreCommand::CreateTag {
                        name: part.to_string(),
                        parent: parent.clone(),
                        status: TagStatus::Available,
                        location: None,
                        position: Position::End,
                    })?;
                    let created = outcome.created.first().cloned().unwrap_or_default();
                    self.store.db().resolve_tag(&created)?
                }
            });
        }
        parent.ok_or_else(|| anyhow!("empty tag path"))
    }

    fn read_resource(&mut self, params: &Value) -> Result<Value> {
        let uri = params
            .get("uri")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("resources/read requires a uri"))?;
        match uri {
            "flowlist://library/summary" => {
                self.store.refresh()?;
                let db = self.store.db();
                let counts = stats::compute(db, now());
                let summary = json!({
                    "root": self.store.paths().root.display().to_string(),
                    "revision": self.store.revision(),
                    "counts": render::stats_value(&counts),
                    "review": query::review_projects(db, now()).iter().map(|id| render::project_value(db, id)).collect::<Vec<_>>(),
                });
                Ok(json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&summary)?,
                    }]
                }))
            }
            other => bail!("unknown resource: {other}"),
        }
    }
}

fn is_modern(params: &Value) -> bool {
    params
        .get("_meta")
        .and_then(|meta| meta.get("io.modelcontextprotocol/protocolVersion"))
        .is_some()
        || params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .is_some_and(|version| version >= MODERN_PROTOCOL)
}

fn capabilities() -> Value {
    json!({
        "tools": {"listChanged": false},
        "resources": {"subscribe": false, "listChanged": false},
        "prompts": {"listChanged": false},
        "logging": {},
    })
}

fn discover() -> Value {
    json!({
        "supportedVersions": SUPPORTED_PROTOCOLS,
        "capabilities": capabilities(),
        "serverInfo": {"name": SERVER_NAME, "version": SERVER_VERSION},
    })
}

fn resources_list() -> Vec<Value> {
    vec![json!({
        "uri": "flowlist://library/summary",
        "name": "Library summary",
        "description": "Counts, review queue and library location",
        "mimeType": "application/json",
    })]
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": code, "message": message},
    })
}

// ------------------------------------------------------------------- schemas

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn s(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn b(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

fn n(description: &str) -> Value {
    json!({"type": "integer", "description": description})
}

fn arr(description: &str) -> Value {
    json!({"type": "array", "items": {"type": "string"}, "description": description})
}

fn tool_definitions() -> Vec<Value> {
    vec![
        tool(
            "capture",
            "Add a task. Supports quick-entry text in `title`, or explicit project/tags/due/defer/estimate fields.",
            schema(
                json!({
                    "title": s("Task title (quick-entry syntax allowed: #tag ::Project @1 due:tomorrow planned:(next monday) est:30m !; @1/@2/@3 set priority)"),
                    "notes": s("Notes"),
                    "project": s("Project or Folder/Project path; created when missing"),
                    "tags": arr("Tag paths such as [\"Errands\", \"Errands/Phone\"]"),
                    "parent": s("Parent task id"),
                    "due": s("Natural-language due date, e.g. tomorrow, next friday 5pm, 2026-12-01"),
                    "defer": s("Natural-language defer date"),
                    "planned": s("Natural-language planned date (never changes availability)"),
                    "estimate_min": n("Estimated minutes"),
                    "flagged": b("Flag the task"),
                    "priority": s("low | medium | high, or null to clear"),
                    "alerts": s("Alerts such as due:-15m, defer, planned:+1h"),
                    "repeat": s("Repeat rule, e.g. every 2 weeks after completion"),
                    "sequential": b("Work through subtasks in order"),
                    "attachments": arr("Local file paths or http(s) links to reference"),
                }),
                &["title"],
            ),
        ),
        tool(
            "add_task",
            "Alias of capture.",
            schema(
                json!({
                    "title": s("Task title"),
                    "notes": s("Notes"),
                    "project": s("Project path"),
                    "tags": arr("Tag paths"),
                    "parent": s("Parent task id"),
                    "due": s("Due date"),
                    "defer": s("Defer date"),
                    "planned": s("Planned date"),
                    "estimate_min": n("Estimated minutes"),
                    "flagged": b("Flag the task"),
                    "priority": s("low | medium | high"),
                    "alerts": s("Extra relative alerts such as due:-15m, defer"),
                    "removed_notifications": arr("Default notification types to remove: defer, planned, latest_start, due"),
                    "custom_notifications": arr("Absolute custom notification dates"),
                    "repeat": s("Repeat rule"),
                }),
                &["title"],
            ),
        ),
        tool(
            "get_task",
            "Fetch one task by id or unique id prefix.",
            schema(json!({"id": s("Task id or prefix")}), &["id"]),
        ),
        tool(
            "update_task",
            "Update a task. Omit a field to leave it unchanged; send null (or \"none\") to clear a date, estimate, project or parent.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "title": s("New title"),
                    "notes": s("New notes"),
                    "due": s("Due date, or null to clear"),
                    "defer": s("Defer date, or null to clear"),
                    "planned": s("Planned date, or null to clear"),
                    "estimate_min": n("Estimated minutes, or null to clear"),
                    "estimate": s("Duration string such as 1h30m"),
                    "flagged": b("Flag state"),
                    "priority": s("low | medium | high, or null to clear"),
                    "alerts": s("Extra relative alerts such as due:-15m, defer; none clears them"),
                    "removed_notifications": arr("Default notification types to remove: defer, planned, latest_start, due"),
                    "custom_notifications": arr("Absolute custom notification dates, or null/[] to clear"),
                    "repeat": s("Repeat rule, or null to clear"),
                    "project": s("Project path, or null to move to the inbox"),
                    "parent": s("Parent task id, or null to clear"),
                    "sequential": b("Sequential children"),
                    "attachments": arr("Replace the task's attachment paths and links"),
                    "clear_attachments": b("Remove every attachment"),
                }),
                &["id"],
            ),
        ),
        tool(
            "complete_task",
            "Complete a task (or reopen it with completed=false). Completing a repeating task creates its next occurrence.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "completed": b("Defaults to true; false reopens the task"),
                }),
                &["id"],
            ),
        ),
        tool(
            "delete_task",
            "Move a task and all of its subtasks to Trash. Set series to move every occurrence of a repeating task too. Use purge_tasks to delete permanently.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "series": b("Move the whole repeating series to Trash"),
                }),
                &["id"],
            ),
        ),
        tool(
            "list_trash",
            "List tasks currently in Trash.",
            schema(json!({}), &[]),
        ),
        tool(
            "restore_tasks",
            "Restore one or more tasks (and their subtrees) from Trash.",
            schema(json!({"ids": arr("Task ids or prefixes")}), &["ids"]),
        ),
        tool(
            "purge_tasks",
            "Permanently delete one or more tasks from Trash.",
            schema(json!({"ids": arr("Task ids or prefixes")}), &["ids"]),
        ),
        tool(
            "empty_trash",
            "Permanently delete every task currently in Trash.",
            schema(json!({}), &[]),
        ),
        tool(
            "move_task",
            "Move a task to the inbox, a project or another task.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "project": s("Destination project path"),
                    "parent": s("Destination parent task id"),
                    "inbox": b("Move to the inbox"),
                    "before": s("Place before this sibling task id"),
                    "after": s("Place after this sibling task id"),
                }),
                &["id"],
            ),
        ),
        tool(
            "convert_task_to_project",
            "Turn a task and its top-level subtasks into a project, in one undoable step.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "folder": s("Folder path for the new project"),
                }),
                &["id"],
            ),
        ),
        tool(
            "set_task_tags",
            "Add or remove tags on a task.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "add": arr("Tag paths to add"),
                    "remove": arr("Tag paths or ids to remove"),
                }),
                &["id"],
            ),
        ),
        tool(
            "list_checklist",
            "List the checklist items of a task with their ids and completion state.",
            schema(json!({"id": s("Task id or prefix")}), &["id"]),
        ),
        tool(
            "add_checklist_item",
            "Append a checklist item to a task. Checklist items are lightweight steps and never appear as tasks.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "title": s("Checklist item title"),
                    "position": n("Insert before this zero-based index; omit to append"),
                }),
                &["id", "title"],
            ),
        ),
        tool(
            "update_checklist_item",
            "Rename a checklist item or mark it complete/incomplete.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "item": s("Checklist item id or prefix"),
                    "title": s("New title"),
                    "completed": b("Completion state"),
                }),
                &["id", "item"],
            ),
        ),
        tool(
            "delete_checklist_item",
            "Remove a checklist item from a task.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "item": s("Checklist item id or prefix"),
                }),
                &["id", "item"],
            ),
        ),
        tool(
            "move_checklist_item",
            "Move a checklist item to a zero-based index.",
            schema(
                json!({
                    "id": s("Task id or prefix"),
                    "item": s("Checklist item id or prefix"),
                    "to": n("Destination index"),
                }),
                &["id", "item", "to"],
            ),
        ),
        tool(
            "list_tasks",
            "List tasks using a built-in view (inbox, today, projects, forecast, flagged, tags, completed, changed, attachments, nearby, review) or a filter.",
            schema(
                json!({
                    "view": s("all | inbox | today | projects | forecast | flagged | tags | completed | changed | attachments | nearby | review | trash | custom"),
                    "perspective": s("Custom perspective name or id (with view=custom)"),
                    "project": s("Project path filter"),
                    "folder": s("Folder path filter (includes child folders)"),
                    "tags": arr("Tag paths; all must match"),
                    "search": s("Text search filter"),
                    "available": b("Only currently available tasks"),
                    "flagged": b("Only flagged tasks"),
                    "inbox": b("Only inbox tasks"),
                    "completed": b("Only completed tasks"),
                    "priority": s("low | medium | high"),
                    "due": s("Due window: today, overdue, tomorrow, next7, next30, none, or a date"),
                    "sort": s("manual | due | defer | planned | name | priority | flagged | estimate | created | completed | due_flagged | tag | added | changed | dropped | review"),
                    "group": s("none | project | folder | tag | tags_combined | due | defer | planned | completed | dropped | added | changed | review | flagged | estimate"),
                    "show_completed": b("Include completed tasks"),
                    "limit": n("Maximum number of rows"),
                }),
                &[],
            ),
        ),
        tool(
            "search",
            "Search tasks, projects, folders, tags and perspectives.",
            schema(
                json!({
                    "query": s("Search text"),
                    "limit": n("Maximum hits (default 25)"),
                }),
                &["query"],
            ),
        ),
        tool(
            "list_projects",
            "List projects in library order.",
            schema(
                json!({"include_closed": b("Include done, dropped and archived projects")}),
                &[],
            ),
        ),
        tool(
            "create_project",
            "Create a project, optionally inside a folder path.",
            schema(
                json!({
                    "name": s("Project name or Folder/Project path"),
                    "folder": s("Folder path; created when missing"),
                    "notes": s("Project notes"),
                    "due": s("Due date"),
                    "defer": s("Defer date"),
                    "planned": s("Planned date; never changes availability"),
                    "review_interval_days": n("Review interval in days"),
                    "kind": s("parallel | sequential | single_actions"),
                    "sequential": b("Work through top-level tasks in order"),
                    "complete_with_last_action": b("Mark the project done when its last task is completed"),
                    "archived": b("Hide the project in Archive; false restores it"),
                    "icon": s("project | folder | star | briefcase | home | heart | book | code | rocket | coffee, or one emoji"),
                    "attachments": arr("Local file paths or http(s) links to reference"),
                }),
                &["name"],
            ),
        ),
        tool(
            "update_project",
            "Update a project.",
            schema(
                json!({
                    "id": s("Project id or prefix"),
                    "name": s("New name"),
                    "notes": s("New notes"),
                    "folder": s("Folder path, or null to move to the root"),
                    "due": s("Due date, or null"),
                    "defer": s("Defer date, or null"),
                    "planned": s("Planned date, or null"),
                    "review_interval_days": n("Review interval in days"),
                    "next_review": s("Next review date"),
                    "kind": s("parallel | sequential | single_actions"),
                    "sequential": b("Sequential tasks"),
                    "complete_with_last_action": b("Mark the project done when its last task is completed"),
                    "icon": s("project | folder | star | briefcase | home | heart | book | code | rocket | coffee, or one emoji"),
                    "attachments": arr("Replace the project's attachment paths and links"),
                    "clear_attachments": b("Remove every attachment"),
                    "alerts": s("Extra relative alerts such as due:-15m, defer; none clears them"),
                    "removed_notifications": arr("Default notification types to remove: defer, planned, latest_start, due"),
                    "custom_notifications": arr("Absolute custom notification dates, or null/[] to clear"),
                }),
                &["id"],
            ),
        ),
        tool(
            "set_project_status",
            "Set a project to active, on-hold, done or dropped.",
            schema(
                json!({
                    "id": s("Project id or prefix"),
                    "status": s("active | on-hold | done | dropped"),
                }),
                &["id", "status"],
            ),
        ),
        tool(
            "archive_projects",
            "Archive projects, or unarchive them with archived: false.",
            schema(
                json!({
                    "ids": arr("Project ids or unambiguous prefixes"),
                    "archived": b("true archives (default), false unarchives"),
                }),
                &["ids"],
            ),
        ),
        tool(
            "archive_folder",
            "Archive every project inside a folder, or unarchive them with archived: false.",
            schema(
                json!({
                    "folder": s("Folder id, prefix or path"),
                    "archived": b("true archives (default), false unarchives"),
                }),
                &["folder"],
            ),
        ),
        tool(
            "delete_project",
            "Delete a project and move its tasks to Trash.",
            schema(
                json!({
                    "id": s("Project id or prefix"),
                }),
                &["id"],
            ),
        ),
        tool(
            "list_tags",
            "List tags with usage counts.",
            schema(json!({}), &[]),
        ),
        tool(
            "create_tag",
            "Create a tag (nested paths supported).",
            schema(
                json!({
                    "path": s("Tag path such as Errands/Phone"),
                    "status": s("available | blocked | on-hold"),
                    "lat": json!({"type": "number", "description": "Latitude"}),
                    "lon": json!({"type": "number", "description": "Longitude"}),
                }),
                &["path"],
            ),
        ),
        tool(
            "delete_tag",
            "Delete a tag and remove it from tasks.",
            schema(json!({"id": s("Tag id or path")}), &["id"]),
        ),
        tool(
            "list_folders",
            "List folders and their project counts.",
            schema(json!({}), &[]),
        ),
        tool(
            "create_folder",
            "Create a folder (nested paths supported).",
            schema(json!({"path": s("Folder path")}), &["path"]),
        ),
        tool(
            "review_list",
            "List projects that are due for review.",
            schema(json!({}), &[]),
        ),
        tool(
            "complete_review",
            "Mark a project reviewed and schedule the next review.",
            schema(
                json!({
                    "id": s("Project id or prefix"),
                    "on": s("Review date (defaults to today)"),
                }),
                &["id"],
            ),
        ),
        tool(
            "stats",
            "Library counters: open, available, flagged, due, overdue, projects and review queue.",
            schema(json!({}), &[]),
        ),
        tool(
            "undo",
            "Undo the most recent change(s).",
            schema(json!({"steps": n("Number of undo steps (default 1)")}), &[]),
        ),
        tool(
            "redo",
            "Redo undone change(s).",
            schema(json!({"steps": n("Number of redo steps (default 1)")}), &[]),
        ),
        tool(
            "export_library",
            "Export the whole library as JSON or TaskPaper.",
            schema(json!({"format": s("json | taskpaper")}), &[]),
        ),
        tool(
            "import_library",
            "Import a JSON or TaskPaper library, replacing or merging.",
            schema(
                json!({
                    "data": s("Document contents"),
                    "format": s("auto | json | taskpaper"),
                    "merge": b("Merge instead of replacing"),
                }),
                &["data"],
            ),
        ),
        tool(
            "library_info",
            "Paths, schema revision, undo state, counters and built-in perspectives.",
            schema(json!({}), &[]),
        ),
    ]
}

// ------------------------------------------------------------------- helpers

fn require_string(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("missing required argument: {key}"))
}

fn string_arg(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn bool_arg(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(Value::as_bool)
}

fn string_array(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn id_list(args: &Value, key: &str) -> Result<Vec<String>> {
    match args.get(key) {
        Some(Value::String(value)) => Ok(vec![value.clone()]),
        Some(Value::Array(values)) => Ok(values
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect()),
        _ => bail!("missing required argument: {key}"),
    }
}

fn stamp_patch(
    args: &Value,
    key: &str,
    reference: chrono::DateTime<chrono::Utc>,
) -> Result<Option<Option<Stamp>>> {
    match args.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) if value.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(Value::String(value)) => Ok(Some(Some(parse_stamp(value, reference)?))),
        _ => bail!("{key} must be a date string or null"),
    }
}

fn repeat_patch(
    args: &Value,
    reference: chrono::DateTime<chrono::Utc>,
) -> Result<Option<Option<flowlist_core::model::RepeatRule>>> {
    match args.get("repeat") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) if value.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(Value::String(value)) => Ok(Some(Some(parse_repeat(value, reference)?))),
        _ => bail!("repeat must be a string or null"),
    }
}

fn estimate_patch(args: &Value) -> Result<Option<Option<u32>>> {
    match args.get("estimate_min") {
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Number(number)) => Ok(Some(Some(number.as_u64().unwrap_or(0) as u32))),
        Some(Value::String(value)) => {
            if value.eq_ignore_ascii_case("none") {
                Ok(Some(None))
            } else {
                Ok(Some(Some(parse_duration(value)?)))
            }
        }
        None => match args.get("estimate") {
            Some(Value::String(value)) => {
                if value.eq_ignore_ascii_case("none") {
                    Ok(Some(None))
                } else {
                    Ok(Some(Some(parse_duration(value)?)))
                }
            }
            _ => Ok(None),
        },
        _ => bail!("estimate_min must be a number or null"),
    }
}

fn project_patch(args: &Value) -> Result<Option<Option<ProjectId>>> {
    match args.get("project") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) if value.eq_ignore_ascii_case("inbox") => Ok(Some(None)),
        Some(Value::String(_)) => Ok(Some(None)),
        _ => bail!("project must be a path or null"),
    }
}

fn parent_patch(args: &Value) -> Result<Option<Option<TaskId>>> {
    match args.get("parent") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) if value.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(Value::String(_)) => Ok(Some(None)),
        _ => bail!("parent must be a task id or null"),
    }
}

fn folder_patch(args: &Value) -> Result<Option<Option<String>>> {
    match args.get("folder") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) => Ok(Some(Some(value.clone()))),
        _ => bail!("folder must be a path or null"),
    }
}

fn optional_stamp(
    args: &Value,
    key: &str,
    reference: chrono::DateTime<chrono::Utc>,
) -> Result<Option<Stamp>> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(parse_stamp(value, reference)?)),
        _ => bail!("{key} must be a date string"),
    }
}

fn find_perspective(db: &Db, spec: &str) -> Result<flowlist_core::ids::PerspectiveId> {
    if let Some(perspective) = db
        .perspectives
        .values()
        .find(|perspective| perspective.name.eq_ignore_ascii_case(spec))
    {
        return Ok(perspective.id.clone());
    }
    db.resolve_perspective(spec)
        .map_err(|error| anyhow!("{error}"))
}

fn resolve_folder(db: &Db, spec: &str) -> Result<FolderId> {
    if let Ok(id) = db.resolve_folder(spec) {
        return Ok(id);
    }
    db.find_folder_path(spec)
        .ok_or_else(|| anyhow!("folder not found: {spec}"))
}

fn resolve_tag(db: &Db, spec: &str) -> Result<TagId> {
    if let Some(id) = db.find_tag_path(spec) {
        return Ok(id);
    }
    db.resolve_tag(spec).map_err(|error| anyhow!("{error}"))
}

fn parse_sort(value: &str) -> Result<SortBy> {
    match value.to_lowercase().as_str() {
        "manual" | "none" => Ok(SortBy::Manual),
        "due" => Ok(SortBy::Due),
        "defer" => Ok(SortBy::Defer),
        "planned" => Ok(SortBy::Planned),
        "name" | "title" => Ok(SortBy::Name),
        "priority" => Ok(SortBy::Priority),
        "flagged" | "flag" => Ok(SortBy::Flagged),
        "estimate" => Ok(SortBy::Estimate),
        "created" => Ok(SortBy::Created),
        "completed" | "completed-at" => Ok(SortBy::CompletedAt),
        "due_flagged" | "due-flagged" | "due+flagged" => Ok(SortBy::DueFlagged),
        "tag" | "tags" => Ok(SortBy::Tag),
        "added" | "added-date" => Ok(SortBy::AddedDate),
        "changed" | "changed-date" => Ok(SortBy::ChangedDate),
        "dropped" | "dropped-date" => Ok(SortBy::DroppedDate),
        "review" | "next-review" => Ok(SortBy::NextReview),
        other => bail!("unknown sort: {other}"),
    }
}

fn parse_group(value: &str) -> Result<GroupBy> {
    match value.to_lowercase().as_str() {
        "none" | "manual" => Ok(GroupBy::None),
        "project" => Ok(GroupBy::Project),
        "folder" => Ok(GroupBy::Folder),
        "tag" | "context" => Ok(GroupBy::Tag),
        "tags_combined" | "tags-combined" | "tags" => Ok(GroupBy::TagsCombined),
        "completed" | "completed-date" => Ok(GroupBy::CompletedDate),
        "dropped" | "dropped-date" => Ok(GroupBy::DroppedDate),
        "added" | "added-date" => Ok(GroupBy::AddedDate),
        "changed" | "changed-date" => Ok(GroupBy::ChangedDate),
        "review" | "next-review" => Ok(GroupBy::NextReview),
        "due" | "due-date" => Ok(GroupBy::DueDate),
        "defer" | "defer-date" => Ok(GroupBy::DeferDate),
        "planned" | "planned-date" => Ok(GroupBy::PlannedDate),
        "flagged" | "flag" => Ok(GroupBy::Flagged),
        "estimate" => Ok(GroupBy::Estimate),
        other => bail!("unknown group: {other}"),
    }
}

fn parse_priority(value: &str) -> Result<Priority> {
    Priority::from_slug(&value.trim().to_lowercase())
        .or_else(|| {
            Priority::ALL
                .into_iter()
                .find(|priority| priority.label().eq_ignore_ascii_case(value.trim()))
        })
        .ok_or_else(|| anyhow!("priority must be low, medium or high"))
}

fn parse_project_icon(value: &str) -> Result<IconSpec> {
    IconSpec::parse(value).map_err(|message| anyhow!(message))
}

fn parse_project_kind(value: &str) -> Result<ProjectKind> {
    ProjectKind::from_slug(value.trim())
        .ok_or_else(|| anyhow!("project kind must be parallel, sequential or single_actions"))
}

fn parse_project_status(value: &str) -> Result<ProjectStatus> {
    match value.to_lowercase().as_str() {
        "active" | "open" => Ok(ProjectStatus::Active),
        "on-hold" | "on_hold" | "hold" => Ok(ProjectStatus::OnHold),
        "done" | "complete" | "completed" => Ok(ProjectStatus::Done),
        "dropped" | "drop" => Ok(ProjectStatus::Dropped),
        other => bail!("unknown project status: {other}"),
    }
}

fn parse_tag_status(value: &str) -> Result<TagStatus> {
    match value.to_lowercase().as_str() {
        "available" | "active" => Ok(TagStatus::Available),
        "blocked" => Ok(TagStatus::Blocked),
        "on-hold" | "on_hold" | "hold" => Ok(TagStatus::OnHold),
        "dropped" | "drop" => Ok(TagStatus::Dropped),
        other => bail!("unknown tag status: {other}"),
    }
}

fn due_window(
    value: &str,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<flowlist_core::model::DueWindow> {
    let today = local_date(at);
    if let Some(window) = query::perspective::window_from_keyword(value, today) {
        return Ok(window);
    }
    Ok(flowlist_core::model::DueWindow::On(
        parse_stamp(value, at)?.local_date(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn server() -> (TempDir, Server) {
        let dir = tempfile::tempdir().unwrap();
        let paths = DataPaths::new(dir.path().join("library"));
        let store = Store::open(paths).unwrap();
        (
            dir,
            Server {
                store,
                protocol: MODERN_PROTOCOL.to_string(),
                shutting_down: false,
            },
        )
    }

    #[test]
    fn archive_tools_hide_and_restore_projects() {
        let (_dir, mut server) = server();
        server
            .tool_create_folder(&json!({"path": "Work"}))
            .unwrap();
        let alpha = server
            .tool_create_project(&json!({"name": "Alpha", "folder": "Work"}))
            .unwrap();
        let beta = server
            .tool_create_project(&json!({"name": "Beta", "folder": "Work"}))
            .unwrap();
        let alpha_id = alpha["id"].as_str().unwrap().to_string();
        let beta_id = beta["id"].as_str().unwrap().to_string();

        let archived = server
            .tool_archive_projects(&json!({
                "ids": [alpha_id.clone(), beta_id.clone()],
                "archived": true,
            }))
            .unwrap();
        assert_eq!(archived["archived"], true);
        assert_eq!(archived["projects"].as_array().unwrap().len(), 2);
        let alpha_key = server.store.db().resolve_project(&alpha_id).unwrap();
        assert!(server.store.db().projects[&alpha_key].is_archived());

        let restored = server
            .tool_update_project(&json!({"id": alpha_id.clone(), "archived": false}))
            .unwrap();
        assert_eq!(restored["archived"], false);
        assert!(!server.store.db().projects[&alpha_key].is_archived());

        let folder = server
            .tool_archive_folder(&json!({"folder": "Work", "archived": true}))
            .unwrap();
        assert_eq!(folder["projects"].as_array().unwrap().len(), 2);
        let beta_key = server.store.db().resolve_project(&beta_id).unwrap();
        assert!(server.store.db().projects[&beta_key].is_archived());

        let folder = server
            .tool_archive_folder(&json!({"folder": "Work", "archived": false}))
            .unwrap();
        assert_eq!(folder["projects"].as_array().unwrap().len(), 2);
        assert!(!server.store.db().projects[&beta_key].is_archived());
    }
}
