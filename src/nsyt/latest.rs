use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LatestData {
    pub latest: String,
    pub latest_sol: u16,
    pub latest_sols: Vec<u16>,
    pub new_count: u16,
    pub sol_count: u16,
    pub total: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Latest {
    pub success: bool,
    pub latest_data: LatestData,
}
