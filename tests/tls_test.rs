//! TLS tests use generated certificates, dummy credentials and ephemeral loopback only.

use pbs_exporter::{
    client::PbsClient,
    config::{PbsConfig, Settings},
};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use std::{
    io::Write,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tempfile::NamedTempFile;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    task::JoinHandle,
};
use tokio_rustls::{
    rustls::{self, pki_types::PrivatePkcs8KeyDer},
    TlsAcceptor,
};

struct Fixture {
    url: String,
    pem: String,
    requests: Arc<AtomicUsize>,
    task: JoinHandle<()>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

async fn fixture() -> Fixture {
    fixture_with_redirect(None).await
}

async fn fixture_with_redirect(redirect: Option<String>) -> Fixture {
    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let pem = cert.pem();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_no_client_auth()
    .with_single_cert(
        vec![cert.der().clone()],
        PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
    )
    .unwrap();
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!(
        "https://localhost:{}",
        listener.local_addr().unwrap().port()
    );
    let requests = Arc::new(AtomicUsize::new(0));
    let count = requests.clone();
    let task = tokio::spawn(async move {
        while let Ok((socket, _)) = listener.accept().await {
            let acceptor = acceptor.clone();
            let count = count.clone();
            let redirect = redirect.clone();
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(socket).await else {
                    return;
                };
                let mut request = vec![0; 8192];
                let mut used = 0;
                loop {
                    let read = stream.read(&mut request[used..]).await.unwrap_or(0);
                    if read == 0 {
                        return;
                    }
                    used += read;
                    if request[..used].windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                    if used == request.len() {
                        return;
                    }
                }
                let request = String::from_utf8_lossy(&request[..used]);
                assert!(request
                    .to_lowercase()
                    .contains("authorization: pbsapitoken=dummy@pam!tls:dummy-secret"));
                count.fetch_add(1, Ordering::SeqCst);
                let body = r#"{"data":{"version":"4.1.0","release":"test","repoid":"test"}}"#;
                let response = if let Some(location) = redirect {
                    format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    )
                };
                stream.write_all(response.as_bytes()).await.unwrap();
            });
        }
    });
    Fixture {
        url,
        pem,
        requests,
        task,
    }
}

fn config(endpoint: &str) -> PbsConfig {
    let mut config = Settings::default().pbs;
    config.endpoint = endpoint.into();
    config.token_id = "dummy@pam!tls".into();
    config.token_secret = "dummy-secret".into();
    config
}

fn ca_file(pem: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(pem.as_bytes()).unwrap();
    file
}

#[tokio::test]
async fn default_tls_rejects_untrusted_peer_before_sending_token() {
    let server = fixture().await;
    let client = PbsClient::new(config(&server.url)).unwrap();
    assert!(client.get_version().await.is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn configured_ca_accepts_private_certificate() {
    let server = fixture().await;
    let file = ca_file(&server.pem);
    let mut config = config(&server.url);
    config.ca_cert_path = Some(file.path().to_str().unwrap().into());
    let client = PbsClient::new(config).unwrap();
    assert_eq!(client.get_version().await.unwrap().version, "4.1.0");
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn explicit_insecure_opt_out_remains_available() {
    let server = fixture().await;
    let mut config = config(&server.url);
    config.verify_tls = false;
    let client = PbsClient::new(config).unwrap();
    assert!(client.get_version().await.is_ok());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
}

#[test]
fn verified_transport_rejects_plain_http() {
    assert!(PbsClient::new(config("http://127.0.0.1:8007")).is_err());
}

#[test]
fn empty_or_missing_ca_fails_closed() {
    let file = ca_file("");
    let mut config = config("https://localhost:8007");
    config.ca_cert_path = Some(file.path().to_str().unwrap().into());
    assert!(PbsClient::new(config.clone()).is_err());
    config.ca_cert_path = Some(
        file.path()
            .with_extension("missing")
            .to_str()
            .unwrap()
            .into(),
    );
    assert!(PbsClient::new(config).is_err());
}

#[test]
fn custom_ca_cannot_be_silently_ignored_by_insecure_mode() {
    let mut config = config("https://localhost:8007");
    config.verify_tls = false;
    config.ca_cert_path = Some("unused.pem".into());
    assert!(PbsClient::new(config).is_err());
}

#[tokio::test]
async fn configured_ca_does_not_disable_hostname_checks() {
    let server = fixture().await;
    let file = ca_file(&server.pem);
    let mut config = config(&server.url.replace("localhost", "127.0.0.1"));
    config.ca_cert_path = Some(file.path().to_str().unwrap().into());
    let client = PbsClient::new(config).unwrap();
    assert!(client.get_version().await.is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unrelated_ca_does_not_authenticate_the_peer() {
    let server = fixture().await;
    let unrelated = generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let file = ca_file(&unrelated.cert.pem());
    let mut config = config(&server.url);
    config.ca_cert_path = Some(file.path().to_str().unwrap().into());
    let client = PbsClient::new(config).unwrap();
    assert!(client.get_version().await.is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn verified_transport_rejects_redirect_to_plain_http() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let target = format!(
        "http://localhost:{}/version",
        listener.local_addr().unwrap().port()
    );
    let server = fixture_with_redirect(Some(target)).await;
    let file = ca_file(&server.pem);
    let mut config = config(&server.url);
    config.ca_cert_path = Some(file.path().to_str().unwrap().into());
    let client = PbsClient::new(config).unwrap();
    assert!(client.get_version().await.is_err());
    assert_eq!(server.requests.load(Ordering::SeqCst), 1);
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), listener.accept())
            .await
            .is_err()
    );
}
