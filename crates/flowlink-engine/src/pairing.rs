use crate::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use std::time::{Duration, Instant};
use zeroize::Zeroizing;
type HmacSha256 = Hmac<Sha256>;
pub struct Exchange(Spake2<Ed25519Group>);
impl Exchange {
    pub fn start(code: &str, controller: bool) -> Result<(Self, Vec<u8>)> {
        if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Enter the six-digit code shown on the Mac".into());
        }
        let a = Identity::new(b"flowlink/1/windows");
        let b = Identity::new(b"flowlink/1/mac");
        let (state, public) = if controller {
            Spake2::<Ed25519Group>::start_a(&Password::new(code.as_bytes()), &a, &b)
        } else {
            Spake2::<Ed25519Group>::start_b(&Password::new(code.as_bytes()), &a, &b)
        };
        Ok((Self(state), public))
    }
    pub fn finish(self, peer: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
        self.0
            .finish(peer)
            .map(Zeroizing::new)
            .map_err(|_| "Pairing exchange failed".into())
    }
}
pub fn transcript(a: &[u8], b: &[u8], client_cert: &[u8], server_cert: &[u8]) -> Vec<u8> {
    let mut out = b"FlowLink pairing v1".to_vec();
    for part in [a, b, client_cert, server_cert] {
        out.extend_from_slice(&(part.len() as u32).to_be_bytes());
        out.extend_from_slice(part);
    }
    out
}
pub fn proof(key: &[u8], transcript: &[u8], role: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key length");
    mac.update(role);
    mac.update(transcript);
    mac.finalize().into_bytes().to_vec()
}
pub fn verify(key: &[u8], transcript: &[u8], role: &[u8], value: &[u8]) -> Result<()> {
    let mut mac = HmacSha256::new_from_slice(key)?;
    mac.update(role);
    mac.update(transcript);
    mac.verify_slice(value)
        .map_err(|_| "Pairing code did not match".into())
}
pub struct PairingLimiter {
    failures: u8,
    window: Instant,
}
impl Default for PairingLimiter {
    fn default() -> Self {
        Self {
            failures: 0,
            window: Instant::now(),
        }
    }
}
impl PairingLimiter {
    pub fn allowed(&mut self) -> bool {
        if self.window.elapsed() > Duration::from_secs(120) {
            self.failures = 0;
            self.window = Instant::now();
        }
        self.failures < 3
    }
    pub fn failed(&mut self) {
        self.failures = self.failures.saturating_add(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrong_code_cannot_confirm() {
        let (a, ap) = Exchange::start("123456", true).unwrap();
        let (b, bp) = Exchange::start("654321", false).unwrap();
        let ak = a.finish(&bp).unwrap();
        let bk = b.finish(&ap).unwrap();
        let t = transcript(&ap, &bp, b"a", b"b");
        assert!(verify(&bk, &t, b"client", &proof(&ak, &t, b"client")).is_err());
    }
    #[test]
    fn identities_are_bound_to_proof() {
        let (a, ap) = Exchange::start("123456", true).unwrap();
        let (b, bp) = Exchange::start("123456", false).unwrap();
        let ak = a.finish(&bp).unwrap();
        let bk = b.finish(&ap).unwrap();
        let t = transcript(&ap, &bp, b"a", b"b");
        let p = proof(&ak, &t, b"client");
        assert!(verify(&bk, &t, b"client", &p).is_ok());
        assert!(verify(&bk, &transcript(&ap, &bp, b"evil", b"b"), b"client", &p).is_err());
        assert!(verify(&bk, &t, b"server", &p).is_err());
    }
    #[test]
    fn rate_limits_apply_globally() {
        let mut l = PairingLimiter::default();
        for _ in 0..3 {
            assert!(l.allowed());
            l.failed();
        }
        assert!(!l.allowed());
    }
}
