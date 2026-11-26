//! Due-date reminders through macOS UserNotifications.
//!
//! The scheduler is intentionally small: when notifications are enabled, the
//! window removes its pending requests and schedules one for every task due in
//! the next month. Date-only due dates remind at 09:00 local, timestamped ones
//! at their exact time. The desktop app must be running only when the schedule
//! needs to change; macOS keeps already-scheduled reminders after a quit.

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use chrono::{Datelike, Duration, NaiveTime, Timelike};
use flowlist_core::db::Db;
use flowlist_core::model::{AlertEvent, DefaultNotification, Settings, TaskAlert};
use flowlist_core::time::{Stamp, local_to_utc, now};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Unknown,
    NotDetermined,
    Denied,
    Authorized,
    Provisional,
}

impl Permission {
    pub fn label(self) -> &'static str {
        match self {
            Permission::Unknown => "Checking…",
            Permission::NotDetermined => "Not asked yet",
            Permission::Denied => "Denied in System Settings",
            Permission::Authorized => "Allowed",
            Permission::Provisional => "Quietly allowed",
        }
    }

    pub fn allowed(self) -> bool {
        matches!(self, Permission::Authorized | Permission::Provisional)
    }
}

/// The last `UNAuthorizationStatus` we saw. The system reports it through an
/// async callback, so the UI reads this cache and refreshes it when the sheet
/// opens or the user flips the toggle.
static PERMISSION: AtomicU8 = AtomicU8::new(0);
static INSTALLED: OnceLock<()> = OnceLock::new();
static LAST_ERROR: Mutex<Option<String>> = Mutex::new(None);

const DATE_ONLY_HOUR: u32 = 9;
const HORIZON_DAYS: i64 = 30;

pub fn supported() -> bool {
    bundled()
}

/// The last authorization error, if the system refused the request. Useful in
/// the Settings status line and in probes.
pub fn last_error() -> Option<String> {
    LAST_ERROR.lock().ok().and_then(|error| error.clone())
}

/// macOS refuses notification authorization to ad-hoc signed bundles. The
/// code is ready; a released build needs an Apple Development or Developer ID
/// signature before reminders can be granted.
pub fn signature_blocked() -> bool {
    last_error().is_some_and(|error| error.contains("Notifications are not allowed"))
}

pub fn permission() -> Permission {
    // Screenshot/layout probes can exercise the scheduler without clicking a
    // system prompt. Never set outside tests.
    if std::env::var_os("FLOWLIST_TEST_NOTIFY").is_some() {
        return Permission::Authorized;
    }
    match PERMISSION.load(Ordering::Relaxed) {
        1 => Permission::NotDetermined,
        2 => Permission::Denied,
        3 => Permission::Authorized,
        4 => Permission::Provisional,
        _ => Permission::Unknown,
    }
}

fn store_status(status: isize) {
    let value = match status {
        0 => 1, // not determined
        1 => 2, // denied
        2 => 3, // authorized
        3 => 4, // provisional
        _ => 0,
    };
    PERMISSION.store(value, Ordering::Relaxed);
}

/// Ask the system for the current authorization state. The answer arrives on
/// a callback; call again a moment later to see the result.
pub fn refresh_permission() {
    #[cfg(target_os = "macos")]
    unsafe {
        use block::ConcreteBlock;
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        if !bundled() {
            return;
        }
        let center: *mut Object =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return;
        }
        let callback = ConcreteBlock::new(|settings: *mut Object| {
            if settings.is_null() {
                return;
            }
            unsafe {
                let status: isize = msg_send![settings, authorizationStatus];
                store_status(status);
            }
        });
        let callback = callback.copy();
        let _: () = msg_send![center, getNotificationSettingsWithCompletionHandler: &*callback];
        // The framework keeps its own copy; the block is tiny and deliberately
        // left alive for the callback.
        let _ = callback;
    }
}

