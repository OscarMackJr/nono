//! Integration tests for supervisor-side audit attestation.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn nono_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nono"))
}

fn run_nono(args: &[&str], home: &Path, state: &Path, cwd: &Path) -> Output {
    nono_bin()
        .args(args)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_STATE_HOME", state)
        .current_dir(cwd)
        .output()
        .expect("failed to run nono")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn setup_isolated_home() -> (
    tempfile::TempDir,
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    PathBuf,
) {
    // The fake HOME's protected `.nono` state root and sensitive-path deny rules
    // must not fall under ANY path the sandbox grants. Two grants are in play and
    // they pull in opposite directions, so HOME and the --allow-cwd workspace must
    // live in DIFFERENT trees:
    //
    //  - `nono run --allow-cwd` grants the workspace's project (repo) root. If the
    //    workspace lived under this repo, that grant would be the whole repo root,
    //    overlapping a fake HOME placed under the repo (fatal on Linux — Landlock
    //    cannot deny-within-allow — and refused on macOS as a protected-state-root
    //    overlap). So the workspace goes in the SYSTEM temp dir, which has no git
    //    root above it: --allow-cwd then grants only the workspace itself.
    //  - On macOS the system temp dir (/private/var/folders, /tmp) is read-granted
    //    by `group:system_read_macos` (policy.json grants /private/var and /tmp),
    //    which would overlap a fake HOME placed there. So HOME/state go UNDER the
    //    repo (/Users/... — not a system_read path), kept disjoint from the
    //    system-temp workspace grant.
    let repo_artifacts = std::env::current_dir()
        .expect("cwd")
        .join("target")
        .join("test-artifacts");
    fs::create_dir_all(&repo_artifacts).expect("create repo artifacts root");
    let home_state_tmp = tempfile::Builder::new()
        .prefix("nono-audit-attestation-it-")
        .tempdir_in(&repo_artifacts)
        .expect("home/state tempdir");
    let home = home_state_tmp.path().join("home");
    let state = home_state_tmp.path().join("state");

    let workspace_tmp = tempfile::Builder::new()
        .prefix("nono-audit-attestation-ws-")
        .tempdir()
        .expect("workspace tempdir");
    let workspace = workspace_tmp.path().join("workspace");

    fs::create_dir_all(home.join(".config")).expect("create config dir");
    fs::create_dir_all(&state).expect("create state dir");
    fs::create_dir_all(&workspace).expect("create workspace dir");
    (home_state_tmp, workspace_tmp, home, state, workspace)
}

fn audit_root(state: &Path) -> PathBuf {
    state.join("nono").join("audit")
}

fn key_path(home: &Path) -> PathBuf {
    home.join("audit-signing-key.pk8.b64")
}

fn pub_key_path_for_file(private_key_path: &Path) -> PathBuf {
    let mut pub_path = private_key_path.as_os_str().to_owned();
    pub_path.push(".pub");
    PathBuf::from(pub_path)
}

fn generate_file_signing_key(home: &Path, state: &Path, cwd: &Path) -> PathBuf {
    let key_path = key_path(home);
    let keyref = format!("file://{}", key_path.display());
    let output = run_nono(
        &["trust", "keygen", "--force", "--keyref", &keyref],
        home,
        state,
        cwd,
    );
    assert_success(&output);
    assert!(key_path.exists(), "private key should exist");
    assert!(
        pub_key_path_for_file(&key_path).exists(),
        "public key should exist"
    );
    key_path
}

fn only_audit_session_id(state: &Path) -> String {
    let audit_root = audit_root(state);
    let mut session_ids: Vec<String> = fs::read_dir(&audit_root)
        .expect("read audit root")
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            Some(entry.file_name().to_string_lossy().to_string())
        })
        .collect();
    session_ids.sort();
    assert_eq!(session_ids.len(), 1, "expected exactly one audit session");
    session_ids.remove(0)
}

