//! Schema versioning and migrations.
//!
//! Documents are parsed as `serde_json::Value` first, so a future schema can
//! be rejected with a helpful message and old documents can be upgraded before
//! strongly-typed deserialization.

use serde_json::{Map, Value, json};

use crate::error::{Error, Result};
use crate::persist::{DataFile, SCHEMA_VERSION};
use crate::time::now;

pub fn prepare(mut value: Value) -> Result<DataFile> {
    // A bare database document (pre-release builds) is wrapped automatically.
    if value.get("db").is_none() && value.get("tasks").is_some() {
        value = json!({
            "schema_version": 0,
            "revision": 1,
            "updated_at": now(),
            "db": value,
        });
    }

    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0) as u32;

    if version > SCHEMA_VERSION {
        return Err(Error::SchemaTooNew {
            found: version,
            supported: SCHEMA_VERSION,
        });
    }

    if version < 1 {
        migrate_0_to_1(&mut value)?;
    }

    let mut file: DataFile = serde_json::from_value(value)?;
    file.schema_version = SCHEMA_VERSION;
    Ok(file)
}

/// v0 was an unversioned document missing settings, custom perspectives and
/// the tag roots array. v1 adds the review/settings envelope.
fn migrate_0_to_1(value: &mut Value) -> Result<()> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| Error::Import("data file root must be an object".into()))?;
    object.insert("schema_version".to_string(), json!(1));
    object.entry("revision").or_insert(json!(1));
    object.entry("updated_at").or_insert(json!(now()));

    let db = object
        .entry("db")
        .or_insert_with(|| Value::Object(Map::new()));
    let db_object = db
        .as_object_mut()
        .ok_or_else(|| Error::Import("db field must be an object".into()))?;
    db_object
        .entry("tasks")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object
        .entry("projects")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object
        .entry("folders")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object
        .entry("tags")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object
        .entry("perspectives")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object.entry("root").or_insert_with(|| json!([]));
    db_object.entry("tag_roots").or_insert_with(|| json!([]));
    db_object.entry("inbox_order").or_insert_with(|| json!([]));
    db_object
        .entry("settings")
        .or_insert_with(|| Value::Object(Map::new()));
    db_object
        .entry("created_at")
        .or_insert_with(|| json!(now()));
    db_object
        .entry("modified_at")
        .or_insert_with(|| json!(now()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_database_is_wrapped() {
        let value = json!({
            "tasks": {},
            "projects": {},
            "folders": {},
            "tags": {},
            "root": [],
            "inbox_order": [],
        });
        let file = prepare(value).unwrap();
        assert_eq!(file.schema_version, SCHEMA_VERSION);
        assert_eq!(file.revision, 1);
    }

    #[test]
    fn future_schema_is_rejected() {
        let value = json!({"schema_version": 99, "db": {}});
        assert!(matches!(
            prepare(value),
            Err(Error::SchemaTooNew { found: 99, .. })
        ));
    }

    #[test]
    fn v0_document_gains_settings() {
        let value = json!({
            "schema_version": 0,
            "revision": 3,
            "db": {"tasks": {}, "projects": {}, "root": []},
        });
        let file = prepare(value).unwrap();
        assert_eq!(file.revision, 3);
        assert!(file.db.settings.default_review_days >= 1);
    }
}
