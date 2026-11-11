//! TLS setup for gateway connections.

use std::sync::Arc;

use tokio_websockets::Connector;

/// Builds the rustls connector used for `wss://` connections.
///
/// The ring crypto provider is passed explicitly (D-035/D-036) instead of relying on rustls'
/// process-level default, which panics when an application enables several providers. Trust
/// anchors come from [`root_store`].
pub(crate) fn connector() -> Connector {
    let roots = root_store();
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the ring provider supports the default protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    Connector::Rustls(tokio_rustls::TlsConnector::from(Arc::new(config)))
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
