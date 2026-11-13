//! The HTTPS mock serenity is pointed at (D-282), and the client that reaches it.
//!
//! serenity 0.12.5's default rate limiter ignores `HttpBuilder::proxy`, so serenity keeps its
//! `https://discord.com` URLs and gets a `reqwest::Client` that resolves `discord.com` to this
//! mock and trusts only a certificate authority generated for the run. The mock answers with
//! the response code of the plain `vs-node` mock ([`djs_vs_node::mock_rest::respond`]), so
//! both mocks send the same bytes.

use std::{
    convert::Infallible,
    fs,
    net::SocketAddr,
    path::Path,
    sync::{Arc, atomic::AtomicU64},
};

use djs_vs_node::{Options, mock_rest::respond};
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use rcgen::{BasicConstraints, CertificateParams, DnType, IsCa, Issuer, KeyPair, KeyUsagePurpose};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use serde_json::json;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

/// The host serenity's URLs name.
const DISCORD: &str = "discord.com";

/// A certificate authority made for one run, and a `discord.com` certificate it signed.
struct Certificates {
    ca_pem: String,
    chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
}

fn certificates() -> Certificates {
    let ca_key = KeyPair::generate().expect("a key pair");
    let mut ca = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
        KeyUsagePurpose::DigitalSignature,
    ];
    ca.distinguished_name
        .push(DnType::CommonName, "djs-rs benchmark CA (test only)");
    let ca_certificate = ca.self_signed(&ca_key).expect("the CA signs itself");
    let issuer = Issuer::new(ca, ca_key);

    let key = KeyPair::generate().expect("a key pair");
    let mut leaf = CertificateParams::new(vec![DISCORD.to_owned()]).expect("leaf parameters");
    leaf.distinguished_name.push(DnType::CommonName, DISCORD);
    let certificate = leaf
        .signed_by(&key, &issuer)
        .expect("the CA signs the leaf");
    Certificates {
        ca_pem: ca_certificate.pem(),
        chain: vec![certificate.der().clone()],
        key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key.serialize_der())),
    }
}

/// `tls-api --dir D [--gateway URL]`: writes the run's CA to `D/ca.pem`, serves the mock API
/// over HTTPS (HTTP/1.1, keep-alive) on a free port of 127.0.0.1 and prints
/// `{"tls_api": "127.0.0.1:<port>", "ca": "<path>"}`. `--gateway` is the URL `GET /gateway`
/// answers (the gateway of a running `vs-node mock-gateway`).
pub(crate) async fn serve(options: &Options) {
    let gateway: Option<Arc<str>> = Some(options.get("gateway", String::new()))
        .filter(|url| !url.is_empty())
        .map(Into::into);
    let dir = Path::new(options.required("dir"));
    fs::create_dir_all(dir).expect("the directory can be created");
    let Certificates { ca_pem, chain, key } = certificates();
    let ca_path = dir.join("ca.pem");
    fs::write(&ca_path, ca_pem).expect("the CA is written");

    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("TLS 1.2 and 1.3")
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .expect("the certificate and its key match");
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a free port");
    let address = listener.local_addr().expect("a bound address");
    println!(
        "{}",
        json!({ "tls_api": address.to_string(), "ca": ca_path.display().to_string() })
    );
    let sequence = Arc::new(AtomicU64::new(0));
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let _ = stream.set_nodelay(true);
        let acceptor = acceptor.clone();
        let gateway = gateway.clone();
        let sequence = Arc::clone(&sequence);
        tokio::spawn(async move {
            let Ok(stream) = acceptor.accept(stream).await else {
                return;
            };
            let service = service_fn(move |request| {
                let gateway = gateway.clone();
                let sequence = Arc::clone(&sequence);
                async move { Ok::<_, Infallible>(respond(request, gateway.as_deref(), &sequence).await) }
            });
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

/// serenity's default client (`Client::builder().use_rustls_tls()`), except that it sends
/// `discord.com` to the HTTPS mock at `--tls` and trusts only the CA at `--ca`: without the
/// built-in roots, the real Discord's certificate would be refused.
pub(crate) fn client(options: &Options) -> reqwest::Client {
    let address: SocketAddr = options
        .required("tls")
        .parse()
        .expect("--tls is an address");
    assert!(
        address.ip().is_loopback(),
        "only a local mock API is allowed, got {address}"
    );
    let ca = fs::read(options.required("ca")).expect("--ca is readable");
    reqwest::Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(false)
        .add_root_certificate(reqwest::Certificate::from_pem(&ca).expect("a PEM certificate"))
        .resolve(DISCORD, address)
        .build()
        .expect("the client is built")
}
