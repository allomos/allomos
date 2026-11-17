//! Writing the taskbag's files: whole files and folders put in place at once, and the list of
//! files a change wrote ([`with_written_files`]).

use super::*;

pub(crate) fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

/// The JSON stored in `task.json`: the task without its description (that lives in
/// `description.md`) and without empty optional fields.
pub(crate) fn task_file_json(task: &Task) -> serde_json::Value {
    let mut v = serde_json::to_value(task).expect("a task always serializes");
    if let Some(map) = v.as_object_mut() {
        map.remove("description");
        map.retain(|_, value| !value.is_null());
    }
    v
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

thread_local! {
    /// The files written or removed by the change running on this thread, while
    /// [`with_written_files`] is listening.
    static WRITTEN: RefCell<Option<BTreeSet<PathBuf>>> = const { RefCell::new(None) };
}

/// Note that `path` was written or removed, when someone is listening.
pub(crate) fn note_written(path: &Path) {
    WRITTEN.with(|w| {
        if let Some(set) = w.borrow_mut().as_mut() {
            set.insert(path.to_path_buf());
        }
    });
}

/// Whether [`with_written_files`] is listening on this thread.
fn listening() -> bool {
    WRITTEN.with(|w| w.borrow().is_some())
}

/// Run `f`, and return with its result every file it wrote or removed through this crate
/// (decision 0017): a file rewritten with the same text is not counted. `alm` prints the Git
/// commands for exactly these files; nothing here runs Git.
pub fn with_written_files<T>(f: impl FnOnce() -> T) -> (T, Vec<PathBuf>) {
    let outer = WRITTEN.with(|w| w.borrow_mut().replace(BTreeSet::new()));
    let out = f();
    let mine = WRITTEN.with(|w| {
        let mut slot = w.borrow_mut();
        let mine = slot.take().unwrap_or_default();
        // Listening inside another listener: the outer one sees these files too.
        *slot = outer.map(|mut o| {
            o.extend(mine.iter().cloned());
            o
        });
        mine
    });
    (out, mine.into_iter().collect())
}

/// A name for a temporary file or folder next to `dir`'s contents, unique to this process.
fn tmp_path(dir: &Path, what: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{TMP_PREFIX}{}-{n}-{what}", std::process::id()))
}

pub(crate) fn write_failed(path: &Path, e: impl std::fmt::Display) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

pub(crate) fn json_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("taskbag values always serialize");
    text.push('\n');
    text
}

/// How long replacing a file keeps trying while another program has it open (Windows).
const REPLACE_RETRY: Duration = Duration::from_millis(1000);
const REPLACE_PAUSE: Duration = Duration::from_millis(50);

/// Whether `e` may be a file that another program has open, which Windows reports as "access
/// denied" (5) or a sharing or lock violation (32, 33) until that program lets go.
fn may_be_in_use(e: &io::Error) -> bool {
    e.kind() == io::ErrorKind::PermissionDenied
        || (cfg!(windows) && matches!(e.raw_os_error(), Some(5 | 32 | 33)))
}

/// Run `attempt` until it works, a failure is not [`may_be_in_use`], or `limit` has passed,
/// pausing `pause` in between. A writer holds the taskbag's lock meanwhile, so the wait stays
/// well within the 5 seconds another writer waits for the lock.
fn retry_in_use<T>(
    limit: Duration,
    pause: Duration,
    mut attempt: impl FnMut() -> io::Result<T>,
) -> io::Result<T> {
    let start = Instant::now();
    loop {
        match attempt() {
            Err(e) if may_be_in_use(&e) && start.elapsed() < limit => std::thread::sleep(pause),
            result => return result,
        }
    }
}

/// Rename `from` over `to`. On Windows this fails while another program (an editor, a virus
/// scanner, Git) has `to` open without sharing it, so it is tried again for up to a second.
pub(crate) fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    retry_in_use(REPLACE_RETRY, REPLACE_PAUSE, || fs::rename(from, to))
}

/// Write `path` whole: the text goes to a temporary file in the same folder, is flushed to
/// disk, and is then renamed over `path`. A reader sees either the old file or the new one,
/// never a part-written one. Line endings are always written as `\n`.
pub(crate) fn write_atomic(path: &Path, text: &str) -> Result<(), Problem> {
    if listening() && fs::read_to_string(path).is_ok_and(|old| old == lf(text)) {
        // The same text again: the file is rewritten, but nothing changed to commit.
    } else {
        note_written(path);
    }
    let dir = path.parent().expect("taskbag files are inside a folder");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = tmp_path(dir, name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(lf(text).as_bytes())?;
        f.sync_all()?;
        drop(f);
        replace_file(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        write_failed(path, e)
    })
}

