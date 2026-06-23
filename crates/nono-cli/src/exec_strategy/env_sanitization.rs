//! Environment sanitization boundary for sandboxed execution.
//!
//! Threat model:
//! - Untrusted parent/shell environments may inject execution behavior via
//!   linker, shell, or interpreter environment variables.
//! - All sandbox execution strategies must share one allow/deny implementation
//!   to avoid drift in security behavior across code paths.

/// Returns true if an environment variable is unsafe to inherit into a sandboxed child.
///
/// Covers linker injection (LD_PRELOAD, DYLD_INSERT_LIBRARIES), shell startup
/// injection (BASH_ENV, PROMPT_COMMAND, IFS), interpreter code/module injection
/// (NODE_OPTIONS, PYTHONPATH, PERL5OPT, RUBYOPT, JAVA_TOOL_OPTIONS, etc.), and
/// Windows hook env-file injection vectors (Low-IL-writer → Medium-IL-reader trust
/// gap, D-09).
pub(crate) fn is_dangerous_env_var(key: &str) -> bool {
    // Linker injection
    key.starts_with("LD_")
        || key.starts_with("DYLD_")
        // AWS credential injection (ZTL-04): strip the entire AWS_* namespace so no
        // AWS credential or config var reaches the sandboxed child process.
        // cfg-UNCONDITIONAL — AWS creds are dangerous on every platform.
        // NOTE: `key.starts_with("AWS_")` is a string-prefix on an env-var NAME,
        // not a path — matches the LD_/DYLD_ precedent; NOT the CLAUDE.md
        // path-starts_with footgun (which concerns filesystem path components).
        || key.starts_with("AWS_")
        // Shell injection
        || key == "BASH_ENV"
        || key == "ENV"
        || key == "CDPATH"
        || key == "GLOBIGNORE"
        || key.starts_with("BASH_FUNC_")
        || key == "PROMPT_COMMAND"
        || key == "IFS"
        // Python injection
        || key == "PYTHONSTARTUP"
        || key == "PYTHONPATH"
        // Node.js injection
        || key == "NODE_OPTIONS"
        || key == "NODE_PATH"
        // Perl injection
        || key == "PERL5OPT"
        || key == "PERL5LIB"
        // Ruby injection
        || key == "RUBYOPT"
        || key == "RUBYLIB"
        || key == "GEM_PATH"
        || key == "GEM_HOME"
        // JVM injection
        || key == "JAVA_TOOL_OPTIONS"
        || key == "_JAVA_OPTIONS"
        // .NET injection
        || key == "DOTNET_STARTUP_HOOKS"
        // Go injection
        || key == "GOFLAGS"
        // 1Password secrets and session tokens — meta-secrets used by
        // the parent to authenticate `op` CLI, must never leak to sandboxed child
        || key == "OP_SERVICE_ACCOUNT_TOKEN"
        || key == "OP_CONNECT_TOKEN"
        || key == "OP_CONNECT_HOST"
        || key.starts_with("OP_SESSION_")
        // Windows hook env-file injection vectors (Low-IL-writer → Medium-IL-reader gap, D-09).
        // On Windows, env-var comparison is case-insensitive; eq_ignore_ascii_case matches
        // all capitalizations. These vars can redirect executable resolution, interpreter
        // search paths, and system directories — a Low-IL hook writing them to the env file
        // could hijack the Medium-IL parent's execution context.
        //
        // WINDOWS-ONLY (cross-target correctness): on Unix env names are case-sensitive and
        // the threat model differs — PATH/TEMP must stay inheritable so the sandboxed child
        // can resolve executables (see test_allows_unrelated_env_vars / test_safe_env_vars_allowed).
        // The D-09 hardening was added unconditionally, which both broke the documented Unix
        // contract and would strip PATH from Unix sandbox children. Gate it to Windows.
        || (cfg!(target_os = "windows")
            && (key.eq_ignore_ascii_case("PATH") // executable resolution hijacking
                || key.eq_ignore_ascii_case("PATHEXT") // extension association hijacking
                || key.eq_ignore_ascii_case("COMSPEC") // cmd interpreter redirect
                || key.eq_ignore_ascii_case("PSModulePath") // PowerShell module injection
                || key.eq_ignore_ascii_case("PSModuleAnalysisCachePath") // PS analysis cache poisoning
                || key.eq_ignore_ascii_case("__PSLockdownPolicy") // PS constrained-language bypass
                || key.eq_ignore_ascii_case("SystemRoot") // system DLL resolution redirect
                || key.eq_ignore_ascii_case("windir") // system directory redirect
                || key.eq_ignore_ascii_case("TEMP") // temp file redirect from parent perspective
                || key.eq_ignore_ascii_case("TMP"))) // same as TEMP
}

