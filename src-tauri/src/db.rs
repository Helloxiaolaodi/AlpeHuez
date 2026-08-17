use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: i64,
    pub name: String,
    pub role: String,
    pub rider_type: String,
    pub rider_name: String,
    pub rider_number: i64,
    pub specialties: Value,
    pub sort_order: i64,
    pub created_at: String,
}

pub fn open(path: &Path) -> Result<Connection, String> {
    Connection::open(path).map_err(|e| e.to_string())
}

pub fn init(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            role TEXT NOT NULL,
            rider_type TEXT NOT NULL,
            rider_name TEXT NOT NULL DEFAULT '',
            rider_number INTEGER NOT NULL DEFAULT 0,
            specialties TEXT NOT NULL DEFAULT '{\"gc\":0,\"climber\":0,\"sprint\":0,\"tt\":0}',
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS workspace_links (
            workspace_id INTEGER PRIMARY KEY,
            links_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS app_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );",
    )
    .map_err(|e| e.to_string())?;
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version < 1 {
        conn.execute_batch("PRAGMA user_version = 1")
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn seed(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM workspaces", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count > 0 {
        return Ok(());
    }
    let seeds: [(&str, &str, &str, i64); 5] = [
        ("主将", "leader", "GC Contender", 1),
        ("平路副将", "domestique", "Rouleur", 2),
        ("山地副将", "domestique", "Climber", 3),
        ("古典赛副将", "domestique", "Puncheur", 4),
        ("带冲手", "domestique", "Lead-out Man", 5),
    ];
    for (i, (name, role, rider_type, num)) in seeds.iter().enumerate() {
        conn.execute(
            "INSERT INTO workspaces (name, role, rider_type, rider_name, rider_number, sort_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![name, role, rider_type, name, num, i as i64, "2026-08-17"],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn list_workspaces(conn: &Connection) -> Result<Vec<Workspace>, String> {
    let mut stmt = conn
        .prepare("SELECT id, name, role, rider_type, rider_name, rider_number, specialties, sort_order, created_at FROM workspaces ORDER BY sort_order")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let specialties: String = row.get(6)?;
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                rider_type: row.get(3)?,
                rider_name: row.get(4)?,
                rider_number: row.get(5)?,
                specialties: serde_json::from_str(&specialties).unwrap_or_else(|_| serde_json::json!({})),
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_workspace(conn: &Connection, id: i64) -> Result<Workspace, String> {
    conn.query_row(
        "SELECT id, name, role, rider_type, rider_name, rider_number, specialties, sort_order, created_at FROM workspaces WHERE id = ?1",
        params![id],
        |row| {
            let specialties: String = row.get(6)?;
            Ok(Workspace {
                id: row.get(0)?,
                name: row.get(1)?,
                role: row.get(2)?,
                rider_type: row.get(3)?,
                rider_name: row.get(4)?,
                rider_number: row.get(5)?,
                specialties: serde_json::from_str(&specialties).unwrap_or_else(|_| serde_json::json!({})),
                sort_order: row.get(7)?,
                created_at: row.get(8)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

pub fn create_workspace(
    conn: &Connection,
    name: &str,
    role: &str,
    rider_type: &str,
    rider_name: &str,
    rider_number: i64,
) -> Result<Workspace, String> {
    let max_order: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), 0) FROM workspaces", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO workspaces (name, role, rider_type, rider_name, rider_number, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![name, role, rider_type, rider_name, rider_number, max_order + 1, "2026-08-17"],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_workspace(conn, id)
}

pub fn update_workspace(
    conn: &Connection,
    id: i64,
    name: &str,
    role: &str,
    rider_type: &str,
    rider_name: &str,
    rider_number: i64,
    specialties: &Value,
) -> Result<(), String> {
    let specialties_str = serde_json::to_string(specialties).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE workspaces SET name = ?1, role = ?2, rider_type = ?3, rider_name = ?4, rider_number = ?5, specialties = ?6 WHERE id = ?7",
        params![name, role, rider_type, rider_name, rider_number, specialties_str, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_workspace(conn: &Connection, id: i64) -> Result<(), String> {
    let role: String = conn
        .query_row("SELECT role FROM workspaces WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if role == "leader" {
        return Err("主将工作台不可删除".into());
    }
    conn.execute("DELETE FROM workspace_links WHERE workspace_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM workspaces WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_workspace_links(conn: &Connection, id: i64) -> Result<Value, String> {
    let links: Option<String> = conn
        .query_row("SELECT links_json FROM workspace_links WHERE workspace_id = ?1", params![id], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    match links {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(serde_json::json!({ "icons": [] })),
    }
}

pub fn save_workspace_links(conn: &Connection, id: i64, data: &Value) -> Result<(), String> {
    let json = serde_json::to_string(data).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO workspace_links (workspace_id, links_json) VALUES (?1, ?2)
         ON CONFLICT(workspace_id) DO UPDATE SET links_json = excluded.links_json",
        params![id, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn get_config(conn: &Connection, key: &str) -> Result<Value, String> {
    let val: Option<String> = conn
        .query_row("SELECT value FROM app_config WHERE key = ?1", params![key], |r| r.get(0))
        .optional()
        .map_err(|e| e.to_string())?;
    match val {
        Some(s) => serde_json::from_str(&s).map_err(|e| e.to_string()),
        None => Ok(Value::Null),
    }
}

pub fn set_config(conn: &Connection, key: &str, value: &Value) -> Result<(), String> {
    let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO app_config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("内存数据库");
        init(&conn).expect("init");
        conn
    }

    #[test]
    fn seed_creates_five_workspaces() {
        let conn = test_conn();
        seed(&conn).expect("seed");
        let ws = list_workspaces(&conn).expect("list");
        assert_eq!(ws.len(), 5);
        assert_eq!(ws[0].role, "leader");
        assert_eq!(ws[0].name, "主将");
        assert_eq!(ws[1].rider_type, "Rouleur");
    }

    #[test]
    fn seed_is_idempotent() {
        let conn = test_conn();
        seed(&conn).expect("seed");
        seed(&conn).expect("seed again");
        assert_eq!(list_workspaces(&conn).expect("list").len(), 5);
    }

    #[test]
    fn create_and_update_workspace() {
        let conn = test_conn();
        let ws = create_workspace(&conn, "测试副将", "domestique", "Sprinter", "测试", 6).expect("create");
        assert_eq!(ws.name, "测试副将");
        update_workspace(&conn, ws.id, "改名", "domestique", "Sprinter", "测试", 6, &serde_json::json!({"gc": 10, "climber": 20, "sprint": 30, "tt": 40})).expect("update");
        let updated = get_workspace(&conn, ws.id).expect("get");
        assert_eq!(updated.name, "改名");
        assert_eq!(updated.specialties["sprint"], 30);
    }

    #[test]
    fn delete_leader_is_rejected() {
        let conn = test_conn();
        seed(&conn).expect("seed");
        let leader = list_workspaces(&conn).expect("list").into_iter().find(|w| w.role == "leader").unwrap();
        assert!(delete_workspace(&conn, leader.id).is_err());
    }

    #[test]
    fn links_and_config_roundtrip() {
        let conn = test_conn();
        let data = serde_json::json!({ "icons": [{ "title": "x", "url": "https://x.com" }] });
        save_workspace_links(&conn, 1, &data).expect("save links");
        assert_eq!(get_workspace_links(&conn, 1).expect("get links"), data);
        set_config(&conn, "active_workspace", &serde_json::json!(1)).expect("set config");
        assert_eq!(get_config(&conn, "active_workspace").expect("get config"), serde_json::json!(1));
    }
}
