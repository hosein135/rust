//! Project file tree and open-buffer management.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

    pub fn refresh_tree(&self) -> TreeNode {
        self.build_tree()
    }
}

/// Locate the bundled `samples/` directory (repo or cwd).
pub fn locate_samples_dir() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(env) = std::env::var("VERILOG_IDE_SAMPLES_DIR") {
        if !env.is_empty() {
            candidates.push(PathBuf::from(env));
        }
    }

    candidates.push(PathBuf::from("samples"));

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("samples"));
        candidates.extend(walk_up(cwd).into_iter().map(|p| p.join("samples")));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("samples"));
            candidates.extend(walk_up(dir.to_path_buf()).into_iter().map(|p| p.join("samples")));
        }
    }

    candidates.into_iter().find(|p| {
        p.is_dir()
            && (p.join("counter.v").is_file()
                || p.read_dir()
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false))
    })
}

fn walk_up(mut start: PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    loop {
        if !start.pop() {
            break;
        }
        out.push(start.clone());
    }
    out
}

/// First Verilog source file under `root` (sorted by path).
pub fn find_first_verilog(root: &Path) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_verilog_files(root, &mut files);
    files.sort();
    files.into_iter().next()
}

/// Compile units for simulation: `.v` / `.sv` only (headers are included).
pub fn collect_hdl_sources(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_verilog_files(root, &mut files);
    files.retain(|p| is_hdl_source(p));
    files.sort();
    files
}

pub fn is_hdl_source(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("v" | "sv")
    )
}

fn collect_verilog_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if should_skip(&name) || name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect_verilog_files(&path, out);
        } else if is_verilog_file(&path) {
            out.push(path);
        }
    }
}

fn is_verilog_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("v" | "vh" | "sv" | "svh" | "vl")
    )
}

/// Collect all directory paths in a tree (for expand-all in explorer).
pub fn collect_dir_paths(node: &TreeNode, out: &mut Vec<PathBuf>) {
    if node.is_dir {
        out.push(node.path.clone());
        for child in &node.children {
            collect_dir_paths(child, out);
        }
    }
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
            } else {
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
