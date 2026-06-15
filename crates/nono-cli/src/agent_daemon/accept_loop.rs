//! Multi-tenant capability pipe accept loop.
//!
//! Placeholder — implemented in Plan 74-04 (Wave 2).
//!
//! This module will contain:
//! - `run_accept_loop(daemon_state: Arc<DaemonState>, shutdown: Arc<Notify>)` — the
//!   tokio async accept loop that creates per-tenant named-pipe instances,
//!   authenticates clients via `ImpersonateNamedPipeClient`, and dispatches to
//!   `serve_frames`.
//! - `authenticate_pipe_client` — server-side client identity verification using
//!   the impersonation gate + `AgentRegistry` membership check.
//! - `serve_frames` — the per-connection request/response handler (capability
//!   query only; no capability expansion; ADR-74 Decision 4).
