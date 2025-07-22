use hiqlite::Row;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Running = 0,
    Completed = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub status: i64,
    pub expire_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub created_at: i64,
}

impl From<Row<'_>> for Workflow {
    fn from(mut row: Row<'_>) -> Self {
        Self {
            id: row.get("id"),
            status: match row.get::<i64>("status") {
                0 => WorkflowStatus::Running,
                1 => WorkflowStatus::Completed,
                _ => panic!("Invalid workflow status"),
            } as i64,
            expire_at: row.get::<Option<i64>>("expire_at"),
            completed_at: row.get::<Option<i64>>("completed_at"),
            created_at: row.get("created_at"),
        }
    }
}
