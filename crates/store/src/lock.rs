//! The taskbag's lock, held for one change, and loading what a change builds on.

use super::*;

/// The taskbag's lock, held for one change: load, check the rules, write, then drop it.
/// Every write goes through a `Lock`, so two Allomos writers never change the taskbag at
/// the same moment. The operating system releases the lock if the process dies.
pub struct Lock {
    root: PathBuf,
    _file: fs::File,
}

impl Lock {
    /// Pack the history of whole earlier months from the 28th of a month (Task-A01-59):
    /// [`events::pack`].
    pub fn pack_history(&self) -> Result<Vec<events::Packed>, Problem> {
        events::pack(&self.root)
    }

    /// Take the lock, waiting up to `wait` for another writer to finish.
    pub fn acquire(root: &Path, wait: Duration) -> Result<Lock, Problem> {
        if !root.join(PROJECT_FILE).is_file() {
            return Err(Problem::TaskbagNotFound {
                path: root.display().to_string(),
            });
        }
        let path = root.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| write_failed(&path, e))?;
        let start = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => {
                    return Ok(Lock {
                        root: root.to_path_buf(),
                        _file: file,
                    });
                }
                Err(TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(Problem::TaskbagBusy {
                        path: root.display().to_string(),
                    });
                }
                Err(TryLockError::Error(e)) => return Err(write_failed(&path, e)),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load the taskbag while holding the lock, so what is read is what the change builds on.
    pub fn load(&self) -> Loaded {
        load(&self.root)
    }

    /// Write a new area file. Refused when the file already exists.
    pub fn create_area(&self, area: &Area) -> Result<(), Problem> {
        let dir = self.root.join("areas");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", area.id));
        if path.exists() {
            return Err(Problem::DuplicateAreaId {
                id: area.id.clone(),
            });
        }
        write_atomic(&path, &json_text(area))
    }

    /// Write a new iteration file. Refused when the file already exists.
    pub fn create_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let dir = self.root.join("iterations");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", iteration.id));
        if path.exists() {
            return Err(Problem::DuplicateIterationId {
                id: iteration.id.clone(),
            });
        }
        write_atomic(&path, &json_text(iteration))
    }

    /// Write a new task folder: `task.json`, plus `description.md` when there is a description.
    pub fn create_task(&self, task: &Task) -> Result<(), Problem> {
        let dir = self.root.join("tasks");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let target = dir.join(&task.id);
        if target.exists() {
            return Err(Problem::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
        let mut files = vec![("task.json", json_text(&task_file_json(task)))];
        if !task.description.is_empty() {
            files.push(("description.md", task.description.clone()));
        }
        create_dir_atomic(&target, &files)
    }

    /// Rewrite an existing area's file (`alm rename area`, Task-A01-29).
    pub fn save_area(&self, area: &Area) -> Result<(), Problem> {
        let path = self.root.join("areas").join(format!("{}.json", area.id));
        write_atomic(&path, &json_text(area))
    }

    /// Rewrite an existing iteration's file (`alm change iteration`, Task-A01-29).
    pub fn save_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let path = self
            .root
            .join("iterations")
            .join(format!("{}.json", iteration.id));
        write_atomic(&path, &json_text(iteration))
    }

    /// Remove an area's or an iteration's file (`alm delete area|iteration`, Task-A01-29).
    /// `folder` is `areas` or `iterations`.
    pub fn remove_item(&self, folder: &str, id: &str) -> Result<(), Problem> {
        let path = self.root.join(folder).join(format!("{id}.json"));
        fs::remove_file(&path).map_err(|e| write_failed(&path, e))?;
        note_written(&path);
        Ok(())
    }

    /// Rewrite `project.json` (`alm labels`).
    pub fn save_project(&self, project: &Project) -> Result<(), Problem> {
        write_atomic(&self.root.join(PROJECT_FILE), &json_text(project))
    }

    /// Rewrite an existing task's `task.json`. Its description is left as it is.
    pub fn save_task(&self, task: &Task) -> Result<(), Problem> {
        let path = self.root.join("tasks").join(&task.id).join("task.json");
        write_atomic(&path, &json_text(&task_file_json(task)))
    }

    /// Rewrite an existing task's `description.md`, or remove it when the description is empty
    /// (the app's task pane, Task-A01-18).
    pub fn save_description(&self, task: &Task) -> Result<(), Problem> {
        let path = self
            .root
            .join("tasks")
            .join(&task.id)
            .join("description.md");
        if task.description.is_empty() {
            match fs::remove_file(&path) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(write_failed(&path, e)),
                Err(_) => Ok(()),
                Ok(()) => {
                    note_written(&path);
                    Ok(())
                }
            }
        } else {
            write_atomic(&path, &task.description)
        }
    }

    /// Make one change and record it (Task-A01-17): write `event`, then run `write`. When `write`
    /// fails the event is taken back, so the history never shows a change that did not happen.
    /// An event with no changes is not written: nothing happened worth recording.
    pub fn record(
        &self,
        event: &Event,
        write: impl FnOnce(&Lock) -> Result<(), Problem>,
    ) -> Result<(), Problem> {
        if event.changes.is_empty() {
            return write(self);
        }
        let file = events::write(&self.root, event)?;
        write(self).inspect_err(|_| {
            let _ = fs::remove_file(&file);
        })
    }
}