/// Request Alert + Sound permission. The system only shows the prompt once;
/// after that the result is visible through `refresh_permission`.
pub fn request_permission() {
    #[cfg(target_os = "macos")]
    unsafe {
        use block::ConcreteBlock;
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        if !bundled() {
            return;
        }
        let center: *mut Object =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return;
        }
        let callback = ConcreteBlock::new(|granted: i8, error: *mut Object| {
            let message = unsafe {
                if error.is_null() {
                    None
                } else {
                    let description: *mut Object = msg_send![error, localizedDescription];
                    let text: *const std::os::raw::c_char = if description.is_null() {
                        std::ptr::null()
                    } else {
                        msg_send![description, UTF8String]
                    };
                    Some(if text.is_null() {
                        "notification authorization failed".to_string()
                    } else {
                        std::ffi::CStr::from_ptr(text)
                            .to_string_lossy()
                            .into_owned()
                    })
                }
            };
            if let Ok(mut stored) = LAST_ERROR.lock() {
                *stored = message;
            }
            let _ = granted;
            refresh_permission();
        });
        let callback = callback.copy();
        // UNAuthorizationOptionSound | .alert
        let _: () = msg_send![
            center,
            requestAuthorizationWithOptions: (2usize | 4usize)
            completionHandler: &*callback
        ];
        let _ = callback;
    }
}

/// Install the foreground delegate once. Without it macOS hides banners while
/// Flowlist itself is the active app.
pub fn install() {
    #[cfg(target_os = "macos")]
    {
        if INSTALLED.get().is_some() || !bundled() {
            return;
        }
        if unsafe { install_delegate() } {
            let _ = INSTALLED.set(());
        }
    }
}

/// Remove every pending Flowlist reminder.
pub fn clear() {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        if !bundled() {
            return;
        }
        let center: *mut Object =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if !center.is_null() {
            let _: () = msg_send![center, removeAllPendingNotificationRequests];
        }
    }
}

/// Rebuild the pending reminders from the current library. Returns how many
/// were scheduled.
pub fn schedule(db: &Db) -> usize {
    clear();
    #[cfg(target_os = "macos")]
    unsafe {
        if !bundled() || !permission().allowed() {
            return 0;
        }
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        let center: *mut Object =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return 0;
        }
        let pending = reminders(db, now());
        let mut scheduled = 0usize;
        for reminder in pending {
            let content: *mut Object = msg_send![class!(UNMutableNotificationContent), new];
            let title = nsstring(&reminder.title);
            let body = nsstring(&reminder.body);
            let sound: *mut Object = msg_send![class!(UNNotificationSound), defaultSound];
            let _: () = msg_send![content, setTitle: title];
            let _: () = msg_send![content, setBody: body];
            let _: () = msg_send![content, setSound: sound];

            let local = reminder.at.with_timezone(&chrono::Local);
            let components: *mut Object = msg_send![class!(NSDateComponents), new];
            let _: () = msg_send![components, setYear: local.year() as isize];
            let _: () = msg_send![components, setMonth: local.month() as isize];
            let _: () = msg_send![components, setDay: local.day() as isize];
            let _: () = msg_send![components, setHour: local.hour() as isize];
            let _: () = msg_send![components, setMinute: local.minute() as isize];
            let trigger: *mut Object = msg_send![
                class!(UNCalendarNotificationTrigger),
                triggerWithDateMatchingComponents: components
                repeats: 0i8
            ];
            let identifier = nsstring(&format!("flowlist.task.{}", reminder.key));
            let request: *mut Object = msg_send![
                class!(UNNotificationRequest),
                requestWithIdentifier: identifier
                content: content
                trigger: trigger
            ];
            if !request.is_null() {
                let _: () = msg_send![
                    center,
                    addNotificationRequest: request
                    withCompletionHandler: std::ptr::null::<Object>()
                ];
                scheduled += 1;
            }
            let _: () = msg_send![content, autorelease];
            let _: () = msg_send![components, autorelease];
        }
        scheduled
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = db;
        0
    }
}

