use herdr_agent_grid::{model::Agent, unread::UnreadMarks};
use std::process::Command;

fn agent(pane_id: &str, status: &str) -> Agent {
    Agent {
        pane_id: pane_id.into(),
        status: status.into(),
        ..Agent::default()
    }
}

fn sqlite(db: &std::path::Path, sql: &str) {
    let ok = Command::new("sqlite3")
        .arg(db)
        .arg(sql)
        .status()
        .unwrap()
        .success();
    assert!(ok, "sqlite3 failed: {sql}");
}

#[test]
fn marked_idle_panes_show_as_done() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("unread.sqlite3");
    sqlite(
        &db,
        "PRAGMA journal_mode=WAL; CREATE TABLE unread (session_key TEXT PRIMARY KEY, pane_id TEXT NOT NULL); \
         INSERT INTO unread VALUES ('a','w1:p1'),('b','w1:p2'),('c','w1:p3');",
    );
    let mut marks = UnreadMarks::new(db.clone());
    let mut agents = vec![
        agent("w1:p1", "idle"),
        agent("w1:p2", "working"),
        agent("w1:p3", "unknown"),
        agent("w1:p4", "idle"),
    ];
    marks.apply(&mut agents);
    let shown: Vec<_> = agents
        .iter()
        .map(|a| (a.status.as_str(), a.unread))
        .collect();
    assert_eq!(
        shown,
        [
            ("done", true),
            ("working", false),
            ("done", true),
            ("idle", false)
        ]
    );

    sqlite(&db, "DELETE FROM unread WHERE pane_id = 'w1:p1';");
    let mut agents = vec![agent("w1:p1", "idle")];
    marks.apply(&mut agents);
    assert_eq!(
        (agents[0].status.as_str(), agents[0].unread),
        ("idle", false)
    );
}

#[test]
fn missing_database_marks_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut marks = UnreadMarks::new(dir.path().join("absent.sqlite3"));
    let mut agents = vec![agent("w1:p1", "idle")];
    marks.apply(&mut agents);
    assert_eq!(agents[0].status, "idle");
}
