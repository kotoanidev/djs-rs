//! TLS setup for REST connections.
//!
//! Kept in sync with `djs-gateway/src/tls.rs` (the crates do not depend on each other).

use std::sync::Arc;

/// The rustls configuration of the HTTPS connector: the ring crypto provider passed
/// explicitly (D-035/D-036) and the trust anchors of [`root_store`]. ALPN is left to the
/// connector builder, which offers HTTP/2 and HTTP/1.1.
pub(crate) fn client_config() -> rustls::ClientConfig {
    rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("the ring provider supports the default protocol versions")
        .with_root_certificates(root_store())
        .with_no_client_auth()
}

/// The trust anchors: Mozilla's root certificates bundled by `webpki-roots` (D-030) and, with
/// the `native-roots` feature, the operating system's certificates as well (D-088, D-100).
pub(crate) fn root_store() -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    #[cfg(feature = "native-roots")]
    add_native_roots(&mut roots);
    roots
}

/// Adds the system's certificates. Failures only lose those certificates: the bundled roots
/// remain, so building a client never fails because of the system store.
#[cfg(feature = "native-roots")]
fn add_native_roots(roots: &mut rustls::RootCertStore) {
    let loaded = rustls_native_certs::load_native_certs();
    for error in &loaded.errors {
        tracing::warn!(%error, "could not load system root certificates");
    }
    let (added, ignored) = roots.add_parsable_certificates(loaded.certs);
    if ignored > 0 {
        tracing::warn!(
            ignored,
            "skipped system root certificates that could not be parsed"
        );
    }
    tracing::debug!(added, "trusting system root certificates");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_roots_are_always_trusted() {
        // With `native-roots`, the system's certificates come on top of the bundled ones.
        let roots = root_store();
        assert!(roots.len() >= webpki_roots::TLS_SERVER_ROOTS.len());
    }
}
