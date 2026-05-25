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
    pub name: Option<String>,
    pub created_at: i64,
}

impl Workflow {
    pub fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            status: row.get(1)?,
            name: row.get(2)?,
            expire_at: row.get(3)?,
            completed_at: row.get(4)?,
            created_at: row.get(5)?,
        })
    }
}