/// A one-off notification for the Settings button.
pub fn send_test() -> bool {
    clear();
    #[cfg(target_os = "macos")]
    unsafe {
        if !bundled() || !permission().allowed() {
            return false;
        }
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        let center: *mut Object =
            msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
        if center.is_null() {
            return false;
        }
        let content: *mut Object = msg_send![class!(UNMutableNotificationContent), new];
        let _: () = msg_send![content, setTitle: nsstring("Flowlist test notification")];
        let _: () = msg_send![content, setBody: nsstring("Due reminders are working.")];
        let sound: *mut Object = msg_send![class!(UNNotificationSound), defaultSound];
        let _: () = msg_send![content, setSound: sound];
        let trigger: *mut Object = msg_send![
            class!(UNTimeIntervalNotificationTrigger),
            triggerWithTimeInterval: 3.0
            repeats: 0i8
        ];
        let request: *mut Object = msg_send![
            class!(UNNotificationRequest),
            requestWithIdentifier: nsstring("flowlist.test")
            content: content
            trigger: trigger
        ];
        let _: () = msg_send![
            center,
            addNotificationRequest: request
            withCompletionHandler: std::ptr::null::<Object>()
        ];
        let _: () = msg_send![content, autorelease];
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn open_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.notifications")
            .spawn();
    }
}

struct Reminder {
    key: String,
    title: String,
    body: String,
    at: chrono::DateTime<chrono::Utc>,
}

/// The notifications that should be pending right now. A notification is
/// generated from the global default switches, per-item removals, relative
/// alerts and absolute custom times. Pure, so the rules can be tested without
/// macOS.
fn reminders(db: &Db, reference: chrono::DateTime<chrono::Utc>) -> Vec<Reminder> {
    let settings = &db.settings;
    let mut pending = Vec::new();
    for task in db.tasks.values() {
        if task.completed || task.is_trashed() {
            continue;
        }
        if db
            .effective_project(&task.id)
            .and_then(|project| db.projects.get(&project))
            .is_some_and(|project| project.status.is_closed())
        {
            continue;
        }
        let location = db
            .effective_project(&task.id)
            .map(|project| db.project_path(&project))
            .unwrap_or_else(|| "Inbox".to_string());
        let tags = item_tags(db, &task.tags);
        push_item_notifications(
            &mut pending,
            settings,
            &format!("task.{}", task.id),
            &task.title,
            &location,
            &tags,
            db.effective_due(&task.id),
            db.effective_defer(&task.id),
            db.effective_planned(&task.id),
            task.estimate_min,
            &task.alerts,
            &task.removed_notifications,
            &task.custom_notifications,
            reference,
        );
    }
    for project in db.projects.values() {
        if project.status.is_closed() {
            continue;
        }
        let location = db.project_path(&project.id);
        let tags = item_tags(db, &project.tags);
        push_item_notifications(
            &mut pending,
            settings,
            &format!("project.{}", project.id),
            &project.name,
            &location,
            &tags,
            project.due,
            project.defer,
            project.planned,
            None,
            &project.alerts,
            &project.removed_notifications,
            &project.custom_notifications,
            reference,
        );
    }
    pending.sort_by_key(|reminder| reminder.at);
    pending
}

fn item_tags(db: &Db, tags: &std::collections::BTreeSet<flowlist_core::ids::TagId>) -> Vec<String> {
    tags.iter()
        .filter_map(|tag| db.tags.get(tag).map(|tag| db.tag_path(&tag.id)))
        .take(2)
        .collect()
}

