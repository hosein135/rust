//! Run Verilog + testbench through [xezim](https://github.com/aionhw/xezim)
//! and collect the resulting VCD waveform.

use crate::project::{collect_hdl_sources, OpenFile};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const DUMP_WRAP_MODULE: &str = "__ide_vcd_dump";
const MAX_TIME: &str = "1ms";

#[derive(Debug, Clone)]
pub struct SimJob {
    pub xezim: PathBuf,
    pub root: PathBuf,
    pub args: Vec<String>,
    pub expected_vcd: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SimResult {
    pub ok: bool,
    pub log: String,
    pub vcd: Option<PathBuf>,
}

impl SimJob {
    pub fn command_preview(&self) -> String {
        let mut s = quote_arg(&self.xezim.display().to_string());
        for a in &self.args {
            s.push(' ');
            s.push_str(&quote_arg(a));
        }
        s
    }
}

pub fn prepare_job(
    root: &Path,
    active: Option<&Path>,
    open: &[OpenFile],
) -> Result<SimJob, String> {
    let xezim = find_xezim()?;
    let sources = collect_hdl_sources(root);
    if sources.is_empty() {
        return Err("No Verilog sources (.v / .sv) in this folder.".into());
    }

    let tb_path = pick_testbench(&sources, active, open)
        .ok_or_else(|| {
            "No testbench found. Add a *_tb.v (or *_tb.sv) file, or a module that calls $dumpfile / $finish."
                .to_string()
        })?;

    let tb_src = std::fs::read_to_string(&tb_path)
        .map_err(|e| format!("Read {}: {e}", tb_path.display()))?;
    let top = pick_top_module(&tb_path, &tb_src).ok_or_else(|| {
        format!(
            "Could not find a module in testbench {}",
            tb_path.display()
        )
    })?;

    let mut include_dirs = Vec::new();
    include_dirs.push(root.to_path_buf());
    for src in &sources {
        if let Some(parent) = src.parent() {
            if !include_dirs.iter().any(|d| d == parent) {
                include_dirs.push(parent.to_path_buf());
            }
        }
    }

    let mut extra_sources = Vec::new();
    let expected_vcd = if let Some(name) = parse_dumpfile(&tb_src) {
        resolve_vcd_path(root, &name)
    } else {
        let vcd_name = format!("{top}.vcd");
        let vcd_path = root.join(&vcd_name);
        extra_sources.push(write_dump_wrap(root, &vcd_name)?);
        vcd_path
    };

    let mut args = vec![
        "--wave".into(),
        "--simulate".into(),
        "--error-exit".into(),
        "--max-time".into(),
        MAX_TIME.into(),
        "-s".into(),
        top.clone(),
    ];
    if extra_sources.iter().any(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s == DUMP_WRAP_MODULE)
    }) {
        args.push("-s".into());
        args.push(DUMP_WRAP_MODULE.into());
    }
    for dir in &include_dirs {
        args.push("-I".into());
        args.push(dir.to_string_lossy().into_owned());
    }
    for src in &sources {
        args.push(src.to_string_lossy().into_owned());
    }
    for src in &extra_sources {
        args.push(src.to_string_lossy().into_owned());
    }

    Ok(SimJob {
        xezim,
        root: root.to_path_buf(),
        args,
        expected_vcd,
    })
}

pub async fn run_job_async(job: SimJob) -> SimResult {
    tokio::task::spawn_blocking(move || run_job(&job))
        .await
        .unwrap_or_else(|e| SimResult {
            ok: false,
            log: format!("Simulation task failed: {e}\n"),
            vcd: None,
        })
}

