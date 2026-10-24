use crate::{storage::Identity, Result};
use rustls::{
    client::danger::{ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    server::danger::{ClientCertVerified, ClientCertVerifier},
    DigitallySignedStruct, DistinguishedName, SignatureScheme,
};
use std::sync::Arc;
#[derive(Debug)]
struct Pin(Vec<u8>);
impl Pin {
    fn check(&self, cert: &CertificateDer<'_>) -> std::result::Result<(), rustls::Error> {
        if cert.as_ref() == self.0 {
            Ok(())
        } else {
            Err(rustls::Error::General(
                "Device identity changed; explicit re-pairing required".into(),
            ))
        }
    }
    fn tls12(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn tls13(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}
impl ServerCertVerifier for Pin {
    fn verify_server_cert(
        &self,
        end: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: &ServerName<'_>,
        _: &[u8],
        _: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        self.check(end)?;
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.tls12(m, c, d)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.tls13(m, c, d)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}
impl ClientCertVerifier for Pin {
    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &[]
    }
    fn verify_client_cert(
        &self,
        end: &CertificateDer<'_>,
        _: &[CertificateDer<'_>],
        _: UnixTime,
    ) -> std::result::Result<ClientCertVerified, rustls::Error> {
        self.check(end)?;
        Ok(ClientCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.tls12(m, c, d)
    }
    fn verify_tls13_signature(
        &self,
        m: &[u8],
        c: &CertificateDer<'_>,
        d: &DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        self.tls13(m, c, d)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}
fn material(id: &Identity) -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    (
        vec![CertificateDer::from(id.certificate.clone())],
        PrivatePkcs8KeyDer::from(id.private_key.to_vec()).into(),
    )
}
pub fn client(id: &Identity, peer: Vec<u8>) -> Result<Arc<rustls::ClientConfig>> {
    let (c, k) = material(id);
    let mut cfg = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(Pin(peer)))
    .with_client_auth_cert(c, k)?;
    cfg.alpn_protocols = vec![b"flowlink/2".to_vec()];
    Ok(Arc::new(cfg))
}
pub fn server(id: &Identity, peer: Vec<u8>) -> Result<Arc<rustls::ServerConfig>> {
    let (c, k) = material(id);
    let mut cfg = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_client_cert_verifier(Arc::new(Pin(peer)))
    .with_single_cert(c, k)?;
    cfg.alpn_protocols = vec![b"flowlink/2".to_vec()];
    Ok(Arc::new(cfg))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    fn identity() -> Identity {
        let c = rcgen::generate_simple_self_signed(vec!["flowlink.local".into()]).unwrap();
        Identity {
            certificate: c.cert.der().to_vec(),
            private_key: zeroize::Zeroizing::new(c.signing_key.serialize_der()),
            id: String::new(),
        }
    }
    #[tokio::test]
    async fn pinned_mutual_tls_exchanges_data() {
        let a = identity();
        let b = identity();
        let acceptor = tokio_rustls::TlsAcceptor::from(server(&b, a.certificate.clone()).unwrap());
        let connector =
            tokio_rustls::TlsConnector::from(client(&a, b.certificate.clone()).unwrap());
        let (left, right) = tokio::io::duplex(8192);
        let receiver = tokio::spawn(async move {
            let mut tls = acceptor.accept(left).await.unwrap();
            let mut data = [0u8; 4];
            tls.read_exact(&mut data).await.unwrap();
            assert_eq!(&data, b"ping");
            tls.write_all(b"pong").await.unwrap();
        });
        let mut tls = connector
            .connect(ServerName::try_from("flowlink.local").unwrap(), right)
            .await
            .unwrap();
        tls.write_all(b"ping").await.unwrap();
        let mut response = [0u8; 4];
        tls.read_exact(&mut response).await.unwrap();
        assert_eq!(&response, b"pong");
        receiver.await.unwrap();
    }
    #[tokio::test]
    async fn changed_server_identity_is_rejected() {
        let a = identity();
        let b = identity();
        let impostor = identity();
        let acceptor =
            tokio_rustls::TlsAcceptor::from(server(&impostor, a.certificate.clone()).unwrap());
        let connector =
            tokio_rustls::TlsConnector::from(client(&a, b.certificate.clone()).unwrap());
        let (left, right) = tokio::io::duplex(8192);
        let receiver = tokio::spawn(async move { acceptor.accept(left).await.is_err() });
        assert!(connector
            .connect(ServerName::try_from("flowlink.local").unwrap(), right)
            .await
            .is_err());
        assert!(receiver.await.unwrap());
    }
}