fn push_item_notifications(
    pending: &mut Vec<Reminder>,
    settings: &Settings,
    item_key: &str,
    title: &str,
    location: &str,
    tags: &[String],
    due: Option<Stamp>,
    defer: Option<Stamp>,
    planned: Option<Stamp>,
    estimate: Option<u32>,
    alerts: &[TaskAlert],
    removed: &std::collections::BTreeSet<DefaultNotification>,
    custom: &[Stamp],
    reference: chrono::DateTime<chrono::Utc>,
) {
    let mut push = |at, label: &str, timing: String, key: String| {
        if at <= reference {
            return;
        }
        let mut parts = vec![location.to_string(), label.to_string()];
        if !tags.is_empty() {
            parts.push(tags.join(", "));
        }
        parts.push(timing);
        pending.push(Reminder {
            key: format!("{item_key}.{key}"),
            title: title.to_string(),
            body: parts.join(" · "),
            at,
        });
    };
    for kind in DefaultNotification::ALL {
        if removed.contains(&kind) {
            continue;
        }
        let enabled = match kind {
            DefaultNotification::Defer => settings.default_defer_notification,
            DefaultNotification::Planned => settings.default_planned_notification,
            DefaultNotification::LatestStart => settings.default_latest_start_notification,
            DefaultNotification::Due => settings.default_due_notification,
        };
        if !enabled {
            continue;
        }
        let at = match kind {
            DefaultNotification::Defer => defer.map(reminder_at),
            DefaultNotification::Planned => planned.map(reminder_at),
            DefaultNotification::Due => due.map(reminder_at),
            DefaultNotification::LatestStart => due
                .zip(estimate)
                .map(|(due, estimate)| reminder_at(due) - Duration::minutes(estimate as i64)),
        };
        if let Some(at) = at {
            push(
                at,
                if kind == DefaultNotification::Defer {
                    "Available"
                } else {
                    kind.label()
                },
                "now".to_string(),
                format!("default.{}", kind.slug()),
            );
        }
    }
    for (index, alert) in alerts.iter().enumerate() {
        let base = match alert.event {
            AlertEvent::Due => due,
            AlertEvent::Defer => defer,
            AlertEvent::Planned => planned,
        };
        if let Some(stamp) = base {
            let at = reminder_at(stamp) + Duration::minutes(alert.offset_minutes as i64);
            push(
                at,
                alert.event.label(),
                timing_label(alert.offset_minutes),
                format!("alert.{index}"),
            );
        }
    }
    for (index, stamp) in custom.iter().enumerate() {
        let at = custom_reminder_at(*stamp, settings);
        push(at, "Custom", "now".to_string(), format!("custom.{index}"));
    }
}

pub(super) fn custom_reminder_at(stamp: Stamp, settings: &Settings) -> chrono::DateTime<chrono::Utc> {
    if stamp.has_time {
        stamp.at
    } else {
        let time = settings
            .default_custom_notification_time
            .unwrap_or_else(|| NaiveTime::from_hms_opt(9, 0, 0).unwrap_or(NaiveTime::MIN));
        local_to_utc(stamp.local_date(), time)
    }
}

fn timing_label(offset: i32) -> String {
    if offset == 0 {
        return "now".to_string();
    }
    let minutes = offset.unsigned_abs();
    let duration = if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else if minutes < 60 {
        format!("{minutes}m")
    } else {
        format!("{}h {}m", minutes / 60, minutes % 60)
    };
    if offset < 0 {
        format!("in {duration}")
    } else {
        format!("{duration} ago")
    }
}

pub(super) fn reminder_at(due: flowlist_core::time::Stamp) -> chrono::DateTime<chrono::Utc> {
    if due.has_time {
        due.at
    } else {
        let time = NaiveTime::from_hms_opt(DATE_ONLY_HOUR, 0, 0).unwrap_or(NaiveTime::MIN);
        local_to_utc(due.local_date(), time)
    }
}

#[cfg(target_os = "macos")]
fn bundled() -> bool {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let bundle: *mut Object = msg_send![class!(NSBundle), mainBundle];
        if bundle.is_null() {
            return false;
        }
        let identifier: *mut Object = msg_send![bundle, bundleIdentifier];
        !identifier.is_null()
    }
}

#[cfg(not(target_os = "macos"))]
fn bundled() -> bool {
    false
}

#[cfg(target_os = "macos")]
fn nsstring(text: &str) -> *mut objc::runtime::Object {
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};
    let text = std::ffi::CString::new(text).unwrap_or_default();
    unsafe { msg_send![class!(NSString), stringWithUTF8String: text.as_ptr()] }
}

