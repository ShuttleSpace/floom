//! Discover services on the local network — the mDNS / DNS-SD question.
//!
//! This is `multicast_dns`'s question, and the catalog's C6
//! (`docs/plugins.md`): "who on this LAN is serving `_http._tcp`?" —
//! find the printer, the cast target, the other player, without knowing an
//! address beforehand.
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # Why there are no platform arms
//!
//! mDNS is a *protocol* (RFC 6762/6763), not a platform API: the wire format
//! is the same everywhere, so one implementation serves every target this
//! crate compiles for — the way `std`'s sockets serve
//! `floom-plugin-network-info`'s `local_ip`. There is no `android.rs` to rot
//! and no host-inert arm to keep honest; the platform-specific socket code
//! lives inside `mdns-sd`, and the cross-target checks compile it for the
//! phone and Windows targets where a host build never would.
//!
//! # What `browse` promises
//!
//! A bounded window, not a stream: send the questions, collect what
//! resolves before the deadline, return one snapshot. Within the window a
//! re-resolved instance replaces its earlier answer and a removed instance
//! leaves the result — the snapshot is *as of the deadline*, which is the
//! only honest thing a local-network census can say.
//!
//! An empty result is "nobody answered in the window": a real answer, not a
//! failure. On a phone it can additionally mean the Android caveat below.
//!
//! # The Android caveat, named
//!
//! Android's Wi-Fi stack may filter multicast packets outright; the
//! platform's remedy is `WifiManager.MulticastLock` — a Java API that needs
//! the `CHANGE_WIFI_MULTICAST_STATE` permission in the app's manifest, which
//! the `hasCode="false"` example apps do not declare. This v1 browses
//! without the lock: on a phone, an empty result may mean filtered multicast
//! rather than an empty network. The follow-up arm is the platform JNI
//! bridge acquiring the lock for an app that declares the permission — a
//! door named here so the next person finds it.
//!
//! # What this crate deliberately does not do
//!
//! No *advertising* ("tell the network I am here"): Flutter's package does
//! not either, and answering is a server posture with its own lifecycle — a
//! different crate if it is ever wanted. No continuous monitoring: an
//! unbounded stream of events is a UI concern (a `floom-signal` fed by a
//! poll of this function), not a query API's job. No unicast DNS-SD, no
//! sub-type browsing, no hostname resolution — the catalog's question is
//! the plain one, and v1 answers it.

use std::collections::{BTreeMap, HashSet};
use std::net::IpAddr;
use std::time::{Duration, Instant};

use mdns_sd::{RecvTimeoutError, ResolvedService, ServiceDaemon, ServiceEvent};

/// One discovered service instance, as resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Service {
    /// The instance's full name, `"printer2._ipp._tcp.local."`.
    pub fullname: String,
    /// The service type browsed, `"_ipp._tcp.local."`.
    pub service_type: String,
    /// The instance label — the part before the service type, `"printer2"`.
    pub instance: String,
    /// The host the instance runs on, `"printer2.local."`.
    pub host: String,
    /// The port on that host.
    pub port: u16,
    /// Every address the host resolved to, in the order `BTreeMap`'s keys
    /// imply — the caller picks, not this crate.
    pub addresses: Vec<IpAddr>,
    /// The instance's TXT properties, decoded. The TXT format's odd cases
    /// (binary values, duplicate keys) become `String::from_utf8_lossy` and
    /// last-write-wins here, which is what a display caller wants; a
    /// byte-exact caller has the wrong crate.
    pub properties: BTreeMap<String, String>,
}

/// Browse the local network for instances of `service_type`, for `duration`.
///
/// `service_type` is the wire name — `"_http._tcp.local."`, with the leading
/// underscore and the trailing domain; a bare `"http"` is refused before any
/// packet is sent, because the platform would answer a different question
/// than the caller asked.
///
/// The daemon is created for the call and shut down after it: discovery is a
/// question with an answer, not a connection the app holds. Creating it
/// binds UDP 5353 — two `browse` calls in one process, or a browse beside
/// the system's own responder, coexist through the reuse the protocol
/// requires.
pub fn browse(service_type: &str, duration: Duration) -> Result<Vec<Service>, String> {
    if !service_type.starts_with('_') || !service_type.ends_with(".local.") {
        return Err(format!(
            "mdns: `{service_type}` is not a service type — the wire name is \
             `_name._tcp.local.` or `_name._udp.local.`; the leading underscore and the \
             trailing domain are what make it one"
        ));
    }
    if duration.is_zero() {
        return Err("mdns: a browse of zero duration asks nothing".into());
    }

    let daemon = ServiceDaemon::new()
        .map_err(|e| format!("mdns: could not start the discovery daemon: {e}"))?;
    let receiver = daemon
        .browse(service_type)
        .map_err(|e| format!("mdns: could not start browsing {service_type}: {e}"))?;

    let deadline = Instant::now() + duration;
    // Keyed by fullname so a re-resolution *replaces* (network answers can
    // arrive more than once, with fresher addresses) instead of duplicating.
    let mut resolved: BTreeMap<String, Service> = BTreeMap::new();
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                let service = Service::from_resolved(info.as_ref());
                resolved.insert(service.fullname.clone(), service);
            }
            Ok(ServiceEvent::ServiceRemoved(_ty, fullname)) => {
                // Gone before the deadline: the snapshot is as-of the end,
                // not as-of the moment of discovery.
                resolved.remove(&fullname);
            }
            // SearchStarted / ServiceFound (resolution pending) and
            // SearchStopped, plus any variant a newer `mdns-sd` adds: none
            // of them is an answer, and a browse that silently survived an
            // upgrade is worth more than a wildcard-free match that breaks.
            Ok(_) => {}
            Err(RecvTimeoutError::Timeout) => break,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    // Both are cleanup of a per-call daemon; by now the answer is in hand
    // and shutdown noise must not turn a successful browse into an `Err`.
    let _ = daemon.stop_browse(service_type);
    let _ = daemon.shutdown();
    Ok(resolved.into_values().collect())
}

