//! Real-git function-grain integration tests for [`RustGitSynResolver`].

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use hotspots::Change;
use hotspots::symbols::SymbolResolver;
use hotspots_rs::RustGitSynResolver;

static REPO_SEQ: AtomicU64 = AtomicU64::new(0);

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn create(label: &str) -> Self {
        let seq = REPO_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "hotspots-fg-{}-{}-{}",
            std::process::id(),
            seq,
            label
        ));
        let _ = fs::remove_dir_all(&path);
        assert!(
            fs::create_dir_all(&path).is_ok(),
            "failed to create temp repo dir"
        );
        let repo = Self { path };
        assert!(git_ok(&repo.path, &["init"]).is_some(), "git init failed");
        assert!(
            git_ok(&repo.path, &["config", "user.name", "Test"]).is_some(),
            "git config user.name failed"
        );
        assert!(
            git_ok(&repo.path, &["config", "user.email", "test@example.com"]).is_some(),
            "git config user.email failed"
        );
        repo
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn git_ok(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").arg("-C").arg(repo).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok().map(|s| s.trim().to_owned())
}

fn git_commit(repo: &Path, message: &str, date: &str) -> String {
    let stamp = format!("{date}T12:00:00");
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["commit", "-m", message])
        .env("GIT_AUTHOR_DATE", &stamp)
        .env("GIT_COMMITTER_DATE", &stamp)
        .output();
    assert!(output.is_ok(), "failed to spawn git commit");
    if let Ok(output) = output {
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    git_ok(repo, &["rev-parse", "--short", "HEAD"]).unwrap_or_default()
}

fn write_file(repo: &Path, rel: &str, contents: &str) {
    let path = repo.join(rel);
    if let Some(parent) = path.parent() {
        assert!(fs::create_dir_all(parent).is_ok());
    }
    assert!(fs::write(path, contents).is_ok());
}

fn stage_all(repo: &Path) {
    assert!(git_ok(repo, &["add", "-A"]).is_some(), "git add failed");
}

fn expand(repo: &Path, changes: &[Change]) -> Vec<Change> {
    let resolver = RustGitSynResolver::new();
    let result = resolver.expand(changes, repo);
    assert!(result.is_ok(), "expand failed: {:?}", result.err());
    result.map(|(rows, _)| rows).unwrap_or_default()
}

fn has_entity(rows: &[Change], entity: &str) -> bool {
    rows.iter().any(|c| c.entity == entity)
}

#[test]
fn expands_added_function_in_real_repo() {
    let repo = TempRepo::create("add");
    write_file(&repo.path, "a.rs", "fn alpha() {}\n");
    stage_all(&repo.path);
    let rev = git_commit(&repo.path, "add alpha", "2024-01-01");
    assert!(!rev.is_empty());
    let changes = [Change::new(
        &rev,
        "Ada",
        "2024-01-01",
        "a.rs",
        Some(1),
        Some(0),
    )];
    let rows = expand(&repo.path, &changes);
    assert!(has_entity(&rows, "a.rs::alpha"));
}

#[test]
fn expands_deleted_function_via_parent_blob() {
    let repo = TempRepo::create("delete");
    write_file(&repo.path, "a.rs", "fn alpha() {}\n");
    stage_all(&repo.path);
    assert!(!git_commit(&repo.path, "add", "2024-01-01").is_empty());
    assert!(fs::remove_file(repo.path.join("a.rs")).is_ok());
    stage_all(&repo.path);
    let rev = git_commit(&repo.path, "delete", "2024-01-02");
    assert!(!rev.is_empty());
    let changes = [Change::new(
        &rev,
        "Ada",
        "2024-01-02",
        "a.rs",
        Some(0),
        Some(1),
    )];
    let rows = expand(&repo.path, &changes);
    assert!(rows.iter().any(|c| c.entity == "a.rs::alpha" && c.deleted == Some(1)));
}

#[test]
fn expands_deleted_function_when_path_exists_on_disk() {
    let repo = TempRepo::create("delete-ondisk");
    write_file(&repo.path, "a.rs", "fn alpha() {}\n");
    stage_all(&repo.path);
    assert!(!git_commit(&repo.path, "add", "2024-01-01").is_empty());
    assert!(fs::remove_file(repo.path.join("a.rs")).is_ok());
    stage_all(&repo.path);
    let rev = git_commit(&repo.path, "delete", "2024-01-02");
    assert!(!rev.is_empty());
    write_file(&repo.path, "a.rs", "fn later() {}\n");
    let changes = [Change::new(
        &rev,
        "Ada",
        "2024-01-02",
        "a.rs",
        Some(0),
        Some(1),
    )];
    let rows = expand(&repo.path, &changes);
    assert!(rows.iter().any(|c| c.entity == "a.rs::alpha" && c.deleted == Some(1)));
}

