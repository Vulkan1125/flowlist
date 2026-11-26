//! Read-only macOS Calendar events for Forecast.
//!
//! EventKit is only available on macOS. The rest of the app talks to this
//! module through a small, platform-neutral event type, and GUI probes can use
//! a deterministic fixture through `FLOWLIST_TEST_CALENDAR`.

use std::sync::atomic::{AtomicU8, Ordering};

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};

use flowlist_core::time::{local_date, local_to_utc, now};

#[cfg(target_os = "macos")]
#[link(name = "EventKit", kind = "framework")]
unsafe extern "C" {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Unknown,
    NotDetermined,
    Denied,
    Restricted,
    Authorized,
}

impl Permission {
    pub fn allowed(self) -> bool {
        self == Permission::Authorized
    }

    pub fn label(self) -> &'static str {
        match self {
            Permission::Unknown => "Checking…",
            Permission::NotDetermined => "Not asked yet",
            Permission::Denied => "Denied in System Settings",
            Permission::Restricted => "Restricted by this Mac",
            Permission::Authorized => "Allowed",
        }
    }
}

static PERMISSION: AtomicU8 = AtomicU8::new(0);

fn store_status(status: isize) {
    let value = match status {
        0 => 1, // not determined
        1 => 4, // restricted
        2 => 2, // denied
        3 => 3, // full access
        _ => 0,
    };
    PERMISSION.store(value, Ordering::Relaxed);
}

/// The calendars and events are read-only. We never write to the user's
/// Calendar database.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct CalendarInfo {
    pub id: String,
    pub title: String,
    pub color: Option<u32>,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Event {
    pub id: String,
    pub title: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub all_day: bool,
    pub calendar: String,
    pub calendar_id: String,
    pub color: Option<u32>,
    pub location: Option<String>,
}

impl Event {
    pub fn local_date(&self) -> NaiveDate {
        local_date(self.start)
    }

    /// A compact label for the agenda and month grid.
    pub fn time_label(&self) -> String {
        if self.all_day {
            "All day".to_string()
        } else {
            let start = self.start.with_timezone(&chrono::Local);
            let end = self.end.with_timezone(&chrono::Local);
            format!("{} – {}", start.format("%H:%M"), end.format("%H:%M"))
        }
    }

    /// A short label for a month-grid chip.
    pub fn chip_label(&self) -> String {
        if self.all_day {
            self.title.clone()
        } else {
            format!(
                "{} {}",
                self.start
                    .with_timezone(&chrono::Local)
                    .format("%H:%M"),
                self.title
            )
        }
    }
}

pub fn permission() -> Permission {
    if std::env::var_os("FLOWLIST_TEST_CALENDAR").is_some() {
        return Permission::Authorized;
    }
    match PERMISSION.load(Ordering::Relaxed) {
        1 => Permission::NotDetermined,
        2 => Permission::Denied,
        3 => Permission::Authorized,
        4 => Permission::Restricted,
        _ => Permission::Unknown,
    }
}

pub fn refresh_permission() {
    #[cfg(target_os = "macos")]
    unsafe {
        use objc::{class, msg_send, sel, sel_impl};

        let status: isize = msg_send![class!(EKEventStore), authorizationStatusForEntityType: 0isize];
        store_status(status);
    }
}

pub fn request_permission() {
    #[cfg(target_os = "macos")]
    unsafe {
        use block::ConcreteBlock;
        use objc::runtime::Object;
        use objc::{class, msg_send, sel, sel_impl};

        let store: *mut Object = msg_send![class!(EKEventStore), alloc];
        let store: *mut Object = msg_send![store, init];
        if store.is_null() {
            return;
        }
        let callback = ConcreteBlock::new(|granted: i8, _error: *mut Object| {
            PERMISSION.store(if granted != 0 { 3 } else { 2 }, Ordering::Relaxed);
        });
        let callback = callback.copy();
        let responds: bool = msg_send![
            store,
            respondsToSelector: sel!(requestFullAccessToEventsWithCompletion:)
        ];
        if responds {
            let _: () = msg_send![
                store,
                requestFullAccessToEventsWithCompletion: &*callback
            ];
        } else {
            let _: () = msg_send![
                store,
                requestAccessToEntityType: 0isize
                completion: &*callback
            ];
        }
        // EventKit keeps the block; the store must outlive the callback.
        let _ = callback;
    }
}

