//! The setup request names every project folder of the taskbag (Task-A05-king717): the setup
//! wizard's step 3 binds several, and step 4's agent sets up AGENTS.md in each.

use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use allomos_store::{
    Actor, DEFAULT_DIR, agent_setup_request_in, folders, init, record_project_folder,
};

fn tmp(name: &str) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("allomos-setup-{name}-{n}"));
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn the_setup_request_names_the_other_project_folders() {
    let code = tmp("code");
    let root = code.join(DEFAULT_DIR);
    init(&root, "p", "P").unwrap();
    // One folder: the request is as before.
    record_project_folder(&root, &code).unwrap();
    let alone = agent_setup_request_in("en", &code, &root);
    assert!(!alone.contains("also serves"), "{alone}");
    // A second folder, bound as the wizard's Add Another Folder… does.
    let site = tmp("site");
    folders::add(&root, Actor::APP, Duration::from_secs(5), &site, None).unwrap();
    for (language, words) in [
        ("en", "Do steps 2 to 6 in each of them"),
        ("zh-CN", "第 2 到第 6 步"),
        ("ja", "手順 2〜6"),
    ] {
        let request = agent_setup_request_in(language, &code, &root);
        assert!(request.contains(words), "{language}: {request}");
        assert!(
            request.contains(&*site.display().to_string()),
            "{language}: {request}"
        );
        // The section's source stays the last line ("see the last line").
        let last = request.trim_end().lines().last().unwrap();
        assert!(!last.contains(words), "{language}: {last}");
    }
    // The main folder is not listed among the others.
    let en = agent_setup_request_in("en", &code, &root);
    let others = en
        .split("also serves these project folders:")
        .nth(1)
        .unwrap();
    let others = others.split("\n\n").next().unwrap();
    assert_eq!(
        others.trim(),
        format!("- {}", site.display()),
        "only the other folder is listed"
    );
    let _ = fs::remove_dir_all(&code);
    let _ = fs::remove_dir_all(&site);
}