/// Fill a new folder with files, then rename it to `target` in one step, so a reader never
/// sees it half-made. Refused when `target` already exists.
pub(crate) fn create_dir_atomic(target: &Path, files: &[(&str, String)]) -> Result<(), Problem> {
    let parent = target.parent().expect("a target has a parent");
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("dir");
    let tmp = tmp_path(parent, name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for (rel_path, text) in files {
            let path = tmp.join(rel_path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = fs::File::create(&path)?;
            f.write_all(lf(text).as_bytes())?;
            f.sync_all()?;
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "already exists",
            ));
        }
        fs::rename(&tmp, target)
    })();
    if result.is_ok() {
        for (rel_path, _) in files {
            note_written(&target.join(rel_path));
        }
    }
    result.map_err(|e| {
        let _ = fs::remove_dir_all(&tmp);
        write_failed(target, e)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::ops;
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

    /// Every text file under `dir` rewritten with `\r\n` line endings, the way Git on Windows
    /// may check a taskbag out.
    fn to_crlf(dir: &Path) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                to_crlf(&path);
            } else if let Ok(text) = fs::read_to_string(&path) {
                fs::write(&path, text.replace('\n', "\r\n")).unwrap();
            }
        }
    }

    #[test]
    fn a_taskbag_checked_out_with_crlf_loads_and_is_written_back_with_lf() {
        let dir = tmp("crlf");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task =
            ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why\nand how\n")).unwrap();
        lock.create_task(&task).unwrap();
        drop(lock);

        to_crlf(&root);
        assert!(
            fs::read_to_string(root.join("project.json"))
                .unwrap()
                .contains("\r\n")
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        assert_eq!(bag.tasks[0].description, "Why\nand how\n");

        // What Allomos writes back has `\n` only, even from text that came with `\r\n`.
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        lock.create_task(&Task {
            id: "A01-02".into(),
            description: "One\r\nTwo\r\n".into(),
            ..task.clone()
        })
        .unwrap();
        drop(lock);
        for file in ["tasks/A01-01/task.json", "tasks/A01-02/description.md"] {
            let text = fs::read_to_string(root.join(file)).unwrap();
            assert!(!text.contains('\r'), "{file}: {text:?}");
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replacing_a_file_that_is_open_for_reading_succeeds() {
        let dir = tmp("open-reader");
        let root = default_root(&dir);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().taskbag.unwrap();
        let mut task = ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap();
        lock.create_task(&task).unwrap();
        let path = root.join("tasks/A01-01/task.json");
        let before = fs::read_to_string(&path).unwrap();

        // Another reader has the file open while it is replaced.
        let mut reader = fs::File::open(&path).unwrap();
        task.holder = Some("me".into());
        lock.save_task(&task).unwrap();
        let mut seen = String::new();
        io::Read::read_to_string(&mut reader, &mut seen).unwrap();
        // The reader sees the old file whole, or (where the system shares it) the new one whole.
        let after = fs::read_to_string(&path).unwrap();
        assert!(seen == before || seen == after, "{seen}");
        assert!(after.contains("\"holder\": \"me\""), "{after}");
        drop(reader);
        drop(lock);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn replacing_retries_while_the_file_is_in_use_then_gives_up() {
        let in_use = || io::Error::from(io::ErrorKind::PermissionDenied);
        // In use twice, then free: it works.
        let mut tries = 0;
        let result = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            if tries < 3 { Err(in_use()) } else { Ok(tries) }
        });
        assert_eq!(result.unwrap(), 3);
        // Another failure is not retried.
        let mut tries = 0;
        let result: io::Result<()> = retry_in_use(Duration::from_secs(1), Duration::ZERO, || {
            tries += 1;
            Err(io::Error::from(io::ErrorKind::NotFound))
        });
        assert!(result.is_err());
        assert_eq!(tries, 1);
        // Still in use when the time is up: the failure is returned.
        let result: io::Result<()> =
            retry_in_use(Duration::from_millis(20), Duration::from_millis(5), || {
                Err(in_use())
            });
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
    }
}
