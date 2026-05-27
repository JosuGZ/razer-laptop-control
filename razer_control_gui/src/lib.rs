//! This is duplicated stuff for now, until we have a proper project structure

use std::fs;

use serde::{Serialize, Deserialize};

const EMBEDDED_DEVICE_DATA: &str = include_str!("../data/devices/laptops.json");
const DEVICE_FILE: &str = "/usr/share/razercontrol/laptops.json";

pub fn get_device_data() -> String {
    fs::read_to_string(DEVICE_FILE)
        .unwrap_or(EMBEDDED_DEVICE_DATA.into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedDevice {
    pub name: String,
    pub vid: String,
    pub pid: String,
    pub features: Vec<String>,
    pub fan: Vec<u16>,
}

impl SupportedDevice {

    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    pub fn can_boost(&self) -> bool {
        self.has_feature("boost")
    }

    pub fn has_logo(&self) -> bool {
        self.has_feature("logo")
    }

}
