//! The short introduction Allomos leaves in a folder it fills from empty.

use super::*;

/// Name of the short introduction Allomos leaves in a folder it fills from empty.
/// Never `README.md`, so it cannot clash with the user's own README (owner, 2026-10-06).
pub const README_FILE: &str = "README-allomos.md";

/// A folder looks empty when it has no entries, or only hidden ones (names starting with
/// `.`, such as `.DS_Store`, `.git` or `.allomos`). A folder that cannot be read does not.
pub fn looks_empty(folder: &Path) -> bool {
    fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .all(|e| e.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

/// The README's text in an interface language (`en`, `zh-CN`, `ja`); English otherwise.
pub fn readme_text(language: &str) -> &'static str {
    match language {
        "zh-CN" => README_ZH_CN,
        "ja" => README_JA,
        _ => README_EN,
    }
}

const README_EN: &str = "# Allomos project

This folder is an Allomos project. Its tasks, areas and progress are in
`.allomos/taskbag/`, a hidden folder.

- Open this folder in the Allomos app to see the tasks by area.
- AI agents and people can also work the tasks from a terminal with the `alm` command
  (`alm list`, `alm next`, `alm claim <task>`).
- The files are plain JSON and Markdown, so a version control tool (Git, for example) can track every change.

Allomos wrote this file because the folder was empty. You may edit or delete it.
";

const README_ZH_CN: &str = "# 换肩项目

此文件夹是一个「换肩」项目。它的任务、领域和进度保存在隐藏文件夹
`.allomos/taskbag/` 中。

- 在「换肩」应用中打开此文件夹，即可按领域查看任务。
- AI 智能体和人也可以在终端中用 `alm` 命令处理任务
  （`alm list`、`alm next`、`alm claim <任务>`）。
- 这些文件是普通的 JSON 和 Markdown，版本控制工具（例如 Git）可以记录每一处更改。

此文件夹原本是空的，所以「换肩」写了这个文件。你可以修改或删除它。
";

const README_JA: &str = "# 肩代プロジェクト

このフォルダーは「肩代」のプロジェクトです。タスク、エリア、進捗は隠しフォルダー
`.allomos/taskbag/` に入っています。

- 「肩代」アプリでこのフォルダーを開くと、タスクをエリアごとに見られます。
- AI エージェントや人は、ターミナルの `alm` コマンドでもタスクを扱えます
  （`alm list`、`alm next`、`alm claim <タスク>`）。
- ファイルはただの JSON と Markdown なので、バージョン管理ツール（たとえば Git）ですべての変更を記録できます。

このフォルダーが空だったので、「肩代」がこのファイルを書きました。編集や削除は自由です。
";

/// Write `README-allomos.md` into `folder` when the folder looks empty (see
/// [`looks_empty`]). Never overwrites a file. Returns whether the README was written.
pub fn add_readme_if_empty(folder: &Path, language: &str) -> io::Result<bool> {
    if !looks_empty(folder) {
        return Ok(false);
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(README_FILE))
    {
        Ok(mut file) => {
            file.write_all(readme_text(language).as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
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
    fn hidden_only_folder_looks_empty() {
        let dir = tmp("hidden-only");
        assert!(looks_empty(&dir));
        fs::write(dir.join(".DS_Store"), "x").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        assert!(looks_empty(&dir));
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: dir.join(DEFAULT_DIR)
            }
        );
        fs::write(dir.join("notes.txt"), "x").unwrap();
        assert!(!looks_empty(&dir));
        assert!(matches!(inspect(&dir), Inspection::NoTaskbag { .. }));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_goes_only_into_a_folder_that_looks_empty() {
        // Empty, and only `.DS_Store`: the taskbag and the README both arrive.
        for extra in [None, Some(".DS_Store")] {
            let dir = tmp("readme-empty");
            if let Some(name) = extra {
                fs::write(dir.join(name), "x").unwrap();
            }
            let root = dir.join(DEFAULT_DIR);
            create_starter(&root, "Demo").unwrap();
            assert_eq!(picked_folder(&root), Some(dir.as_path()));
            assert!(add_readme_if_empty(&dir, "en").unwrap());
            let text = fs::read_to_string(dir.join(README_FILE)).unwrap();
            assert!(text.contains(".allomos/taskbag/"));
            assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
            // Now the folder has a visible file: a second call writes nothing.
            assert!(!add_readme_if_empty(&dir, "ja").unwrap());
            assert_eq!(fs::read_to_string(dir.join(README_FILE)).unwrap(), text);
            fs::remove_dir_all(dir).unwrap();
        }
        // A folder with a visible file gets no README.
        let dir = tmp("readme-code");
        fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        create_starter(&dir.join(DEFAULT_DIR), "Code").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert!(!dir.join(README_FILE).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_never_overwrites() {
        let dir = tmp("readme-keep");
        fs::create_dir(dir.join(".git")).unwrap();
        let path = dir.join(README_FILE);
        // An existing README is a visible file, so the folder no longer looks empty.
        fs::write(&path, "mine").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_speaks_three_languages() {
        assert!(readme_text("en").starts_with("# Allomos project"));
        assert!(readme_text("zh-CN").contains("此文件夹是一个「换肩」项目"));
        assert!(readme_text("ja").contains("このフォルダーは「肩代」のプロジェクトです"));
        assert_eq!(readme_text("fr"), readme_text("en"));
        for lang in ["en", "zh-CN", "ja"] {
            assert!(readme_text(lang).contains("`.allomos/taskbag/`"));
            assert!(readme_text(lang).contains("`alm"));
        }
        assert_eq!(picked_folder(Path::new("/x/taskbag")), None);
    }
}
