use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setting {
    pub id: String,
    pub label: String,
    pub source: String,
    pub category: String,
    pub group: String,
    pub file_kind: String,
    pub profile: String,
    pub pointer: String,
    pub value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanFile {
    pub id: String,
    pub source: String,
    pub path: String,
    pub file_kind: String,
    pub profile: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScanRequest {
    pub minecraft_root: Option<String>,
    pub lunar_root: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub settings: Vec<Setting>,
    pub files: Vec<ScanFile>,
    pub minecraft_detected: bool,
    pub lunar_detected: bool,
    pub minecraft_versions: Vec<String>,
    pub lunar_profiles: Vec<String>,
    pub warnings: Vec<String>,
    pub platform: String,
    pub running_processes: Vec<String>,
}

pub fn setting_id(source: &str, profile: &str, file_kind: &str, pointer: &str) -> String {
    format!("{}:{}:{}:{}", source, profile, file_kind, pointer)
}

pub fn file_id(source: &str, profile: &str, file_kind: &str) -> String {
    format!("{}:{}:{}", source, profile, file_kind)
}