fn env_key_matches(left: &str, right: &str) -> bool {
    if cfg!(target_os = "windows") {
        left.eq_ignore_ascii_case(right)
    } else {
        left == right
    }
}

/// Decide whether an inherited env var should be dropped for sandbox execution.
pub(super) fn should_skip_env_var(
    key: &str,
    config_env_vars: &[(&str, &str)],
    blocked_extra: &[&str],
) -> bool {
    config_env_vars
        .iter()
        .any(|(ek, _)| env_key_matches(ek, key))
        || blocked_extra
            .iter()
            .any(|blocked| env_key_matches(blocked, key))
        || is_dangerous_env_var(key)
}

/// Returns true if `key` matches any pattern in `patterns`.
///
/// Supports exact names (`"PATH"`) and prefix patterns ending with `*`
/// (`"AWS_*"` matches `AWS_REGION`, `AWS_SECRET_ACCESS_KEY`, etc.).
/// A bare `"*"` matches everything. The `*` wildcard is only valid as a
/// trailing suffix — patterns like `"A*B"` or `"*X"` are skipped.
///
/// Extracted from `is_env_var_allowed` per upstream v0.52.0 `a022e5c7`
/// (Plan 34-08a Task 6) so both `is_env_var_allowed` and `is_env_var_denied`
/// independently delegate to it, avoiding direct coupling.
fn matches_env_var_patterns(key: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Some(prefix) = pattern.strip_suffix('*') {
            if prefix.contains('*') {
                continue;
            }
            if key.starts_with(prefix) {
                return true;
            }
        } else if !pattern.contains('*') && key == *pattern {
            return true;
        }
    }
    false
}

/// Returns true if an environment variable matches the allow-list.
///
/// Supports exact names (`"PATH"`) and prefix patterns ending with `*`
/// (`"AWS_*"` matches `AWS_REGION`, `AWS_SECRET_ACCESS_KEY`, etc.).
/// A bare `"*"` matches everything.
///
/// Ported from upstream v0.37.0 `1b412a7` per Plan 34-08a Task 3 (D-20
/// manual replay). Refactored to delegate to `matches_env_var_patterns`
/// per upstream v0.52.0 `a022e5c7` (Plan 34-08a Task 6).
/// Wired into Unix (`exec_strategy.rs:435-457`) AND Windows
/// (`exec_strategy_windows/launch.rs::build_child_env`) execution paths.
pub(crate) fn is_env_var_allowed(key: &str, allowed_env_vars: &[String]) -> bool {
    matches_env_var_patterns(key, allowed_env_vars)
}

/// Validates that all env var patterns use `*` only as a trailing suffix.
/// `field_name` is used in the error message (e.g. `"allow_vars"` or `"deny_vars"`).
/// Returns an error message describing the first invalid pattern, or None if valid.
///
/// Ported from upstream v0.37.0 `1b412a7` per Plan 34-08a Task 3 (D-20
/// manual replay); renamed from `validate_allow_vars_pattern` to
/// `validate_env_var_patterns` with a `field_name` parameter per upstream
/// v0.52.0 `3657c935` (Plan 34-08a Task 4).
pub(crate) fn validate_env_var_patterns(patterns: &[String], field_name: &str) -> Option<String> {
    for pattern in patterns {
        if pattern.contains('*') && !pattern.ends_with('*') {
            return Some(format!(
                "Invalid {} pattern '{}': '*' is only valid as a trailing suffix",
                field_name, pattern
            ));
        }
        if pattern.starts_with('*') && pattern.len() > 1 {
            return Some(format!(
                "Invalid {} pattern '{}': use a bare '*' to match all variables, or a specific prefix like 'AWS_*'",
                field_name, pattern
            ));
        }
    }
    None
}

