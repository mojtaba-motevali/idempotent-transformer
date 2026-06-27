use std::error::Error;

#[derive(Debug, Clone)]
pub struct Server {
    pub id: u64,
    pub addr_raft: String,
}

impl Server {
    /// Parse `"<id> <raft-addr>"` — the format used by `--node` CLI flags.
    pub fn parse(s: &str) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        match &parts[..] {
            [id, raft] => Ok(Server {
                id: id.parse::<u64>()?,
                addr_raft: raft.to_string(),
            }),
            _ => Err(format!("bad node spec: '{s}' (expected: id raft-addr)").into()),
        }
    }
}
