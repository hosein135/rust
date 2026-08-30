//! Project file tree and open-buffer management.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdeProject {
    pub name: String,
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct OpenFile {
    pub path: PathBuf,
    pub content: String,
    pub dirty: bool,
    pub cursor: usize,
}

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub children: Vec<TreeNode>,
}

impl IdeProject {
    pub fn new(root: PathBuf) -> Self {
        let name = root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "project".into());
        Self { name, root }
    }

    pub fn build_tree(&self) -> TreeNode {
        build_dir_tree(&self.root)
    }

    pub fn list_verilog_files(&self) -> Vec<PathBuf> {
        WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.into_path())
            .filter(|p| is_verilog_path(p))
            .collect()
    }
}

pub fn is_verilog_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("v" | "vh" | "sv" | "svh" | "vl")
    )
}

fn should_skip(name: &str) -> bool {
    matches!(
        name,
        "target" | ".git" | "node_modules" | ".verilog-ide-data" | ".idea" | ".vscode"
    )
}

fn build_dir_tree(dir: &Path) -> TreeNode {
    let name = dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| dir.to_string_lossy().to_string());

    let mut children = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            (!is_dir, e.file_name().to_string_lossy().to_lowercase())
        });
        for entry in entries {
            let path = entry.path();
            let fname = entry.file_name().to_string_lossy().to_string();
            if should_skip(&fname) || fname.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                children.push(build_dir_tree(&path));
            } else if is_verilog_path(&path)
                || matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("md" | "toml" | "txt" | "json" | "cfg" | "do" | "tcl")
                )
            {
                children.push(TreeNode {
                    path,
                    name: fname,
                    is_dir: false,
                    children: Vec::new(),
                });
            }
        }
    }

    TreeNode {
        path: dir.to_path_buf(),
        name,
        is_dir: true,
        children,
    }
}

pub fn load_file(path: &Path) -> Result<OpenFile, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("Read {}: {e}", path.display()))?;
    Ok(OpenFile {
        path: path.to_path_buf(),
        content,
        dirty: false,
        cursor: 0,
    })
}

pub fn save_file(file: &mut OpenFile) -> Result<(), String> {
    if let Some(parent) = file.path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&file.path, &file.content)
        .map_err(|e| format!("Write {}: {e}", file.path.display()))?;
    file.dirty = false;
    Ok(())
}
