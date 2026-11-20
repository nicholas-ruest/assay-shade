use std::fs;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let directory = tempdir().unwrap();
    let targets = directory.path().join("targets.bed");
    let blind = directory.path().join("blind.bed");
    fs::write(
        &targets,
        "chr1\t10\t30\tamplicon-a\nchr2\t0\t10\tamplicon-b\n",
    )
    .unwrap();
    fs::write(&blind, "chr1\t15\t20\nchr1\t18\t25\n").unwrap();
    (directory, targets, blind)
}

#[test]
fn audit_emits_json_with_union_coverage() {
    let (_directory, targets, blind) = fixture();
    Command::cargo_bin("assay-shade")
        .unwrap()
        .args([
            "audit",
            "--targets",
            targets.to_str().unwrap(),
            "--blind",
            blind.to_str().unwrap(),
            "--format",
            "json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"blind_bases\": 10"))
        .stdout(predicate::str::contains("\"affected_targets\": 1"));
}

#[test]
fn mask_emits_only_safe_fragments() {
    let (_directory, targets, blind) = fixture();
    Command::cargo_bin("assay-shade")
        .unwrap()
        .args([
            "mask",
            "--targets",
            targets.to_str().unwrap(),
            "--blind",
            blind.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::eq(
            "chr1\t10\t15\tamplicon-a:safe:1\nchr1\t25\t30\tamplicon-a:safe:2\nchr2\t0\t10\tamplicon-b:safe:1\n",
        ));
}

#[test]
fn threshold_failure_is_automation_friendly() {
    let (_directory, targets, blind) = fixture();
    Command::cargo_bin("assay-shade")
        .unwrap()
        .args([
            "audit",
            "--targets",
            targets.to_str().unwrap(),
            "--blind",
            blind.to_str().unwrap(),
            "--fail-above",
            "20",
        ])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::is_empty());
}

#[test]
fn malformed_bed_reports_filename_and_line() {
    let (directory, _targets, blind) = fixture();
    let malformed = directory.path().join("bad.bed");
    fs::write(&malformed, "chr1 10 20\n").unwrap();
    Command::cargo_bin("assay-shade")
        .unwrap()
        .args([
            "audit",
            "--targets",
            malformed.to_str().unwrap(),
            "--blind",
            blind.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bad.bed"))
        .stderr(predicate::str::contains("line 1"));
}
