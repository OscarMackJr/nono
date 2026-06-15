//! Per-agent launch orchestration.
//!
//! Placeholder — implemented in Plan 74-04 (Wave 2).
//!
//! This module will contain:
//! - `launch_agent(daemon_state: &Arc<DaemonState>, req: LaunchRequest)` —
//!   orchestrates the Phase 71 broker-arm launch path to create a confined
//!   `AppContainer` child, mint the job object with `KILL_ON_JOB_CLOSE`, insert
//!   the minted package SID into `AgentRegistry`, and build an `AgentTenant`
//!   owning struct that is inserted into `DaemonState::tenants`.
//! - The spawn sequence calls `create_process_containment` /
//!   `apply_process_handle_to_containment` from
//!   `exec_strategy_windows/launch.rs` and then `AgentRegistry::insert` before
//!   the process is resumed (fail-secure ordering: registry entry exists before
//!   the agent can issue any pipe requests).
