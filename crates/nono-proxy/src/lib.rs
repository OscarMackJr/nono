//! Network filtering proxy for the nono sandbox.
//!
//! `nono-proxy` provides three proxy modes:
//!
//! 1. **CONNECT tunnel** (`connect`) - Host-filtered HTTPS tunnelling.
//!    The proxy validates the target host against an allowlist and cloud
//!    metadata deny list, then establishes a raw TCP tunnel.
//!
//! 2. **Reverse proxy** (`reverse`) - Credential injection for API calls.
//!    Requests arrive at `http://127.0.0.1:<port>/<service>/...`, the proxy
//!    injects the real API credential and forwards to the upstream.
//!
//! 3. **External proxy** (`external`) - Enterprise proxy passthrough.
//!    CONNECT requests are chained through a corporate proxy with the
//!    default deny list enforced as a floor.
//!
//! The proxy runs **unsandboxed** in the supervisor process. The sandboxed
//! child can only reach `localhost:<port>` via `NetworkMode::ProxyOnly`.

pub mod audit;
mod auth;
pub mod capture;
pub mod config;
pub mod connect;
pub mod credential;
pub mod diagnostic;
pub mod error;
pub mod external;
pub mod filter;
pub mod oauth2;
pub mod pool;
pub mod reverse;
pub mod route;
pub mod server;
/// SPIFFE/SPIRE Workload API credential sources.
///
/// Made `pub` (upstream `c831dade`/#1272 has `pub mod spiffe;` too) so
/// `crates/nono-proxy/tests/spiffe_integration.rs` (Plan 113-07) can construct
/// `SpiffeJwtSource` directly and call `delegation_from_jwt` for its
/// `SPIRE_AGENT_SOCKET`-gated live tests — an external integration-test binary
/// cannot see a private (`mod`) module. `SpiffeJwtSource::connect` already
/// fails closed on an unreachable socket and its `Debug` impl is redacted
/// (`spiffe.rs`), so widening this to `pub` adds a construction surface, not a
/// secret-disclosure one.
pub mod spiffe;
pub mod token;

pub use config::ProxyConfig;
pub use credential::{CredentialLoadOutcome, CredentialStore};
pub use diagnostic::{ProxyDiagnostic, ProxyDiagnosticCode, ProxyDiagnosticSeverity};
pub use error::{ProxyError, Result};
pub use server::{start, ProxyHandle};