#[test]
fn expands_rename_away_old_path_and_new_path() {
    let repo = TempRepo::create("rename");
    write_file(&repo.path, "old.rs", "fn alpha() {}\n");
    stage_all(&repo.path);
    assert!(!git_commit(&repo.path, "add old", "2024-01-01").is_empty());
    assert!(git_ok(&repo.path, &["mv", "old.rs", "new.rs"]).is_some());
    let rev = git_commit(&repo.path, "rename", "2024-01-02");
    assert!(!rev.is_empty());
    let changes = [
        Change::new(&rev, "Ada", "2024-01-02", "old.rs", Some(0), Some(1)),
        Change::new(&rev, "Ada", "2024-01-02", "new.rs", Some(1), Some(0)),
    ];
    let rows = expand(&repo.path, &changes);
    assert!(has_entity(&rows, "old.rs::alpha"));
    assert!(has_entity(&rows, "new.rs::alpha"));
}

#[test]
fn expands_root_commit_without_parent() {
    let repo = TempRepo::create("root");
    write_file(&repo.path, "a.rs", "fn alpha() {}\n");
    stage_all(&repo.path);
    let rev = git_commit(&repo.path, "root", "2024-01-01");
    assert!(!rev.is_empty());
    let changes = [Change::new(
        &rev,
        "Ada",
        "2024-01-01",
        "a.rs",
        Some(1),
        Some(0),
    )];
    let rows = expand(&repo.path, &changes);
    assert!(has_entity(&rows, "a.rs::alpha"));
}

#[test]
fn duplicate_fn_ids_stable_across_commits() {
    let repo = TempRepo::create("dupes");
    write_file(&repo.path, "a.rs", "fn twin() {}\nfn twin() {}\n");
    stage_all(&repo.path);
    let rev1 = git_commit(&repo.path, "twins", "2024-01-01");
    assert!(!rev1.is_empty());
    write_file(
        &repo.path,
        "a.rs",
        "fn twin() { let _x = 1; }\nfn twin() {}\n",
    );
    stage_all(&repo.path);
    let rev2 = git_commit(&repo.path, "edit first", "2024-01-02");
    assert!(!rev2.is_empty());
    let changes = [
        Change::new(&rev1, "Ada", "2024-01-01", "a.rs", Some(2), Some(0)),
        Change::new(&rev2, "Ada", "2024-01-02", "a.rs", Some(1), Some(1)),
    ];
    let rows = expand(&repo.path, &changes);
    assert!(rows.iter().any(|c| c.rev == rev1 && c.entity == "a.rs::twin#1"));
    assert!(rows.iter().any(|c| c.rev == rev1 && c.entity == "a.rs::twin#2"));
    assert!(rows.iter().any(|c| c.rev == rev2 && c.entity.starts_with("a.rs::twin#")));
    assert!(!rows.iter().any(|c| c.entity == "a.rs::twin"));
}

#[test]
fn attributes_trait_method_only_edit() {
    let repo = TempRepo::create("trait");
    let v1 = "trait Foo {\n    fn bar(&self) {\n        let _a = 1;\n    }\n}\n";
    write_file(&repo.path, "a.rs", v1);
    stage_all(&repo.path);
    assert!(!git_commit(&repo.path, "add trait", "2024-01-01").is_empty());
    let v2 = "trait Foo {\n    fn bar(&self) {\n        let _a = 2;\n    }\n}\n";
    write_file(&repo.path, "a.rs", v2);
    stage_all(&repo.path);
    let rev = git_commit(&repo.path, "edit bar", "2024-01-02");
    assert!(!rev.is_empty());
    let changes = [Change::new(
        &rev,
        "Ada",
        "2024-01-02",
        "a.rs",
        Some(1),
        Some(1),
    )];
    let rows = expand(&repo.path, &changes);
    let entities: Vec<_> = rows.iter().map(|c| c.entity.as_str()).collect();
    assert!(
        has_entity(&rows, "a.rs::Foo::bar"),
        "expected trait method attribution, got: {entities:?}"
    );
    assert!(!rows.iter().any(|c| c.entity == "a.rs"));
}