/// Returns true if an environment variable matches the deny-list.
///
/// Uses the same pattern syntax as `is_env_var_allowed`: exact names and
/// trailing-`*` prefix patterns.
///
/// Ported from upstream v0.52.0 `3657c935` per Plan 34-08a Task 4
/// (D-20 manual-replay-by-escalation). Refactored to delegate directly to
/// `matches_env_var_patterns` (avoiding coupling to `is_env_var_allowed`)
/// per upstream v0.52.0 `a022e5c7` (Plan 34-08a Task 6).
/// Wired into Unix (`exec_strategy.rs:435-457`) AND Windows
/// (`exec_strategy_windows/launch.rs::build_child_env`) execution paths.
pub(crate) fn is_env_var_denied(key: &str, denied_env_vars: &[String]) -> bool {
    matches_env_var_patterns(key, denied_env_vars)
}

/// Returns true if `name` is a valid POSIX environment variable name:
/// a non-empty `[A-Za-z_][A-Za-z0-9_]*` with no `=` or NUL.
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validates `environment.set_vars` keys before they are injected into the
/// sandboxed child. Returns an error message describing the first invalid key,
/// or `None` if all keys are acceptable.
///
/// Rejected keys:
/// - `PATH` (reserved; controlled via allow/deny filtering, not injection)
/// - any `NONO_*` key (reserved for nono's own injected variables)
/// - empty or syntactically invalid names (must match `[A-Za-z_][A-Za-z0-9_]*`)
///
/// The dangerous-variable blocklist (`LD_PRELOAD`, `NODE_OPTIONS`, …) is
/// intentionally NOT applied here: that defense targets injection from an
/// untrusted parent shell, whereas `set_vars` is explicit operator intent.
pub(crate) fn validate_set_vars(
    set_vars: &std::collections::HashMap<String, String>,
) -> Option<String> {
    for key in set_vars.keys() {
        if key == "PATH" {
            return Some(
                "Invalid set_vars key 'PATH': PATH is reserved; use allow_vars/deny_vars to \
                 control it"
                    .to_string(),
            );
        }
        if key.starts_with("NONO_") {
            return Some(format!(
                "Invalid set_vars key '{}': the NONO_* prefix is reserved",
                key
            ));
        }
        if !is_valid_env_var_name(key) {
            return Some(format!(
                "Invalid set_vars key '{}': environment variable names must match \
                 [A-Za-z_][A-Za-z0-9_]*",
                key
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // 1Password env var blocklist — security-critical regression tests
    //
    // These vars are credential or session leaks that must NEVER reach a
    // sandboxed child process. If a future refactor accidentally removes one,
    // these tests will catch it.
    // ============================================================================

    #[test]
    fn test_blocks_op_service_account_token() {
        assert!(is_dangerous_env_var("OP_SERVICE_ACCOUNT_TOKEN"));
    }

    #[test]
    fn test_blocks_op_connect_token() {
        assert!(is_dangerous_env_var("OP_CONNECT_TOKEN"));
    }

    #[test]
    fn test_blocks_op_connect_host() {
        assert!(is_dangerous_env_var("OP_CONNECT_HOST"));
    }

    #[test]
    fn test_blocks_op_session_prefix() {
        // OP_SESSION_* vars carry per-account bearer tokens
        assert!(is_dangerous_env_var("OP_SESSION_my_team"));
        assert!(is_dangerous_env_var("OP_SESSION_personal"));
        assert!(is_dangerous_env_var("OP_SESSION_"));
    }

    #[test]
    fn test_allows_unrelated_env_vars() {
        // Env vars that happen to start with "OP" but aren't 1Password
        assert!(!is_dangerous_env_var("OPENAI_API_KEY"));
        assert!(!is_dangerous_env_var("OPERATOR_TOKEN"));
        assert!(!is_dangerous_env_var("OPTIONS"));
        assert!(!is_dangerous_env_var("HOME"));
        // NOTE: PATH is NOT dangerous on non-Windows (case-sensitive, different threat model).
        // On Windows, PATH is dangerous (executable resolution hijacking via D-09 eq_ignore_ascii_case).
        #[cfg(not(target_os = "windows"))]
        assert!(!is_dangerous_env_var("PATH"));
    }

    // ============================================================================
    // AWS credential blocklist (ZTL-04) — security-critical regression tests
    //
    // AWS_* credentials in the parent env MUST NEVER reach a sandboxed child
    // process. The strip is cfg-UNCONDITIONAL (not Windows-only) because AWS
    // credentials are dangerous on every platform. If a future refactor removes
    // the clause, these tests will catch it.
    // ============================================================================

    #[test]
    fn test_blocks_aws_access_key_id() {
        assert!(is_dangerous_env_var("AWS_ACCESS_KEY_ID"));
    }

    #[test]
    fn test_blocks_aws_secret_access_key() {
        assert!(is_dangerous_env_var("AWS_SECRET_ACCESS_KEY"));
    }

    #[test]
    fn test_blocks_aws_session_token() {
        assert!(is_dangerous_env_var("AWS_SESSION_TOKEN"));
    }

    #[test]
    fn test_blocks_aws_region() {
        // Even non-secret AWS vars are stripped: the entire AWS_ namespace is
        // dangerous because it configures the AWS SDK endpoint + credential
        // resolution for the child process.
        assert!(is_dangerous_env_var("AWS_REGION"));
    }

    #[test]
    fn test_blocks_aws_prefix_arbitrary_suffix() {
        // The prefix check covers all current and future AWS_* vars.
        assert!(is_dangerous_env_var("AWS_DEFAULT_REGION"));
        assert!(is_dangerous_env_var("AWS_PROFILE"));
        assert!(is_dangerous_env_var("AWS_ROLE_ARN"));
        assert!(is_dangerous_env_var("AWS_WEB_IDENTITY_TOKEN_FILE"));
        assert!(is_dangerous_env_var("AWS_"));
    }

    #[test]
    fn test_allows_non_aws_unrelated_var() {
        // Vars that start with letters similar to AWS but are not AWS_* must
        // remain allowed (unrelated to the AWS_ strip).
        assert!(!is_dangerous_env_var("HOME"));
        assert!(!is_dangerous_env_var("USER"));
        assert!(!is_dangerous_env_var("LANG"));
    }

    // ============================================================================
    // Existing categories — spot-check that the broader blocklist still works
    // ============================================================================

    #[test]
    fn test_blocks_linker_injection() {
        assert!(is_dangerous_env_var("LD_PRELOAD"));
        assert!(is_dangerous_env_var("DYLD_INSERT_LIBRARIES"));
    }

    #[test]
    fn test_blocks_interpreter_injection() {
        assert!(is_dangerous_env_var("NODE_OPTIONS"));
        assert!(is_dangerous_env_var("PYTHONPATH"));
        assert!(is_dangerous_env_var("RUBYOPT"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_should_skip_env_var_matches_windows_keys_case_insensitively() {
        assert!(should_skip_env_var(
            "ProgramData",
            &[("PROGRAMDATA", r"C:\sandbox\programdata")],
            &[]
        ));
        assert!(should_skip_env_var("Path", &[], &["PATH"]));
    }

    /// Windows hook env-file injection vectors (D-09) — all 10 danger vars must be
    /// blocked case-insensitively. A Low-IL hook writing any of these to the env file
    /// could hijack the Medium-IL parent's execution context.
    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_dangerous_vars_blocked() {
        assert!(is_dangerous_env_var("PATH"));
        assert!(is_dangerous_env_var("Path")); // case-insensitive
        assert!(is_dangerous_env_var("PATHEXT"));
        assert!(is_dangerous_env_var("COMSPEC"));
        assert!(is_dangerous_env_var("PSModulePath"));
        assert!(is_dangerous_env_var("PSModuleAnalysisCachePath"));
        assert!(is_dangerous_env_var("__PSLockdownPolicy"));
        assert!(is_dangerous_env_var("SystemRoot"));
        assert!(is_dangerous_env_var("windir"));
        assert!(is_dangerous_env_var("TEMP"));
        assert!(is_dangerous_env_var("TMP"));
    }

    // ============================================================================
    // Environment variable allow-list — is_env_var_allowed (ported v0.37.0 1b412a7)
    // ============================================================================

    #[test]
    fn test_env_var_allowed_exact_match() {
        let allowed: Vec<String> = vec!["PATH".into(), "HOME".into()];
        assert!(is_env_var_allowed("PATH", &allowed));
        assert!(is_env_var_allowed("HOME", &allowed));
    }

    #[test]
    fn test_env_var_allowed_exact_no_match() {
        let allowed: Vec<String> = vec!["PATH".into(), "HOME".into()];
        assert!(!is_env_var_allowed("SECRET", &allowed));
    }

    #[test]
    fn test_env_var_allowed_prefix_match() {
        let allowed: Vec<String> = vec!["AWS_*".into()];
        assert!(is_env_var_allowed("AWS_REGION", &allowed));
        assert!(is_env_var_allowed("AWS_SECRET_ACCESS_KEY", &allowed));
    }

    #[test]
    fn test_env_var_allowed_prefix_no_match() {
        let allowed: Vec<String> = vec!["AWS_*".into()];
        assert!(!is_env_var_allowed("GCP_REGION", &allowed));
    }

    #[test]
    fn test_env_var_allowed_empty_list() {
        let allowed: Vec<String> = vec![];
        assert!(!is_env_var_allowed("PATH", &allowed));
    }

    #[test]
    fn test_env_var_allowed_bare_star() {
        let allowed: Vec<String> = vec!["*".into()];
        assert!(is_env_var_allowed("ANYTHING", &allowed));
        assert!(is_env_var_allowed("PATH", &allowed));
    }

    #[test]
    fn test_env_var_allowed_prefix_does_not_match_partial() {
        let allowed: Vec<String> = vec!["AWS_*".into()];
        assert!(!is_env_var_allowed("AWS", &allowed));
    }

    #[test]
    fn test_env_var_allowed_prefix_matches_empty_suffix() {
        let allowed: Vec<String> = vec!["AWS_*".into()];
        assert!(is_env_var_allowed("AWS_", &allowed));
    }

    #[test]
    fn test_env_var_allowed_mixed_patterns() {
        let allowed: Vec<String> = vec!["PATH".into(), "AWS_*".into()];
        assert!(is_env_var_allowed("PATH", &allowed));
        assert!(is_env_var_allowed("AWS_REGION", &allowed));
        assert!(!is_env_var_allowed("HOME", &allowed));
    }

    #[test]
    fn test_env_var_allowed_mid_star_ignored() {
        let allowed: Vec<String> = vec!["A*B".into()];
        assert!(!is_env_var_allowed("AXB", &allowed));
        assert!(!is_env_var_allowed("A*B", &allowed));
    }

    // ============================================================================
    // Pattern validation — validate_env_var_patterns (ported v0.37.0 1b412a7,
    // renamed from validate_allow_vars_pattern in v0.52.0 3657c935)
    // ============================================================================

    #[test]
    fn test_validate_valid_patterns() {
        let patterns: Vec<String> = vec!["PATH".into(), "AWS_*".into(), "*".into()];
        assert!(validate_env_var_patterns(&patterns, "allow_vars").is_none());
    }

    #[test]
    fn test_validate_rejects_mid_star() {
        let patterns: Vec<String> = vec!["A*B".into()];
        let err = validate_env_var_patterns(&patterns, "allow_vars");
        assert!(err.is_some());
        assert!(err.as_ref().is_some_and(|e| e.contains("A*B")));
    }

    #[test]
    fn test_validate_rejects_leading_star_with_suffix() {
        let patterns: Vec<String> = vec!["*X".into()];
        let err = validate_env_var_patterns(&patterns, "allow_vars");
        assert!(err.is_some());
        assert!(err.as_ref().is_some_and(|e| e.contains("*X")));
    }

    #[test]
    fn test_validate_accepts_bare_star() {
        let patterns: Vec<String> = vec!["*".into()];
        assert!(validate_env_var_patterns(&patterns, "allow_vars").is_none());
    }

    #[test]
    fn test_validate_exact_name_no_star() {
        let patterns: Vec<String> = vec!["PATH".into()];
        assert!(validate_env_var_patterns(&patterns, "allow_vars").is_none());
    }

    #[test]
    fn test_validate_deny_vars_field_name_in_error() {
        let patterns: Vec<String> = vec!["A*B".into()];
        let err = validate_env_var_patterns(&patterns, "deny_vars");
        assert!(err.as_ref().is_some_and(|e| e.contains("deny_vars")));
        assert!(err.as_ref().is_some_and(|e| e.contains("A*B")));
    }

    // ============================================================================
    // is_env_var_denied (ported v0.52.0 3657c935)
    // ============================================================================

    #[test]
    fn test_env_var_denied_exact_match() {
        let denied: Vec<String> = vec!["GH_TOKEN".into(), "ANTHROPIC_API_KEY".into()];
        assert!(is_env_var_denied("GH_TOKEN", &denied));
        assert!(is_env_var_denied("ANTHROPIC_API_KEY", &denied));
    }

    #[test]
    fn test_env_var_denied_prefix_match() {
        let denied: Vec<String> = vec!["GITHUB_*".into()];
        assert!(is_env_var_denied("GITHUB_TOKEN", &denied));
        assert!(is_env_var_denied("GITHUB_ACTIONS", &denied));
        assert!(!is_env_var_denied("GH_TOKEN", &denied));
    }

    #[test]
    fn test_env_var_denied_no_match() {
        let denied: Vec<String> = vec!["GH_TOKEN".into()];
        assert!(!is_env_var_denied("PATH", &denied));
        assert!(!is_env_var_denied("HOME", &denied));
    }

    #[test]
    fn test_env_var_denied_empty_list() {
        let denied: Vec<String> = vec![];
        assert!(!is_env_var_denied("GH_TOKEN", &denied));
    }

    #[test]
    fn test_env_var_denied_overrides_allowed() {
        // Simulates: deny_vars has GH_TOKEN, allow_vars has GH_TOKEN
        // deny wins: denied should return true regardless of allowed
        let denied: Vec<String> = vec!["GH_TOKEN".into()];
        let allowed: Vec<String> = vec!["GH_TOKEN".into()];
        assert!(is_env_var_denied("GH_TOKEN", &denied));
        assert!(is_env_var_allowed("GH_TOKEN", &allowed));
        // In exec path, deny is checked before allow, so GH_TOKEN is stripped
    }

    // ============================================================================
    // set_vars validation
    // ============================================================================

    fn set_vars_from(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn test_set_vars_accepts_normal_keys() {
        let set_vars = set_vars_from(&[("RUST_LOG", "debug"), ("MY_VAR", "value")]);
        assert_eq!(validate_set_vars(&set_vars), None);
    }

    #[test]
    fn test_set_vars_accepts_dangerous_keys() {
        // set_vars is explicit operator intent: dangerous keys are NOT blocked.
        let set_vars = set_vars_from(&[("LD_PRELOAD", "/tmp/x.so")]);
        assert_eq!(validate_set_vars(&set_vars), None);
        let set_vars = set_vars_from(&[("NODE_OPTIONS", "--max-old-space-size=4096")]);
        assert_eq!(validate_set_vars(&set_vars), None);
        let set_vars = set_vars_from(&[("DYLD_INSERT_LIBRARIES", "/tmp/x.dylib")]);
        assert_eq!(validate_set_vars(&set_vars), None);
    }

    #[test]
    fn test_set_vars_rejects_path() {
        let set_vars = set_vars_from(&[("PATH", "/usr/bin")]);
        assert!(validate_set_vars(&set_vars).is_some());
    }

    #[test]
    fn test_set_vars_rejects_nono_prefix() {
        let set_vars = set_vars_from(&[("NONO_FOO", "bar")]);
        assert!(validate_set_vars(&set_vars).is_some());
        let set_vars = set_vars_from(&[("NONO_CAP_FILE", "/tmp/cap")]);
        assert!(validate_set_vars(&set_vars).is_some());
    }

    #[test]
    fn test_set_vars_rejects_invalid_names() {
        // empty name
        assert!(validate_set_vars(&set_vars_from(&[("", "v")])).is_some());
        // leading digit
        assert!(validate_set_vars(&set_vars_from(&[("1FOO", "v")])).is_some());
        // contains '='
        assert!(validate_set_vars(&set_vars_from(&[("A=B", "v")])).is_some());
        // contains a dash
        assert!(validate_set_vars(&set_vars_from(&[("MY-VAR", "v")])).is_some());
    }

    #[test]
    fn test_set_vars_empty_is_ok() {
        let set_vars: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        assert_eq!(validate_set_vars(&set_vars), None);
    }
}
