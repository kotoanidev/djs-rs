//! TLS setup for gateway connections.

use std::sync::Arc;

use tokio_websockets::Connector;

/// Builds the rustls connector used for `wss://` connections.
///
/// The ring crypto provider is passed explicitly (D-035/D-036) instead of relying on rustls'
/// process-level default, which panics when an application enables several providers. Trust
/// anchors are Mozilla's root certificates bundled by `webpki-roots` (D-030).
pub(crate) fn connector() -> Connector {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("the ring provider supports the default protocol versions")
    .with_root_certificates(roots)
    .with_no_client_auth();
    Connector::Rustls(tokio_rustls::TlsConnector::from(Arc::new(config)))
}
