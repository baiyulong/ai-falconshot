use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClipboardContent {
    Empty,
    Text,
    Image,
    Html,
    Color(String),
    FilePaths(Vec<String>),
    Unknown,
}
