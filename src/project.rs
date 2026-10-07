//! Project file tree and open-buffer management.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdeProject {
    pub name: String,
    pub root: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileView {
    Text,
    Waveform,
}

#[derive(Clone, Debug)]
pub struct OpenFile {
    pub path: PathBuf,
    pub content: String,
    pub dirty: bool,
    pub cursor: usize,
    pub view: FileView,
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

/// The bundled Rust full-adder sample, if this tree has one.
pub fn locate_full_adder_sample() -> Option<PathBuf> {
    let samples = locate_samples_dir()?;
    let nested = samples.join("full_adder");
    if nested.join("Cargo.toml").is_file() && nested.join("src").join("adder.rs").is_file() {
        Some(nested)
    } else {
        None
    }
}

pub fn is_rust_source(path: &Path) -> bool {
    ext_is(path, &["rs"])
}

pub fn is_rust_testbench(path: &Path) -> bool {
    is_rust_source(path) && is_tb_stem(path)
}

/// A folder whose sources are Rust (a Cargo package with a `*_tb.rs`).
pub fn is_rust_project(root: &Path) -> bool {
    root.join("Cargo.toml").is_file() && collect_rust_files(root).iter().any(|p| is_rust_testbench(p))
}

/// Files to open when a folder is loaded: the Rust design then its testbench,
/// or the first Verilog source when the folder is a Verilog project.
pub fn initial_editor_files(root: &Path) -> Vec<PathBuf> {
    if is_rust_project(root) {
        let mut hdl = Vec::new();
        let mut tb = Vec::new();
        for path in collect_rust_files(root) {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if name == "lib.rs" || name == "main.rs" {
                continue;
            }
            if is_rust_testbench(&path) {
                tb.push(path);
            } else {
                hdl.push(path);
            }
        }
        hdl.sort();
        tb.sort();
        tb.extend(hdl);
        if !tb.is_empty() {
            return tb;
        }
    }
    find_first_verilog(root).into_iter().collect()
}

fn collect_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust(dir, &mut files);
    files.sort();
    files
}

fn collect_rust(dir: &Path, out: &mut Vec<PathBuf>) {
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
            collect_rust(&path, out);
        } else if is_rust_source(&path) {
            out.push(path);
        }
    }
}

fn is_tb_stem(path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    stem.ends_with("_tb")
        || stem.starts_with("tb_")
        || stem.ends_with("_testbench")
        || stem.ends_with("_test")
        || stem.contains("testbench")
}

fn ext_is(path: &Path, exts: &[&str]) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| {
            let lower = s.to_ascii_lowercase();
            exts.iter().any(|ext| lower == *ext)
        })
        .unwrap_or(false)
}

/// Compile units for simulation: `.v` / `.sv` only (headers are included).
pub fn collect_hdl_sources(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_verilog_files(root, &mut files);
    files.retain(|p| is_hdl_source(p));
    files.sort();
    files
}

/// Waveform dumps opened by Surfer/wellen (`.vcd` / `.fst` / `.ghw`).
pub fn is_wave_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("vcd" | "fst" | "ghw")
    )
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
        "target" | ".git" | "node_modules" | ".verilog-ide-data" | ".idea" | ".vscode" | "vendor"
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
        view: FileView::Text,
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
