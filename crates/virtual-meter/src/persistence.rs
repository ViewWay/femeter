//! 状态持久化
//!
//! JSON 格式保存/加载电表状态

use crate::{EnergyData, EventRecord};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedState {
    pub energy: EnergyData,
    pub events: Vec<EventRecord>,
    pub load_profile_records: Vec<crate::load_profile::FreezeRecord>,
    pub max_demand_p: f64,
    pub max_demand_p_time: chrono::DateTime<chrono::Utc>,
    pub max_demand_q: f64,
    pub max_demand_q_time: chrono::DateTime<chrono::Utc>,
    pub statistics: crate::statistics::Statistics,
    pub calibration: crate::calibration::CalibrationParams,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            energy: EnergyData::default(),
            events: Vec::new(),
            load_profile_records: Vec::new(),
            max_demand_p: 0.0,
            max_demand_p_time: chrono::Utc::now(),
            max_demand_q: 0.0,
            max_demand_q_time: chrono::Utc::now(),
            statistics: crate::statistics::Statistics::default(),
            calibration: crate::calibration::CalibrationParams::default(),
            saved_at: chrono::Utc::now(),
        }
    }
}

pub struct Persistence {
    path: String,
}

impl Persistence {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub fn save(&self, state: &PersistedState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }

    pub fn load(&self) -> Result<PersistedState> {
        let json = std::fs::read_to_string(&self.path)?;
        let state: PersistedState = serde_json::from_str(&json)?;
        Ok(state)
    }

    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_load_roundtrip() {
        let dir = std::env::temp_dir().join("femeter_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("test_state.json");
        let path_str = path.to_str().unwrap();

        let mut state = PersistedState::default();
        state.energy.wh_total = 12345.678;
        state.saved_at = chrono::Utc::now();

        let p = Persistence::new(path_str);
        p.save(&state).unwrap();
        assert!(p.exists());

        let loaded = p.load().unwrap();
        assert!((loaded.energy.wh_total - 12345.678).abs() < 0.001);

        std::fs::remove_file(path).ok();
    }
}
