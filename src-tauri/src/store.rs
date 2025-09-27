use std::path::Path;
use tauri_plugin_sql::{Migration, MigrationKind};

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            description: "create_snapshots_table",
            sql: "CREATE TABLE snapshots (id INTEGER PRIMARY KEY, file_name TEXT NOT NULL, content TEXT NOT NULL, created_at TEXT DEFAULT CURRENT_TIMESTAMP);",
            kind: MigrationKind::Up,
        },
    ]
}

/// TODO! Add error handling
pub fn init(db_path: &Path) -> tauri_plugin_sql::Builder {
    let url = format!("sqlite:{}", db_path.to_str().unwrap());

    tauri_plugin_sql::Builder::default().add_migrations(&url, get_migrations())
}
