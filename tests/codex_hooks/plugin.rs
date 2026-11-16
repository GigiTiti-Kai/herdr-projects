//! Codex's hooks judged when a plugin's files were changed where git does not look (skip-worktree, assume-unchanged,
//! a filter of the checkout's own), and what a plugin's file loads that node would find its own way.
use super::{allowed, clean, hooks_of, judge, listed, own_repo, plugin, publish, put, repo_hook};
use crate::common;
use common::*;

#[test]
fn a_plugin_file_changed_where_git_is_told_not_to_look_is_not_trusted() {
    let w = world();
    let root = plugin(&w.base, &clean());
    let hooks = hooks_of(&listed(&root, "modified"));
    git(&root, &["update-index", "--skip-worktree", "hooks/b.js"]);
    std::fs::write(root.join("hooks/b.js"), "exports.go = () => 2;\n").unwrap();
    assert_eq!(git(&root, &["status", "--porcelain"]), "", "git says nothing changed");
    let why = judge(&w, 1, &hooks, &allowed()).unwrap_err();
    assert!(why.contains("hooks/b.js is not as at the plugin's commit"), "{why}");
    // The same with --assume-unchanged, on the plugin and on a file of your repo.
    git(&root, &["update-index", "--no-skip-worktree", "hooks/b.js"]);
    git(&root, &["checkout", "--", "hooks/b.js"]);
    assert!(judge(&w, 1, &hooks, &allowed()).is_ok());
    git(&root, &["update-index", "--assume-unchanged", "hooks/a.js"]);
    std::fs::write(root.join("hooks/a.js"), "exports.go = () => 2;\n").unwrap();
    assert_eq!(git(&root, &["status", "--porcelain"]), "");
    assert!(judge(&w, 1, &hooks, &allowed()).unwrap_err().contains("hooks/a.js is not as at the plugin's commit"));
    git(&root, &["update-index", "--no-assume-unchanged", "hooks/a.js"]);
    git(&root, &["checkout", "--", "hooks/a.js"]);
    // A filter of the checkout's own that hands git the committed bytes, whatever is on disk.
    std::fs::write(root.join(".git/info/attributes"), "hooks/b.js filter=same\n").unwrap();
    git(&root, &["config", "filter.same.clean", "git cat-file blob HEAD:%f"]);
    git(&root, &["update-index", "--skip-worktree", "hooks/b.js"]);
    std::fs::write(root.join("hooks/b.js"), "exports.go = () => 2;\n").unwrap();
    assert!(judge(&w, 1, &hooks, &allowed()).unwrap_err().contains("hooks/b.js is not as at the plugin's commit"), "through a filter");
    let r = own_repo(&w.base);
    std::fs::write(r.join(".git/info/attributes"), "hooks/a.sh filter=same\n").unwrap();
    git(&r, &["config", "filter.same.clean", "git cat-file blob HEAD:%f"]);
    std::fs::write(r.join("hooks/a.sh"), "#!/bin/sh\necho through a filter\n").unwrap();
    assert!(judge(&w, 1, &hooks_of(&repo_hook(&r)), &allowed()).unwrap_err().contains("hooks/a.sh in me/dots is not the same as on origin/main"), "repo through a filter");
    git(&r, &["checkout", "--", "hooks/a.sh"]);
    std::fs::remove_file(r.join(".git/info/attributes")).unwrap();
    git(&r, &["update-index", "--assume-unchanged", "hooks/a.sh"]);
    std::fs::write(r.join("hooks/a.sh"), "#!/bin/sh\necho changed\n").unwrap();
    assert!(judge(&w, 1, &hooks_of(&repo_hook(&r)), &allowed()).unwrap_err().contains("hooks/a.sh in me/dots is not the same as on origin/main"));
}

#[test]
fn a_file_loaded_as_something_node_finds_but_the_board_cannot_read_is_not_trusted() {
    let w = world();
    // A file it loads that node would find as something the board cannot read (a native `.node`).
    let p2 = plugin(&w.base, &[("hooks/hooks.json", "{}"), ("hooks/a.js", "require('./native');\n"), ("hooks/native.node", "\u{0}\u{1}")]);
    std::fs::write(p2.join("hooks/native.node"), [0xffu8, 0xfe, 0x00]).unwrap();
    put(&p2, &[]);
    publish(&p2);
    assert!(judge(&w, 1, &hooks_of(&listed(&p2, "modified")), &allowed()).unwrap_err().contains("native.node does not read as text"));
}

#[test]
fn judging_a_changed_plugin_runs_no_command_of_its_checkout() {
    let w = world();
    let root = plugin(&w.base, &clean());
    let hooks = hooks_of(&listed(&root, "modified"));
    // A filter of the checkout's own: git would run it on a file it reads through filters (`git status`).
    let mark = w.base.join("filter-ran");
    std::fs::write(root.join(".git/info/attributes"), "*.js filter=probe\n").unwrap();
    git(&root, &["config", "filter.probe.clean", &format!("tee {}", mark.display())]);
    git(&root, &["config", "core.fsmonitor", &format!("touch {}", mark.display())]);
    std::fs::write(root.join("hooks/a.js"), "const b = require('./b');\nb.go(2);\n").unwrap();
    let why = judge(&w, 1, &hooks, &allowed()).unwrap_err();
    assert!(why.contains("hooks/a.js is not as at the plugin's commit"), "{why}");
    assert!(!mark.exists(), "judging ran a command of the checkout's own");
    // A file the checkout has that is not the plugin's own.
    std::fs::write(root.join("hooks/a.js"), clean()[1].1).unwrap(); // by hand: a git of the test's own would run them
    std::fs::write(root.join("extra.js"), "1\n").unwrap();
    let why = judge(&w, 1, &hooks, &allowed()).unwrap_err();
    assert!(why.ends_with("plugin pt@pt's checkout has files of its own (extra.js)"), "{why}");
    assert!(!mark.exists());
}

#[test]
fn a_package_folder_it_loads_is_not_followed_and_not_trusted() {
    let w = world();
    let root = plugin(&w.base, &[("hooks/hooks.json", "{}"), ("hooks/a.js", "require('./pkg');\n"), ("hooks/pkg/package.json", "{\"main\":\"entry\"}"), ("hooks/pkg/entry", "require('https').get(u)\n")]);
    let why = judge(&w, 1, &hooks_of(&listed(&root, "modified")), &allowed()).unwrap_err();
    assert!(why.contains("loads the package folder ./pkg, whose own entry the board does not follow"), "{why}");
}