#[test]
fn audit_verify_reports_signed_attestation_with_pinned_public_key() {
    let (_home_tmp, _workspace_tmp, home, state, workspace) = setup_isolated_home();
    let key_path = generate_file_signing_key(&home, &state, &workspace);
    let keyref = format!("file://{}", key_path.display());

    let run_output = run_nono(
        &[
            "run",
            "--audit-integrity",
            "--allow-cwd",
            "--audit-sign-key",
            &keyref,
            "--",
            "/bin/pwd",
        ],
        &home,
        &state,
        &workspace,
    );
    assert_success(&run_output);

    let session_id = only_audit_session_id(&state);
    let pub_key_path = format!("{}", pub_key_path_for_file(&key_path).display());
    let verify_output = run_nono(
        &[
            "audit",
            "verify",
            &session_id,
            "--public-key-file",
            &pub_key_path,
            "--json",
        ],
        &home,
        &state,
        &workspace,
    );
    assert_success(&verify_output);

    let json: Value = serde_json::from_slice(&verify_output.stdout).expect("parse verify json");
    // `nono audit verify --json` (fork shape, see audit_commands.rs::cmd_verify):
    //   { "integrity": <AuditVerificationResult>,
    //     "attestation_present": <bool>,
    //     "attestation_valid": <bool|null> }
    // The upstream `session`/`ledger`/`attestation` envelope was never wired in
    // this fork (the append-only ledger module `audit_ledger.rs` is not declared
    // in the crate, so there is no `ledger` object to assert against). The
    // integrity result carries the record/merkle checks; `attestation_valid` is
    // the AND of every attestation sub-check (present, key_id_matches,
    // signature_verified, merkle_root_matches, session_id_matches, no error) via
    // `AuditAttestationVerificationResult::is_valid()`, so asserting it `true` is
    // as strong as the per-field upstream assertions.
    assert_eq!(json["integrity"]["records_verified"], true);
    assert_eq!(json["integrity"]["merkle_root_matches"], true);
    assert_eq!(json["integrity"]["event_count_matches"], true);
    assert_eq!(json["attestation_present"], true);
    assert_eq!(json["attestation_valid"], true);
}

#[test]
fn rollback_signed_session_verifies_from_audit_dir_bundle() {
    let (_home_tmp, _workspace_tmp, home, state, workspace) = setup_isolated_home();
    fs::write(workspace.join("tracked.txt"), "before\n").expect("write tracked file");
    let key_path = generate_file_signing_key(&home, &state, &workspace);
    let keyref = format!("file://{}", key_path.display());

    let run_output = run_nono(
        &[
            "run",
            "--audit-integrity",
            "--allow-cwd",
            "--rollback",
            "--no-rollback-prompt",
            "--audit-sign-key",
            &keyref,
            "--",
            "/bin/pwd",
        ],
        &home,
        &state,
        &workspace,
    );
    assert_success(&run_output);

    let session_id = only_audit_session_id(&state);
    let audit_dir = audit_root(&state).join(&session_id);
    let rollback_dir = state.join("nono").join("rollbacks").join(&session_id);
    assert!(
        audit_dir.join("audit-attestation.bundle").exists(),
        "bundle should live in audit dir"
    );
    assert!(
        !rollback_dir.join("audit-attestation.bundle").exists(),
        "bundle should not be required in rollback dir"
    );

    let verify_output = run_nono(
        &["audit", "verify", &session_id, "--json"],
        &home,
        &state,
        &workspace,
    );
    assert_success(&verify_output);

    let json: Value = serde_json::from_slice(&verify_output.stdout).expect("parse verify json");
    // Fork verify-JSON shape (see audit_commands.rs::cmd_verify). `attestation_valid`
    // is the AND of signature_verified, merkle_root_matches, session_id_matches,
    // key_id_matches and a clean verification_error, so `true` here means the signed
    // bundle in the audit dir verified end-to-end against the recorded session.
    assert_eq!(json["attestation_present"], true);
    assert_eq!(json["attestation_valid"], true);
}
