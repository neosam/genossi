use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FileTreeEntry {
    #[serde(rename = "file")]
    File { name: String, path: String },
    #[serde(rename = "directory")]
    Directory {
        name: String,
        path: String,
        children: Vec<FileTreeEntry>,
    },
}

#[derive(Debug, Clone)]
pub enum TemplateError {
    NotFound,
    PathTraversal,
    DirectoryNotEmpty,
    IoError(Arc<str>),
    RenderError(Arc<str>),
}
