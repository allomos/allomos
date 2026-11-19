//! The Evidence screen's screenshots (Task-A01-fondue470, decision 0029 § 6): a failed check's
//! picture, read from the task's `evidence/` folder in the taskbag, where `alm check-result
//! --screenshot` copied it. Only reads, and only that folder: the task ID must be one and the
//! file a plain name (no folders, no `..`), and what is read must be a PNG, JPEG or WebP picture.

use std::path::{Path, PathBuf};

use allomos_core::check_result::{self, EVIDENCE_DIR};
use allomos_core::Problem;

/// A little more than `alm` takes, so a picture copied by hand at the limit still shows.
const MOST_READ: u64 = 2 * check_result::MOST_BYTES;

/// Whether `file` is a plain file name: not empty, no folder parts, not hidden.
fn plain_name(file: &str) -> bool {
    !file.is_empty()
        && !file.starts_with('.')
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// The picture's bytes; refused (`link_refused`) for anything but a plain name in a task's
/// `evidence/` folder, `link_not_found` when this computer does not have it.
fn read(taskbag: &Path, task: &str, file: &str) -> Result<Vec<u8>, Problem> {
    let refused = || Problem::LinkRefused {
        link: format!("{EVIDENCE_DIR}/{file}"),
    };
    if !allomos_core::ops::valid_id(task) || !plain_name(file) {
        return Err(refused());
    }
    let path: PathBuf = taskbag.join("tasks").join(task).join(EVIDENCE_DIR).join(file);
    let missing = || Problem::LinkNotFound {
        link: format!("tasks/{task}/{EVIDENCE_DIR}/{file}"),
    };
    let meta = std::fs::metadata(&path).map_err(|_| missing())?;
    if !meta.is_file() || meta.len() > MOST_READ {
        return Err(refused());
    }
    let bytes = std::fs::read(&path).map_err(|_| missing())?;
    if check_result::image_kind(&bytes).is_none() {
        return Err(refused());
    }
    Ok(bytes)
}

/// One screenshot of task `task` in the taskbag at `taskbag`, as bytes (the page shows them as a
/// picture). Only reads.
#[tauri::command(async)]
pub fn evidence_screenshot(
    taskbag: PathBuf,
    task: String,
    file: String,
) -> Result<tauri::ipc::Response, Problem> {
    read(&taskbag, &task, &file).map(tauri::ipc::Response::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_pictures_in_a_tasks_evidence_folder() {
        let root = std::env::temp_dir().join(format!("alm-shot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("tasks").join("A01-01").join(EVIDENCE_DIR);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.png"), b"\x89PNG\r\n\x1a\nxx").unwrap();
        std::fs::write(dir.join("b.png"), b"not a picture").unwrap();
        std::fs::write(root.join("tasks").join("A01-01").join("task.json"), "{}").unwrap();

        assert_eq!(read(&root, "A01-01", "a.png").unwrap(), b"\x89PNG\r\n\x1a\nxx");
        let code = |task: &str, file: &str| read(&root, task, file).unwrap_err().code();
        assert_eq!(code("A01-01", "b.png"), "link_refused");
        assert_eq!(code("A01-01", "../task.json"), "link_refused");
        assert_eq!(code("A01-01", ".hidden.png"), "link_refused");
        assert_eq!(code("../A01-01", "a.png"), "link_refused");
        assert_eq!(code("A01-01", "c.png"), "link_not_found");
        let _ = std::fs::remove_dir_all(&root);
    }
}
