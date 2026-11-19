//! The file size check (`alm guide file-size`, Task-A07-anchor134): a source file over the
//! project's line limit may only get shorter, and a new source file may not go over it.
//!
//! The baseline is `docs/workflow/file-size-baseline.txt`: its first line is the limit
//! (`limit <lines>`), then one `<path> <lines>` line for each source file over the limit.
//! Tests do not count: in Rust, everything from the `#[cfg(test)]` that opens a test module
//! (`mod … {`) to the end of the file; in the web app, `*.test.*` and `*.spec.*` files and
//! the `tests/` folders, which sit outside the folders checked here.
//!
//! Run it with the other tests (`cargo test --workspace`), or alone:
//! `cargo test -p allomos-core --test file_size`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The folders whose files are source, relative to the repository root.
const SOURCE_ROOTS: &[&str] = &["crates", "apps/desktop/src-tauri", "apps/web/src"];
/// Folders never searched: build output, installed packages and test folders.
const SKIPPED_DIRS: &[&str] = &["target", "node_modules", "tests", "gen", "dist"];
const BASELINE: &str = "docs/workflow/file-size-baseline.txt";
const GUIDE: &str = "`alm guide file-size` and docs/workflow/CODING_RULES.md § Writing code";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root exists")
}

fn is_source(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let rust = name.ends_with(".rs");
    let web = (name.ends_with(".ts") || name.ends_with(".tsx"))
        && !name.ends_with(".d.ts")
        && !name.contains(".test.")
        && !name.contains(".spec.");
    rust || web
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if !SKIPPED_DIRS.contains(&name.as_ref()) && !name.starts_with('.') {
                collect(&path, out);
            }
        } else if is_source(&path) {
            out.push(path);
        }
    }
}

/// Lines that count: for Rust, the lines before the `#[cfg(test)]` that opens a test module.
fn counted_lines(path: &Path, text: &str) -> usize {
    let lines: Vec<&str> = text.lines().collect();
    if path.extension().is_some_and(|e| e == "rs") {
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != "#[cfg(test)]" {
                continue;
            }
            let next = lines[i + 1..]
                .iter()
                .map(|l| l.trim())
                .find(|l| !l.is_empty());
            if next.is_some_and(opens_test_module) {
                return i;
            }
        }
    }
    lines.len()
}

fn opens_test_module(line: &str) -> bool {
    let rest = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    rest.starts_with("mod ") && rest.ends_with('{')
}

/// Every source file with the lines that count, by its path from the repository root.
fn measure(root: &Path) -> BTreeMap<String, usize> {
    let mut files = Vec::new();
    for source in SOURCE_ROOTS {
        collect(&root.join(source), &mut files);
    }
    files
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(&path).unwrap_or_default();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, counted_lines(&path, &text))
        })
        .collect()
}

struct Baseline {
    limit: usize,
    files: BTreeMap<String, usize>,
}

fn read_baseline(root: &Path) -> Baseline {
    let text = fs::read_to_string(root.join(BASELINE)).expect("the file size baseline exists");
    let mut limit = None;
    let mut files = BTreeMap::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let (key, value) = line
            .rsplit_once(' ')
            .expect("each baseline line is `<path> <lines>`");
        let value: usize = value.parse().expect("the line count is a number");
        if key == "limit" {
            limit = Some(value);
        } else {
            files.insert(key.to_string(), value);
        }
    }
    Baseline {
        limit: limit.expect("the baseline starts with `limit <lines>`"),
        files,
    }
}