impl Service {
    fn from_resolved(info: &ResolvedService) -> Service {
        // "printer2._ipp._tcp.local." minus "._ipp._tcp.local." is
        // "printer2" — the instance label a caller prints.
        let instance = info
            .fullname
            .strip_suffix(&info.ty_domain)
            .unwrap_or(&info.fullname)
            .trim_end_matches('.')
            .to_string();

        // A HashSet of scoped addresses has no meaningful order; sorted
        // output is the one order two calls can agree on.
        let addresses: Vec<IpAddr> = {
            let mut set: HashSet<IpAddr> = HashSet::new();
            for scoped in &info.addresses {
                set.insert(scoped.to_ip_addr());
            }
            let mut list: Vec<IpAddr> = set.into_iter().collect();
            list.sort();
            list
        };

        let properties = info
            .txt_properties
            .iter()
            .map(|property| (property.key().to_string(), property.val_str().to_string()))
            .collect();

        Service {
            fullname: info.fullname.clone(),
            service_type: info.ty_domain.clone(),
            instance,
            host: info.host.clone(),
            port: info.port,
            addresses,
            properties,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A name that is not a wire service type is refused before dispatch —
    /// no daemon, no socket, no packet — and the reason shows the shape it
    /// expects.
    #[test]
    fn a_name_that_is_not_a_service_type_is_refused_before_dispatch() {
        for bad in ["http", "_http._tcp", "printer.local.", ""] {
            let err = browse(bad, Duration::from_secs(1)).unwrap_err();
            assert!(err.contains("is not a service type"), "{bad}: {err}");
        }
        // Zero duration is a browse that cannot ask anything.
        assert!(browse("_http._tcp.local.", Duration::ZERO).is_err());
    }

    /// A real roundtrip on this host: register a service on one daemon,
    /// discover it through this crate's own daemon — two sockets, the
    /// multicast loopback between them, the same path two *processes* on a
    /// LAN would take — and check the fields the resolver filled in.
    ///
    /// The service type is unique per run so a previous run's lingering
    /// announcement (services linger seconds after their daemon dies)
    /// cannot be mistaken for this run's discovery — and its first label is
    /// kept short on purpose: on the macOS host this crate was built on,
    /// `mdns-sd` silently never resolved types whose name label ran past
    /// ~16 characters (`_floom_mdns_<pid>` never answered; `_mdns_<pid>`
    /// answered every time). Nothing in the API or the RFC makes that
    /// distinction, so it is recorded here as an observation, not a rule —
    /// the test asks the question a caller would ask, with the shortest
    /// honest name.
    #[test]
    fn a_registered_service_is_discovered_on_this_host() {
        use mdns_sd::ServiceInfo;

        let service_type = format!("_mdns_{}._tcp.local.", std::process::id());
        let instance = "roundtrip";
        let port: u16 = 42424;

        let advertiser = ServiceDaemon::new().expect("the test daemon starts");
        let info = ServiceInfo::new(
            &service_type,
            instance,
            "floom-mdns-test.local.",
            IpAddr::from([127, 0, 0, 1]),
            port,
            None,
        )
        .expect("the test service description is well-formed");
        advertiser
            .register(info)
            .expect("the test service registers");

        let found = browse(&service_type, Duration::from_secs(5)).expect("the browse runs");
        let _ = advertiser.shutdown();

        let ours = found
            .iter()
            .find(|service| service.instance == instance)
            .unwrap_or_else(|| panic!("the registered service was not discovered: {found:?}"));
        assert_eq!(ours.port, port);
        assert_eq!(ours.host, "floom-mdns-test.local.");
        assert!(
            ours.addresses.contains(&IpAddr::from([127, 0, 0, 1])),
            "the registered address was not resolved: {:?}",
            ours.addresses
        );
    }
}
