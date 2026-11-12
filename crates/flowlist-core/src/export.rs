//! JSON import/export. This is the lossless interchange format and is also
//! what `flowlist export` writes by default.

use crate::db::Db;
use crate::error::{Error, Result};
use crate::persist::DataFile;
use crate::persist::migrate;

pub fn to_json(db: &Db) -> Result<String> {
    let file = DataFile::new(db.clone());
    Ok(serde_json::to_string_pretty(&file)?)
}

/// Accepts a full data file (`{"db": ...}`), a versioned document, or a bare
/// database object (what older/other tools may produce).
pub fn from_json(text: &str) -> Result<Db> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let db = if value.get("db").is_some() || value.get("schema_version").is_some() {
        migrate::prepare(value)?.db
    } else {
        let db: Db = serde_json::from_value(value)
            .map_err(|error| Error::Import(format!("could not read library JSON: {error}")))?;
        db
    };
    db.validate()
        .map_err(|error| Error::Import(format!("imported library is invalid: {error}")))?;
    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Project, Task};

    #[test]
    fn json_roundtrips_a_library() {
        let mut db = Db::new();
        let project = db.add_project(Project::new("Home"));
        let mut task = Task::new("Clean kitchen");
        task.project = Some(project.clone());
        let id = task.id.clone();
        db.tasks.insert(id.clone(), task);
        db.projects
            .get_mut(&project)
            .unwrap()
            .task_order
            .push(id.clone());
        db.touch();
        db.validate().unwrap();

        let json = to_json(&db).unwrap();
        let restored = from_json(&json).unwrap();
        assert_eq!(restored.tasks.len(), 1);
        assert_eq!(restored.projects.len(), 1);
    }

    #[test]
    fn bare_database_is_accepted() {
        let json = serde_json::to_string(&Db::new()).unwrap();
        assert!(from_json(&json).is_ok());
    }
}