/// The taskbag loaded at `root`, or why it cannot be worked on.
pub fn taskbag_of(root: &Path, loaded: Loaded) -> Result<(Taskbag, Vec<Issue>), Problem> {
    match loaded.taskbag {
        Some(bag) => Ok((bag, loaded.issues)),
        None => Err(Problem::TaskbagUnreadable {
            path: root.display().to_string(),
            detail: loaded
                .issues
                .first()
                .map(|i| format!("{}: {}", i.file, i.problem.english()))
                .unwrap_or_default(),
        }),
    }
}

/// Before any change, for `alm` and the app alike: refuse a taskbag in a format this Allomos does
/// not write ([`versions::write_allowed`]: a newer one, or an older one whose tasks are to be moved
/// in, Task-A01-45); take the lock (waiting up to `wait` for another writer), then load what the
/// change builds on.
pub(crate) fn lock_for_change(
    root: &Path,
    wait: Duration,
) -> Result<(Lock, Taskbag, Vec<Issue>), Problem> {
    versions::write_allowed(root)?;
    versions::upgraded_for_writing(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, issues) = taskbag_of(root, lock.load())?;
    Ok((lock, bag, issues))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn writes_go_through_the_lock() {
        let dir = tmp("lock");
        let root = dir.join(DEFAULT_DIR);
        assert!(matches!(
            Lock::acquire(&root, Duration::ZERO),
            Err(Problem::TaskbagNotFound { .. })
        ));
        init(&root, "p", "P").unwrap();
        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        // A second writer waits, then gives up while the first holds the lock.
        assert!(matches!(
            Lock::acquire(&root, Duration::from_millis(20)),
            Err(Problem::TaskbagBusy { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let area = allomos_core::ops::new_area(&bag, "A01", "First").unwrap();
        lock.create_area(&area).unwrap();
        assert!(matches!(
            lock.create_area(&area),
            Err(Problem::DuplicateAreaId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let task =
            allomos_core::ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why")).unwrap();
        lock.create_task(&task).unwrap();
        assert!(matches!(
            lock.create_task(&task),
            Err(Problem::DuplicateTaskId { .. })
        ));
        let bag = lock.load().taskbag.unwrap();
        let claimed = allomos_core::ops::claim(&bag, "A01-01", Some("me"), false)
            .unwrap()
            .task;
        lock.save_task(&claimed).unwrap();
        drop(lock);
        assert!(Lock::acquire(&root, Duration::ZERO).is_ok());

        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let t = &loaded.taskbag.unwrap().tasks[0];
        assert_eq!(t.holder.as_deref(), Some("me"));
        assert_eq!(t.description, "Why\n");
        let json = fs::read_to_string(root.join("tasks/A01-01/task.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("description"));
        assert_ne!(stamp(&root), before);
        // Only the lock and the taskbag's own files: no temporary files left behind.
        let hidden: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with(TMP_PREFIX))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
        fs::remove_dir_all(dir).unwrap();
    }
}
