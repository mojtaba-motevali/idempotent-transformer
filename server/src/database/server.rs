use std::error::Error;

#[derive(Debug, Clone)]
pub struct Server {
    pub id: u64,
    pub addr_api: String,
    pub addr_raft: String,
}

impl Server {
    pub fn parse(s: &str) -> Result<Self, Box<dyn Error>> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        match &parts[..] {
            [id, api, raft] => Ok(Server {
                id: id.parse::<u64>().unwrap(),
                addr_api: api.to_string(),
                addr_raft: raft.to_string(),
            }),
            _ => Err(format!("bad node spec: '{s}' (need 3 fields)").into()),
        }
    }
}
