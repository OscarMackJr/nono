use crate::cli::SandboxArgs;
use crate::launch_runtime::ProxyLaunchOptions;
use crate::network_policy;
use crate::profile::AllowDomainEntry;
use crate::sandbox_prepare::{validate_external_proxy_bypass, PreparedSandbox};
use nono::{CapabilitySet, NonoError, Result};
use tracing::info;
use tracing::warn;

pub(crate) struct ActiveProxyRuntime {
    pub(crate) env_vars: Vec<(String, String)>,
    pub(crate) handle: Option<nono_proxy::server::ProxyHandle>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EffectiveProxySettings {
    pub(crate) network_profile: Option<String>,
    pub(crate) allow_domain: Vec<AllowDomainEntry>,
    pub(crate) credentials: Vec<String>,
}

/// Parse a `--allow-domain` CLI argument that may be a bare hostname or a URL with path.
///
/// - Plain hostname (e.g. `api.openai.com`) → `AllowDomainEntry::Plain`
/// - URL with non-root path (e.g. `https://api.github.com/repos/my-org/**`) →
///   `AllowDomainEntry::WithEndpoints` with a single wildcard-method rule
/// - URL with root or empty path → `AllowDomainEntry::Plain` (no endpoint restriction)
/// - Unparseable input → `AllowDomainEntry::Plain` (fallback)
///
/// Upstream-commit: 75b2265
fn parse_allow_domain_arg(input: &str) -> AllowDomainEntry {
    if let Ok(parsed) = url::Url::parse(input) {
        let domain = parsed.host_str().unwrap_or(input).to_string();
        let path = parsed.path();
        if path.is_empty() || path == "/" {
            AllowDomainEntry::Plain(domain)
        } else {
            AllowDomainEntry::WithEndpoints {
                domain,
                endpoints: vec![nono_proxy::config::EndpointRule {
                    method: "*".to_string(),
                    path: path.to_string(),
                }],
            }
        }
    } else {
        AllowDomainEntry::Plain(input.to_string())
    }
}

pub(crate) fn prepare_proxy_launch_options(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
    silent: bool,
) -> Result<ProxyLaunchOptions> {
    validate_external_proxy_bypass(args, prepared)?;

    let effective_proxy = resolve_effective_proxy_settings(args, prepared);
    let network_profile = effective_proxy.network_profile;
    let allow_domain = effective_proxy.allow_domain;
    let credentials = effective_proxy.credentials;
    let allow_bind_ports = merge_dedup_ports(&prepared.listen_ports, &args.allow_bind);

    let upstream_proxy = if args.allow_net {
        None
    } else {
        args.external_proxy
            .clone()
            .or_else(|| prepared.upstream_proxy.clone())
    };

    let upstream_bypass = if args.allow_net {
        Vec::new()
    } else if args.external_proxy.is_some() {
        args.external_proxy_bypass.clone()
    } else {
        let mut bypass = prepared.upstream_bypass.clone();
        bypass.extend(args.external_proxy_bypass.clone());
        bypass
    };

    let active = if matches!(prepared.caps.network_mode(), nono::NetworkMode::Blocked) {
        if !credentials.is_empty()
            || network_profile.is_some()
            || !allow_domain.is_empty()
            || upstream_proxy.is_some()
        {
            warn!(
                "--block-net is active; ignoring proxy configuration \
                 that would re-enable network access"
            );
            if !silent {
                eprintln!(
                    "  [nono] Warning: --block-net overrides proxy/credential settings. \
                     Network remains fully blocked."
                );
            }
        }
        false
    } else {
        matches!(
            prepared.caps.network_mode(),
            nono::NetworkMode::ProxyOnly { .. }
        ) || !credentials.is_empty()
            || network_profile.is_some()
            || !allow_domain.is_empty()
            || upstream_proxy.is_some()
    };

    Ok(ProxyLaunchOptions {
        active,
        network_profile,
        allow_domain,
        credentials,
        custom_credentials: prepared.custom_credentials.clone(),
        upstream_proxy,
        upstream_bypass,
        allow_bind_ports,
        proxy_port: args.proxy_port,
        open_url_origins: prepared.open_url_origins.clone(),
        open_url_allow_localhost: prepared.open_url_allow_localhost,
        allow_launch_services_active: prepared.allow_launch_services_active,
    })
}

pub(crate) fn resolve_effective_proxy_settings(
    args: &SandboxArgs,
    prepared: &PreparedSandbox,
) -> EffectiveProxySettings {
    if args.allow_net {
        return EffectiveProxySettings {
            network_profile: None,
            allow_domain: Vec::new(),
            credentials: Vec::new(),
        };
    }

    let network_profile = args
        .network_profile
        .clone()
        .or_else(|| prepared.network_profile.clone());
    // Convert PreparedSandbox allow_domain (Vec<String>) to Vec<AllowDomainEntry> via
    // parse_allow_domain_arg, then extend with CLI --allow-domain args also parsed.
    let mut allow_domain: Vec<AllowDomainEntry> = prepared
        .allow_domain
        .iter()
        .map(|s| parse_allow_domain_arg(s))
        .collect();
    allow_domain.extend(args.allow_proxy.iter().map(|s| parse_allow_domain_arg(s)));
    let mut credentials = prepared.credentials.clone();
    credentials.extend(args.proxy_credential.clone());

    EffectiveProxySettings {
        network_profile,
        allow_domain,
        credentials,
    }
}

pub(crate) fn merge_dedup_ports(a: &[u16], b: &[u16]) -> Vec<u16> {
    let mut ports = a.to_vec();
    ports.extend_from_slice(b);
    ports.sort_unstable();
    ports.dedup();
    ports
}

pub(crate) fn build_proxy_config_from_flags(
    proxy: &ProxyLaunchOptions,
) -> Result<nono_proxy::config::ProxyConfig> {
    let net_policy_json = crate::config::embedded::embedded_network_policy_json();
    let net_policy = network_policy::load_network_policy(net_policy_json)?;

    let mut resolved = if let Some(ref profile_name) = proxy.network_profile {
        network_policy::resolve_network_profile(&net_policy, profile_name)?
    } else {
        network_policy::ResolvedNetworkPolicy {
            hosts: Vec::new(),
            suffixes: Vec::new(),
            routes: Vec::new(),
            profile_credentials: Vec::new(),
        }
    };

    let mut all_credentials = resolved.profile_credentials.clone();
    for cred in &proxy.credentials {
        if !all_credentials.contains(cred) {
            all_credentials.push(cred.clone());
        }
    }

    let routes = network_policy::resolve_credentials(
        &net_policy,
        &all_credentials,
        &proxy.custom_credentials,
    )?;
    resolved.routes = routes;

    // Partition allow_domain entries into plain hosts and endpoint-scoped routes.
    // C5 rider (22e6c40): also push endpoint route upstreams into plain_hosts so
    // the proxy filter allowlist allows upstream TCP connections for TLS-intercept routes.
    let (mut plain_hosts, endpoint_routes) =
        network_policy::partition_allow_domain(&net_policy, &proxy.allow_domain)?;
    for route in &endpoint_routes {
        if let Some(hp) = route.upstream.strip_prefix("https://") {
            plain_hosts.push(hp.to_string());
        } else if let Some(hp) = route.upstream.strip_prefix("http://") {
            plain_hosts.push(hp.to_string());
        }
    }
    resolved.routes.extend(endpoint_routes);
    let mut proxy_config = network_policy::build_proxy_config(&resolved, &plain_hosts);

    if let Some(ref addr) = proxy.upstream_proxy {
        proxy_config.external_proxy = Some(nono_proxy::config::ExternalProxyConfig {
            address: addr.clone(),
            auth: None,
            bypass_hosts: proxy.upstream_bypass.clone(),
        });
    }

    if let Some(port) = proxy.proxy_port {
        proxy_config.bind_port = port;
    }

    Ok(proxy_config)
}

pub(crate) fn start_proxy_runtime(
    proxy: &ProxyLaunchOptions,
    caps: &mut CapabilitySet,
) -> Result<ActiveProxyRuntime> {
    if !proxy.active {
        return Ok(ActiveProxyRuntime {
            env_vars: Vec::new(),
            handle: None,
        });
    }

    let mut proxy_config = build_proxy_config_from_flags(proxy)?;
    proxy_config.direct_connect_ports = caps.tcp_connect_ports().to_vec();
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy runtime: {}", e)))?;
    let handle = rt
        .block_on(async { nono_proxy::server::start(proxy_config.clone()).await })
        .map_err(|e| NonoError::SandboxInit(format!("Failed to start proxy: {}", e)))?;

    let port = handle.port;
    if proxy.allow_bind_ports.is_empty() {
        info!("Network proxy started on localhost:{}", port);
    } else {
        info!(
            "Network proxy started on localhost:{}, bind ports: {:?}",
            port, proxy.allow_bind_ports
        );
    }
    caps.set_network_mode_mut(nono::NetworkMode::ProxyOnly {
        port,
        bind_ports: proxy.allow_bind_ports.clone(),
    });

    let mut env_vars: Vec<(String, String)> = Vec::new();
    for (key, value) in handle.env_vars() {
        env_vars.push((key, value));
    }

    for (key, value) in handle.credential_env_vars(&proxy_config) {
        env_vars.push((key, value));
    }

    std::mem::forget(rt);

    Ok(ActiveProxyRuntime {
        env_vars,
        handle: Some(handle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::AllowDomainEntry;

    /// Upstream-commit: 75b2265 — parse_allow_domain_arg tests
    #[test]
    fn parse_allow_domain_plain_hostname() {
        let entry = parse_allow_domain_arg("api.openai.com");
        assert_eq!(entry, AllowDomainEntry::Plain("api.openai.com".to_string()));
    }

    #[test]
    fn parse_allow_domain_url_with_path_produces_with_endpoints() {
        let entry = parse_allow_domain_arg("https://api.github.com/repos/my-org/**");
        assert_eq!(
            entry,
            AllowDomainEntry::WithEndpoints {
                domain: "api.github.com".to_string(),
                endpoints: vec![nono_proxy::config::EndpointRule {
                    method: "*".to_string(),
                    path: "/repos/my-org/**".to_string(),
                }],
            }
        );
    }

    #[test]
    fn parse_allow_domain_url_with_root_path_produces_plain() {
        let entry = parse_allow_domain_arg("https://api.github.com/");
        assert_eq!(
            entry,
            AllowDomainEntry::Plain("api.github.com".to_string())
        );
    }

    #[test]
    fn parse_allow_domain_url_no_path_produces_plain() {
        let entry = parse_allow_domain_arg("https://api.github.com");
        assert_eq!(
            entry,
            AllowDomainEntry::Plain("api.github.com".to_string())
        );
    }

    #[test]
    fn parse_allow_domain_unparseable_input_falls_back_to_plain() {
        let entry = parse_allow_domain_arg("not-a-url");
        assert_eq!(entry, AllowDomainEntry::Plain("not-a-url".to_string()));
    }
}
