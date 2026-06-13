//! Build script for nono-cli
//!
//! Embeds policy and hook scripts into the binary at compile time.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Rebuild if data files change.
    //
    // NOTE: `rerun-if-changed=data/` only tracks the directory entry's own mtime,
    // not the files nested inside it. Editing a file under data/hooks/ does NOT
    // update data/'s mtime, so cargo would otherwise keep embedding a STALE copy
    // (a real hazard: it would re-embed the pre-R-A1 vulnerable hook script).
    // Declare explicit per-file directives for every file we embed below so any
    // edit reliably retriggers this build script.
    println!("cargo:rerun-if-changed=data/");
    println!("cargo:rerun-if-changed=data/policy.json");
    println!("cargo:rerun-if-changed=data/network-policy.json");
    println!("cargo:rerun-if-changed=data/hooks/nono-hook.sh");
    println!("cargo:rerun-if-changed=data/hooks/nono-tool-hook.ps1");
    println!("cargo:rerun-if-changed=data/nono-profile.schema.json");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = Path::new(&out_dir);

    // === Bake the compile-time Cargo target root for the dev-build trust gate ===
    // R-B4 fix: the Windows broker Authenticode self-trust-anchor gate is skipped
    // ONLY for genuine local Cargo dev builds. The skip must key off a signal an
    // attacker outside this build cannot forge — NOT a runtime path substring.
    //
    // OUT_DIR has the canonical Cargo layout `<target>/<profile>/build/<pkg>-<hash>/out`.
    // Walking up 4 ancestors yields `<target>` (the Cargo target dir for THIS build,
    // honoring CARGO_TARGET_DIR overrides). We bake its absolute path into the binary.
    // At runtime, `is_dev_build_layout` requires the running exe to live UNDER this
    // exact baked root (component-wise, after canonicalization). An attacker cannot
    // reproduce the developer machine's absolute target path, and a copied binary at
    // e.g. `C:\Users\victim\target\release\nono.exe` is NOT under the baked root.
    //
    // Production MSI/release binaries are built by the signed pipeline and installed
    // to `Program Files\nono\` etc. — never under this baked target dir — so the gate
    // ENFORCES Authenticode there. `cargo test --release` runs test binaries from
    // `<target>/release/deps/`, which IS under the baked root, so the unsigned dev
    // broker is correctly skipped (the documented `#[cfg(debug_assertions)]` hazard
    // is avoided because this is a path-provenance signal, not a build profile flag).
    match out_path.ancestors().nth(4) {
        Some(target_root) => {
            println!(
                "cargo:rustc-env=NONO_DEV_TARGET_ROOT={}",
                target_root.display()
            );
        }
        None => {
            // Fail-closed: if we cannot derive the target root, bake an empty value.
            // The runtime check treats an empty/missing baked root as "no dev skip",
            // so the gate is ENFORCED rather than bypassed.
            println!("cargo:warning=Could not derive Cargo target root from OUT_DIR; broker dev-skip will be disabled");
            println!("cargo:rustc-env=NONO_DEV_TARGET_ROOT=");
        }
    }

    // === Embed policy JSON ===
    let policy_path = Path::new("data/policy.json");
    if policy_path.exists() {
        let content = fs::read_to_string(policy_path).expect("Failed to read policy.json");

        // Write to OUT_DIR for include_str! macro
        fs::write(out_path.join("policy.json"), &content)
            .expect("Failed to write policy.json to OUT_DIR");

        println!("cargo:rustc-env=POLICY_JSON_EMBEDDED=1");
    } else {
        println!("cargo:warning=data/policy.json not found");
        println!("cargo:rustc-env=POLICY_JSON_EMBEDDED=0");
    }

    // === Embed network policy JSON ===
    let net_policy_path = Path::new("data/network-policy.json");
    if net_policy_path.exists() {
        let content =
            fs::read_to_string(net_policy_path).expect("Failed to read network-policy.json");
        fs::write(out_path.join("network-policy.json"), &content)
            .expect("Failed to write network-policy.json to OUT_DIR");
        println!("cargo:rustc-env=NETWORK_POLICY_JSON_EMBEDDED=1");
    } else {
        println!("cargo:warning=data/network-policy.json not found");
        println!("cargo:rustc-env=NETWORK_POLICY_JSON_EMBEDDED=0");
    }

    // === Embed hook scripts ===
    let hook_path = Path::new("data/hooks/nono-hook.sh");
    if hook_path.exists() {
        let content = fs::read_to_string(hook_path).expect("Failed to read hook script");
        fs::write(out_path.join("nono-hook.sh"), &content)
            .expect("Failed to write hook script to OUT_DIR");
    }
    let tool_hook_path = Path::new("data/hooks/nono-tool-hook.ps1");
    if tool_hook_path.exists() {
        let content = fs::read_to_string(tool_hook_path).expect("Failed to read tool hook script");
        fs::write(out_path.join("nono-tool-hook.ps1"), &content)
            .expect("Failed to write tool hook script to OUT_DIR");
    }

    // === Embed profile JSON Schema ===
    let schema_path = Path::new("data/nono-profile.schema.json");
    if schema_path.exists() {
        let content = fs::read_to_string(schema_path).expect("Failed to read profile schema");
        fs::write(out_path.join("nono-profile.schema.json"), &content)
            .expect("Failed to write profile schema to OUT_DIR");
    }

    // === Embed profile authoring guide ===
    let guide_path = Path::new("data/profile-authoring-guide.md");
    if guide_path.exists() {
        let content =
            fs::read_to_string(guide_path).expect("Failed to read profile authoring guide");
        fs::write(out_path.join("profile-authoring-guide.md"), &content)
            .expect("Failed to write profile authoring guide to OUT_DIR");
    }

    // === Stage placeholder Windows WFP driver artifact next to build outputs ===
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let driver_path = Path::new("data/windows/nono-wfp-driver.sys");
        println!("cargo:rerun-if-changed={}", driver_path.display());

        if driver_path.exists() {
            if let Some(profile_dir) = out_path.ancestors().nth(3) {
                fs::copy(driver_path, profile_dir.join("nono-wfp-driver.sys"))
                    .expect("Failed to copy placeholder WFP driver artifact to build output");
            } else {
                println!("cargo:warning=Could not determine target profile dir for placeholder WFP driver artifact");
            }
        } else {
            println!("cargo:warning=data/windows/nono-wfp-driver.sys not found");
        }
    }
}