/// The limit in the project's settings, when the taskbag is beside this checkout.
fn taskbag_limit(root: &Path) -> Option<usize> {
    let taskbag = std::env::var_os("ALLOMOS_TASKBAG")
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or_else(|| root.join("../allomos-taskbag/.allomos/taskbag"));
    let text = fs::read_to_string(taskbag.join("project.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    Some(
        json.pointer("/file_size/limit_lines")
            .and_then(serde_json::Value::as_u64)
            .map_or(1000, |n| n as usize),
    )
}

/// What is wrong, in plain words, one item per file; empty when the check passes.
fn problems(
    limit: usize,
    baseline: &BTreeMap<String, usize>,
    now: &BTreeMap<String, usize>,
) -> Vec<String> {
    let mut out = Vec::new();
    for (path, &lines) in now {
        match baseline.get(path) {
            Some(&allowed) if lines > allowed => out.push(format!(
                "{path} has {lines} lines (tests not counted), {} more than the {allowed} it is allowed.\n  \
                 A file over the {limit}-line limit may only get shorter. Put the new code in another file, \
                 or move unrelated old code out of this one. If it truly must grow, raise its line in \
                 {BASELINE} to {lines} in the same commit and record why with \
                 `alm ai-decided <task> \"file-size: {path}: {allowed} → {lines} …\"`.",
                lines - allowed
            )),
            Some(&allowed) if lines < allowed && lines > limit => out.push(format!(
                "{path} got shorter: {lines} lines (tests not counted), down from {allowed}. Good.\n  \
                 Lower its line in {BASELINE} to `{path} {lines}` in the same commit, so it cannot grow back."
            )),
            Some(_) if lines <= limit => out.push(format!(
                "{path} is now {lines} lines (tests not counted), within the {limit}-line limit. Good.\n  \
                 Remove its line from {BASELINE} in the same commit."
            )),
            None if lines > limit => out.push(format!(
                "{path} has {lines} lines (tests not counted), over the {limit}-line limit, and it is not in \
                 {BASELINE}.\n  A new file may not go over the limit: split it into files of their own. \
                 (Only the files listed in the baseline may stay over it, and only get shorter.)"
            )),
            _ => {}
        }
    }
    for path in baseline.keys().filter(|p| !now.contains_key(*p)) {
        out.push(format!(
            "{path} is in {BASELINE} but no longer exists (or is no longer a source file).\n  \
             Remove its line from the baseline in the same commit."
        ));
    }
    out
}

#[test]
fn source_files_over_the_limit_only_get_shorter() {
    let root = repo_root();
    let baseline = read_baseline(&root);
    if let Some(setting) = taskbag_limit(&root) {
        assert_eq!(
            baseline.limit, setting,
            "The project's line limit is {setting} (`alm file-size-settings`), but {BASELINE} starts with \
             `limit {}`. Change that first line to `limit {setting}` and update the baseline to match.",
            baseline.limit
        );
    }
    let problems = problems(baseline.limit, &baseline.files, &measure(&root));
    assert!(
        problems.is_empty(),
        "The file size check failed ({GUIDE}):\n\n{}\n",
        problems.join("\n\n")
    );
}

#[test]
fn the_check_says_what_to_do() {
    let base = BTreeMap::from([
        ("big.rs".to_string(), 1200),
        ("shrunk.rs".to_string(), 1100),
    ]);
    let now = BTreeMap::from([
        ("big.rs".to_string(), 1201),
        ("shrunk.rs".to_string(), 1050),
        ("new.rs".to_string(), 1001),
        ("small.rs".to_string(), 1000),
    ]);
    let found = problems(1000, &base, &now);
    assert_eq!(found.len(), 3, "{found:#?}");
    assert!(
        found
            .iter()
            .any(|p| p.starts_with("big.rs has 1201 lines") && p.contains("1 more than the 1200"))
    );
    assert!(
        found
            .iter()
            .any(|p| p.starts_with("shrunk.rs got shorter") && p.contains("`shrunk.rs 1050`"))
    );
    assert!(
        found
            .iter()
            .any(|p| p.starts_with("new.rs has 1001 lines") && p.contains("not in"))
    );
    let gone = problems(1000, &base, &BTreeMap::from([("big.rs".to_string(), 1200)]));
    assert_eq!(gone.len(), 1);
    assert!(gone[0].starts_with("shrunk.rs is in"));
}

#[test]
fn rust_tests_do_not_count() {
    let text = "fn a() {}\n#[cfg(test)]\nfn helper() {}\nfn b() {}\n\n#[cfg(test)]\n\nmod tests {\n    #[test]\n    fn t() {}\n}\n";
    assert_eq!(counted_lines(Path::new("x.rs"), text), 5);
    assert_eq!(counted_lines(Path::new("x.ts"), text), text.lines().count());
}