#[cfg(target_os = "macos")]
unsafe fn install_delegate() -> bool {
    use block::Block;
    use objc::declare::ClassDecl;
    use objc::runtime::{Object, Protocol, Sel};
    use objc::{class, msg_send, sel, sel_impl};

    #[link(name = "UserNotifications", kind = "framework")]
    unsafe extern "C" {}

    extern "C" fn will_present(
        _: &Object,
        _: Sel,
        _: *mut Object,
        _: *mut Object,
        completion: *mut Object,
    ) {
        if completion.is_null() {
            return;
        }
        // UNNotificationPresentationOptionBanner | .sound
        unsafe {
            let block = &*(completion as *const Block<(usize,), ()>);
            let _: () = block.call((16 | 2,));
        }
    }

    extern "C" fn did_receive(
        _: &Object,
        _: Sel,
        _: *mut Object,
        _: *mut Object,
        completion: *mut Object,
    ) {
        if completion.is_null() {
            return;
        }
        unsafe {
            let block = &*(completion as *const Block<(), ()>);
            let _: () = block.call(());
        }
    }

    let Some(protocol) = Protocol::get("UNUserNotificationCenterDelegate") else {
        return false;
    };
    let Some(mut declaration) = ClassDecl::new("FlowlistNotificationDelegate", class!(NSObject))
    else {
        return false;
    };
    declaration.add_protocol(protocol);
    unsafe {
        declaration.add_method(
            sel!(userNotificationCenter:willPresentNotification:withCompletionHandler:),
            will_present as extern "C" fn(&Object, Sel, *mut Object, *mut Object, *mut Object),
        );
        declaration.add_method(
            sel!(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:),
            did_receive as extern "C" fn(&Object, Sel, *mut Object, *mut Object, *mut Object),
        );
    }
    declaration.register();

    let delegate: *mut Object = msg_send![class!(FlowlistNotificationDelegate), new];
    if delegate.is_null() {
        return false;
    }
    // The center's delegate is weak; keep our singleton alive for the process.
    let _: () = msg_send![delegate, retain];
    let center: *mut Object =
        msg_send![class!(UNUserNotificationCenter), currentNotificationCenter];
    if center.is_null() {
        return false;
    }
    let _: () = msg_send![center, setDelegate: delegate];
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, NaiveTime};
    use flowlist_core::db::Db;
    use flowlist_core::model::{AlertEvent, Project, ProjectStatus, Task, TaskAlert};
    use flowlist_core::time::{Stamp, local_to_utc};

    fn task(title: &str, due: Stamp) -> Task {
        let mut task = Task::new(title);
        task.due = Some(due);
        task
    }

    #[test]
    fn reminders_pick_future_due_tasks_and_use_nine_for_date_only() {
        let mut db = Db::new();
        let reference = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        );
        let later = task(
            "Timestamped",
            Stamp::on(
                NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
                NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            ),
        );
        let date_only = task(
            "Date only",
            Stamp::date_only(NaiveDate::from_ymd_opt(2026, 11, 22).unwrap()),
        );
        let past = task(
            "Past",
            Stamp::on(
                NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
                NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            ),
        );
        db.add_task_to_inbox(later);
        db.add_task_to_inbox(date_only);
        db.add_task_to_inbox(past);

        let pending = reminders(&db, reference);
        assert_eq!(pending.len(), 2);
        let date_only_reminder = pending
            .iter()
            .find(|reminder| reminder.title == "Date only")
            .unwrap();
        assert_eq!(
            date_only_reminder.at.with_timezone(&chrono::Local).hour(),
            9
        );
    }

    #[test]
    fn custom_alerts_schedule_alongside_default_notifications() {
        let mut db = Db::new();
        db.settings.default_due_notification = true;
        let reference = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        );
        let mut custom = task(
            "Custom alerts",
            Stamp::on(
                NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
                NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            ),
        );
        custom.defer = Some(Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        ));
        custom.alerts = vec![
            TaskAlert::new(AlertEvent::Due, -30),
            TaskAlert::new(AlertEvent::Defer, 0),
        ];
        db.add_task_to_inbox(custom);

        let pending = reminders(&db, reference);
        assert_eq!(pending.len(), 3);
        assert!(
            pending
                .iter()
                .any(|reminder| reminder.body.contains("Due · in 30m"))
        );
        assert!(
            pending
                .iter()
                .any(|reminder| reminder.body.contains("Available · now"))
        );
        assert!(
            pending
                .iter()
                .any(|reminder| reminder.body.contains("Due · now"))
        );

        // Removing the default due notification leaves the relative alerts.
        db.tasks
            .values_mut()
            .next()
            .unwrap()
            .removed_notifications
            .insert(DefaultNotification::Due);
        let remaining = reminders(&db, reference);
        assert_eq!(remaining.len(), 2);
        assert!(
            remaining
                .iter()
                .all(|reminder| !reminder.key.contains("default.due"))
        );
    }

    #[test]
    fn default_notifications_cover_defer_planned_latest_start_and_due() {
        let mut db = Db::new();
        db.settings.default_defer_notification = true;
        db.settings.default_planned_notification = true;
        db.settings.default_latest_start_notification = true;
        db.settings.default_due_notification = true;
        let reference = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        );
        let mut item = task(
            "All defaults",
            Stamp::on(
                NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
                NaiveTime::from_hms_opt(10, 30, 0).unwrap(),
            ),
        );
        item.defer = Some(Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(9, 0, 0).unwrap(),
        ));
        item.planned = Some(Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(9, 30, 0).unwrap(),
        ));
        item.estimate_min = Some(30);
        db.add_task_to_inbox(item);

        let pending = reminders(&db, reference);
        assert_eq!(pending.len(), 4);
        assert!(pending.iter().any(|r| r.body.contains("Latest start")));
        assert!(pending.iter().any(|r| r.body.contains("Available")));
        assert!(pending.iter().any(|r| r.body.contains("Planned")));
        assert!(pending.iter().any(|r| r.body.contains("Due")));

        let task_id = db.tasks.keys().next().unwrap().clone();
        let task = db.tasks.get_mut(&task_id).unwrap();
        task.removed_notifications
            .insert(flowlist_core::model::DefaultNotification::Due);
        assert!(
            !reminders(&db, reference)
                .iter()
                .any(|r| r.body.contains("Due"))
        );
    }

    #[test]
    fn project_and_custom_notifications_are_scheduled() {
        let mut db = Db::new();
        db.settings.default_due_notification = true;
        let reference = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        );
        let mut project = Project::new("Notify project");
        project.due = Some(Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        ));
        project.custom_notifications.push(Stamp::date_only(
            NaiveDate::from_ymd_opt(2026, 11, 22).unwrap(),
        ));
        db.add_project(project);

        let pending = reminders(&db, reference);
        assert!(
            pending
                .iter()
                .any(|r| r.title == "Notify project" && r.body.contains("Due"))
        );
        let custom = pending.iter().find(|r| r.body.contains("Custom")).unwrap();
        assert_eq!(custom.at.with_timezone(&chrono::Local).hour(), 9);
    }

    #[test]
    fn reminders_skip_completed_trashed_and_closed_projects() {
        let mut db = Db::new();
        let reference = local_to_utc(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
        );
        let due = Stamp::on(
            NaiveDate::from_ymd_opt(2026, 11, 21).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );
        let mut completed = task("Completed", due);
        completed.completed = true;
        db.add_task_to_inbox(completed);
        let mut trashed = task("Trashed", due);
        trashed.trashed_at = Some(reference);
        db.add_task_to_inbox(trashed);

        let mut project = Project::new("Closed");
        project.status = ProjectStatus::Done;
        let project_id = db.add_project(project);
        let mut in_project = task("In closed project", due);
        in_project.project = Some(project_id);
        db.tasks.insert(in_project.id.clone(), in_project);

        assert!(reminders(&db, reference).is_empty());
    }
}
