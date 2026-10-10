use serde::{Deserialize, Serialize};

// Content and source hints are private UI data, never Debug or telemetry fields.
#[derive(Clone, Deserialize, Serialize)]
pub struct Entry {
    pub id: u64,
    pub kind: String,
    pub mimes: Vec<String>,
    pub bytes: u64,
    pub preview: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub thumb: bool,
    pub pinned: bool,
    pub source: Option<String>,
    pub at: u64,
}

#[derive(Clone, Serialize)]
pub struct Page {
    pub items: Vec<Entry>,
    pub more: bool,
}