fn run_job(job: &SimJob) -> SimResult {
    let mut log = String::new();
    log.push_str(&format!("cwd: {}\n", job.root.display()));

    let mut cmd = Command::new(&job.xezim);
    cmd.args(&job.args)
        .current_dir(&job.root)
        .env("CARGO_TERM_COLOR", "never");

    let output = match cmd.output() {
        Ok(o) => o,
        Err(e) => {
            log.push_str(&format!("failed to spawn xezim: {e}\n"));
            return SimResult {
                ok: false,
                log,
                vcd: None,
            };
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stdout.is_empty() {
        log.push_str(&stdout);
        if !stdout.ends_with('\n') {
            log.push('\n');
        }
    }
    if !stderr.is_empty() {
        log.push_str(&stderr);
        if !stderr.ends_with('\n') {
            log.push('\n');
        }
    }

    let ok = output.status.success();
    if !ok {
        log.push_str(&format!(
            "xezim exited with {}\n",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".into())
        ));
    }

    let vcd = find_vcd(&job.root, &job.expected_vcd);
    match &vcd {
        Some(path) => log.push_str(&format!("VCD written: {}\n", path.display())),
        None if ok => log.push_str(
            "xezim finished but no .vcd was found. The testbench needs $dumpfile / $dumpvars, and the run must pass --wave (already requested).\n",
        ),
        None => {}
    }

    SimResult { ok, log, vcd }
}

pub fn find_xezim() -> Result<PathBuf, String> {
    let mut candidates = Vec::new();

    for key in ["XEZIM", "VERILOG_IDE_XEZIM"] {
        if let Ok(val) = std::env::var(key) {
            if !val.is_empty() {
                candidates.push(PathBuf::from(val));
            }
        }
    }

    candidates.extend(path_lookup("xezim"));

    if let Some(home) = home_dir() {
        candidates.push(home.join(".cargo").join("bin").join(exe_name("xezim")));
        let cache = cache_root(&home);
        candidates.push(cache.join("xezim-build").join("bin").join(exe_name("xezim")));
        candidates.push(cache.join("bin").join(exe_name("xezim")));
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(
            cwd.join("..")
                .join("xezim")
                .join("target")
                .join("release")
                .join(exe_name("xezim")),
        );
    }

    if let Some(found) = candidates.into_iter().find(|p| p.is_file()) {
        return Ok(found);
    }

    Err(
        "xezim not found. Install https://github.com/aionhw/xezim (`cargo build --release`) \
         and put it on PATH, or set XEZIM to the binary. On Linux / macOS / WSL, ./run.sh \
         provides xezim via Nix (first Run compiles it)."
            .into(),
    )
}

fn pick_testbench(
    sources: &[PathBuf],
    active: Option<&Path>,
    open: &[OpenFile],
) -> Option<PathBuf> {
    let scored = |path: &Path, content: &str| -> i32 {
        let mut score = 0;
        if is_testbench_filename(path) {
            score += 8;
        }
        if content.contains("$dumpfile") || content.contains("$dumpvars") {
            score += 6;
        }
        if content.contains("$finish") {
            score += 3;
        }
        if module_names(content)
            .iter()
            .any(|m| is_testbench_ident(m))
        {
            score += 4;
        }
        score
    };

    if let Some(active) = active {
        if sources.iter().any(|s| s == active) {
            let content = open
                .iter()
                .find(|f| f.path == active)
                .map(|f| f.content.clone())
                .or_else(|| std::fs::read_to_string(active).ok())
                .unwrap_or_default();
            if scored(active, &content) > 0 {
                return Some(active.to_path_buf());
            }
        }
    }

    let mut best: Option<(i32, PathBuf)> = None;
    for path in sources {
        let content = open
            .iter()
            .find(|f| f.path == *path)
            .map(|f| f.content.clone())
            .or_else(|| std::fs::read_to_string(path).ok())
            .unwrap_or_default();
        let score = scored(path, &content);
        if score <= 0 {
            continue;
        }
        if best.as_ref().is_none_or(|(s, _)| score > *s) {
            best = Some((score, path.clone()));
        }
    }
    best.map(|(_, p)| p)
}

fn pick_top_module(path: &Path, content: &str) -> Option<String> {
    let names = module_names(content);
    if names.is_empty() {
        return None;
    }
    let stem = path.file_stem()?.to_str()?;
    if names.iter().any(|n| n == stem) {
        return Some(stem.to_string());
    }
    names
        .iter()
        .rev()
        .find(|n| is_testbench_ident(n))
        .cloned()
        .or_else(|| names.last().cloned())
}

pub fn is_testbench_filename(path: impl AsRef<Path>) -> bool {
    let name = path
        .as_ref()
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(&name);
    is_testbench_ident(stem)
}

pub fn is_testbench_ident(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with("_tb")
        || n.starts_with("tb_")
        || n.ends_with("_testbench")
        || n.ends_with("_test")
        || n.contains("testbench")
}

fn module_names(src: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut search_from = 0;
    while let Some(rel) = src[search_from..].find("module") {
        let idx = search_from + rel;
        let prev = if idx == 0 {
            '\n'
        } else {
            src[..idx].chars().next_back().unwrap_or('\n')
        };
        if prev.is_ascii_alphanumeric() || prev == '_' || prev == '$' {
            search_from = idx + 6;
            continue;
        }
        let after = src[idx + 6..].trim_start();
        let name: String = after
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            names.push(name);
        }
        search_from = idx + 6;
    }
    names
}

fn parse_dumpfile(src: &str) -> Option<String> {
    let key = "$dumpfile";
    let idx = src.find(key)?;
    let rest = src[idx + key.len()..].trim_start();
    let rest = rest.strip_prefix('(')?.trim_start();
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = rest.get(1..)?;
    let end = body.find(quote)?;
    let name = body[..end].trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn resolve_vcd_path(root: &Path, name: &str) -> PathBuf {
    let p = PathBuf::from(name);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn write_dump_wrap(root: &Path, vcd_name: &str) -> Result<PathBuf, String> {
    let dir = root.join(".verilog-ide-data");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{DUMP_WRAP_MODULE}.sv"));
    let vcd_lit = vcd_name.replace('\\', "/").replace('"', "");
    let body = format!(
        r#"`timescale 1ns / 1ps
// Generated by Verilog IDE so xezim can write a VCD (--wave).
module {DUMP_WRAP_MODULE};
    initial begin
        $dumpfile("{vcd_lit}");
        $dumpvars(0);
    end
endmodule
"#
    );
    std::fs::write(&path, body).map_err(|e| format!("Write {}: {e}", path.display()))?;
    Ok(path)
}

fn find_vcd(root: &Path, expected: &Path) -> Option<PathBuf> {
    if vcd_looks_valid(expected) {
        return Some(expected.to_path_buf());
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return None;
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("vcd") && vcd_looks_valid(&path) {
            found.push(path);
        }
    }
    found.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .unwrap_or(Duration::from_secs(u64::MAX))
    });
    found.into_iter().next()
}

fn vcd_looks_valid(path: &Path) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() || meta.len() == 0 {
        return false;
    }
    let Ok(head) = std::fs::read_to_string(path) else {
        return meta.len() > 16;
    };
    head.contains("$timescale") || head.contains("$var") || head.contains("$date")
}

fn path_lookup(name: &str) -> Vec<PathBuf> {
    let exe = exe_name(name);
    let Some(paths) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    std::env::split_paths(&paths)
        .map(|dir| dir.join(&exe))
        .filter(|p| p.is_file())
        .collect()
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) && !name.ends_with(".exe") {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn cache_root(home: &Path) -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("verilog-ide");
        }
    }
    home.join(".cache").join("verilog-ide")
}

fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '\'')) {
        format!("\"{}\"", arg.replace('"', "\\\""))
    } else {
        arg.to_string()
    }
}
