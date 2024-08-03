use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_hash_without_prefix_suffix_or_length() {
    let mut cmd = Command::cargo_bin("solidity_signature_collision").unwrap();
    cmd.arg("--target-hash")
        .arg("0x64d98f6e")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found matching signature:"));
}

#[test]
fn test_hash_without_prefix_suffix_with_length_8() {
    let mut cmd = Command::cargo_bin("solidity_signature_collision").unwrap();
    cmd.arg("--target-hash")
        .arg("0x64d98f6e")
        .arg("--length")
        .arg("8")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found matching signature:"));
}

#[test]
fn test_hash_without_prefix_with_length_8_and_suffix_bool() {
    let mut cmd = Command::cargo_bin("solidity_signature_collision").unwrap();
    cmd.arg("--target-hash")
        .arg("0x64d98f6e")
        .arg("--length")
        .arg("8")
        .arg("--suffix")
        .arg("(bool)")
        .assert()
        .success()
        .stdout(predicate::str::contains("Found matching signature:"));
}