/// Events whose local start date falls inside `[from, to]`, sorted with
/// all-day events first and then by start time.
pub fn calendars() -> Vec<CalendarInfo> {
    if std::env::var_os("FLOWLIST_TEST_CALENDAR").is_some() {
        return test_calendars();
    }
    if !permission().allowed() {
        return Vec::new();
    }
    #[cfg(target_os = "macos")]
    unsafe {
        return macos_calendars();
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

#[cfg(target_os = "macos")]
unsafe fn macos_calendars() -> Vec<CalendarInfo> {
    use std::ffi::CStr;
    use std::os::raw::c_char;
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    fn string(ptr: *mut Object) -> Option<String> {
        if ptr.is_null() { return None; }
        unsafe {
            let text: *const c_char = msg_send![ptr, UTF8String];
            if text.is_null() { None } else {
                Some(CStr::from_ptr(text).to_string_lossy().into_owned())
            }
        }
    }

    let store: *mut Object = msg_send![class!(EKEventStore), alloc];
    let store: *mut Object = msg_send![store, init];
    if store.is_null() { return Vec::new(); }
    let raw: *mut Object = msg_send![store, calendarsForEntityType: 0isize];
    let mut out = Vec::new();
    if !raw.is_null() {
        let count: usize = msg_send![raw, count];
        for index in 0..count {
            let calendar: *mut Object = msg_send![raw, objectAtIndex: index];
            if calendar.is_null() { continue; }
            let id = string(msg_send![calendar, calendarIdentifier]).unwrap_or_default();
            let title = string(msg_send![calendar, title]).unwrap_or_else(|| "Calendar".to_string());
            let source: *mut Object = msg_send![calendar, source];
            let source = if source.is_null() { None } else { string(msg_send![source, title]) };
            out.push(CalendarInfo { id, title, color: event_color(calendar), source });
        }
    }
    let _: () = msg_send![store, release];
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

pub fn events(from: NaiveDate, to: NaiveDate) -> Vec<Event> {
    if std::env::var_os("FLOWLIST_TEST_CALENDAR").is_some() {
        return test_events(from, to);
    }
    if !permission().allowed() {
        return Vec::new();
    }
    #[cfg(target_os = "macos")]
    unsafe {
        return macos_events(from, to);
    }
    #[cfg(not(target_os = "macos"))]
    Vec::new()
}

#[cfg(target_os = "macos")]
unsafe fn macos_events(from: NaiveDate, to: NaiveDate) -> Vec<Event> {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    fn string(ptr: *mut Object) -> Option<String> {
        if ptr.is_null() {
            return None;
        }
        unsafe {
            let text: *const c_char = msg_send![ptr, UTF8String];
            if text.is_null() {
                None
            } else {
                Some(CStr::from_ptr(text).to_string_lossy().into_owned())
            }
        }
    }

    fn date(ptr: *mut Object) -> Option<DateTime<Utc>> {
        if ptr.is_null() {
            return None;
        }
        unsafe {
            let seconds: f64 = msg_send![ptr, timeIntervalSince1970];
            DateTime::<Utc>::from_timestamp(seconds as i64, 0)
        }
    }

    let start = local_to_utc(from, NaiveTime::MIN);
    let end = local_to_utc(to + Duration::days(1), NaiveTime::MIN);
    let start_date: *mut Object = msg_send![
        class!(NSDate),
        dateWithTimeIntervalSince1970: start.timestamp() as f64
    ];
    let end_date: *mut Object = msg_send![
        class!(NSDate),
        dateWithTimeIntervalSince1970: end.timestamp() as f64
    ];

    let store: *mut Object = msg_send![class!(EKEventStore), alloc];
    let store: *mut Object = msg_send![store, init];
    if store.is_null() {
        return Vec::new();
    }
    let predicate: *mut Object = msg_send![
        store,
        predicateForEventsWithStartDate: start_date
        endDate: end_date
        calendars: std::ptr::null::<Object>()
    ];
    let raw_events: *mut Object = msg_send![store, eventsMatchingPredicate: predicate];
    let mut events = Vec::new();
    if !raw_events.is_null() {
        let count: usize = msg_send![raw_events, count];
        for index in 0..count {
            let event: *mut Object = msg_send![raw_events, objectAtIndex: index];
            if event.is_null() {
                continue;
            }
            let Some(start) = date(msg_send![event, startDate]) else {
                continue;
            };
            if local_date(start) < from || local_date(start) > to {
                continue;
            }
            let end = date(msg_send![event, endDate])
                .filter(|end| *end >= start)
                .unwrap_or(start);
            let calendar: *mut Object = msg_send![event, calendar];
            let calendar_title = string(msg_send![calendar, title])
                .unwrap_or_else(|| "Calendar".to_string());
            let calendar_id = string(msg_send![calendar, calendarIdentifier]).unwrap_or_default();
            let color = event_color(calendar);
            events.push(Event {
                id: string(msg_send![event, eventIdentifier])
                    .unwrap_or_else(|| format!("event:{index}")),
                title: string(msg_send![event, title]).unwrap_or_else(|| "Untitled event".to_string()),
                start,
                end,
                all_day: msg_send![event, isAllDay],
                calendar: calendar_title,
                calendar_id,
                color,
                location: string(msg_send![event, location]),
            });
        }
    }
    let _: () = msg_send![store, release];
    events.sort_by_key(|event| (local_date(event.start), !event.all_day, event.start, event.title.to_lowercase()));
    events
}

#[cfg(target_os = "macos")]
unsafe fn event_color(calendar: *mut objc::runtime::Object) -> Option<u32> {
    use objc::runtime::Object;
    use objc::{msg_send, sel, sel_impl};

    if calendar.is_null() {
        return None;
    }
    let color: *mut Object = msg_send![calendar, color];
    if color.is_null() {
        return None;
    }
    let red: f64 = msg_send![color, redComponent];
    let green: f64 = msg_send![color, greenComponent];
    let blue: f64 = msg_send![color, blueComponent];
    let to_byte = |value: f64| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    Some((to_byte(red) << 16) | (to_byte(green) << 8) | to_byte(blue))
}

fn test_calendars() -> Vec<CalendarInfo> {
    vec![
        CalendarInfo { id: "work".into(), title: "Work".into(), color: Some(0x3B82F6), source: Some("iCloud".into()) },
        CalendarInfo { id: "personal".into(), title: "Personal".into(), color: Some(0xF59E0B), source: Some("iCloud".into()) },
        CalendarInfo { id: "holidays".into(), title: "Holidays".into(), color: Some(0xEF4444), source: Some("Holidays".into()) },
        CalendarInfo { id: "birthdays".into(), title: "Birthdays".into(), color: Some(0xA855F7), source: Some("Birthdays".into()) },
    ]
}

fn test_events(from: NaiveDate, to: NaiveDate) -> Vec<Event> {
    let today = local_date(now());
    let tomorrow = today + Duration::days(1);
    let at = |date: NaiveDate, hour: u32, minute: u32| {
        local_to_utc(
            date,
            NaiveTime::from_hms_opt(hour, minute, 0).unwrap_or(NaiveTime::MIN),
        )
    };
    let all = vec![
        Event {
            id: "test-standup".to_string(),
            title: "Team standup".to_string(),
            start: at(today, 9, 0),
            end: at(today, 9, 30),
            all_day: false,
            calendar: "Work".to_string(),
            calendar_id: "work".to_string(),
            color: Some(0x3B82F6),
            location: Some("Meeting room".to_string()),
        },
        Event {
            id: "test-holiday".to_string(),
            title: "Holiday".to_string(),
            start: at(today, 0, 0),
            end: at(tomorrow, 0, 0),
            all_day: true,
            calendar: "Holidays".to_string(),
            calendar_id: "holidays".to_string(),
            color: Some(0xEF4444),
            location: None,
        },
        Event {
            id: "test-birthday".to_string(),
            title: "Alex's birthday".to_string(),
            start: at(today, 0, 0),
            end: at(tomorrow, 0, 0),
            all_day: true,
            calendar: "Birthdays".to_string(),
            calendar_id: "birthdays".to_string(),
            color: Some(0xA855F7),
            location: None,
        },
        Event {
            id: "test-review".to_string(),
            title: "Design review".to_string(),
            start: at(tomorrow, 14, 0),
            end: at(tomorrow, 15, 0),
            all_day: false,
            calendar: "Work".to_string(),
            calendar_id: "work".to_string(),
            color: Some(0x3B82F6),
            location: None,
        },
    ];
    all.into_iter()
        .filter(|event| {
            let date = event.local_date();
            date >= from && date <= to
        })
        .collect()
}

pub fn open_settings() {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Calendars")
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_day_events_sort_first_and_have_a_plain_label() {
        let date = local_date(now());
        let mut events = test_events(date - Duration::days(1), date + Duration::days(1));
        events.sort_by_key(|event| (event.local_date(), !event.all_day, event.start));
        assert!(events.iter().any(|event| event.all_day));
        assert!(events.iter().any(|event| event.time_label() == "All day"));
    }

    #[test]
    fn range_filters_events_by_local_day() {
        let today = local_date(now());
        assert_eq!(test_events(today, today).len(), 3);
        assert_eq!(
            test_events(today + Duration::days(1), today + Duration::days(1)).len(),
            1
        );
    }
}
