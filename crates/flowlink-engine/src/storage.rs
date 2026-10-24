use crate::{
    model::{Settings, TrustedDevice},
    Result,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct Identity {
    pub certificate: Vec<u8>,
    pub private_key: Zeroizing<Vec<u8>>,
    pub id: String,
}
#[derive(Serialize, Deserialize)]
struct SecretIdentity {
    certificate: String,
    private_key: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub settings: Settings,
    pub trusted: Vec<TrustedDevice>,
}
pub struct Store {
    path: PathBuf,
    pub config: Mutex<Config>,
    pub identity: Identity,
}
impl Store {
    pub fn open(directory: &Path) -> Result<Self> {
        fs::create_dir_all(directory)?;
        let path = directory.join("settings.json");
        let config: Config = if path.exists() {
            let bytes = fs::read(&path)?;
            if bytes.len() > 1024 * 1024 {
                return Err("Settings file exceeds size limit".into());
            }
            serde_json::from_slice(&bytes)?
        } else {
            Config::default()
        };
        config.settings.validate()?;
        let entry = keyring::Entry::new("app.flowlink.identity", "device-v1")?;
        let identity = match entry.get_password() {
            Ok(secret) => {
                let secret = Zeroizing::new(secret);
                let saved: SecretIdentity = serde_json::from_str(&secret)?;
                let cert = hex::decode(saved.certificate)?;
                let key = Zeroizing::new(hex::decode(saved.private_key)?);
                Identity {
                    id: hex::encode(Sha256::digest(&cert)),
                    certificate: cert,
                    private_key: key,
                }
            }
            Err(keyring::Error::NoEntry) => {
                let generated = rcgen::generate_simple_self_signed(vec!["flowlink.local".into()])?;
                let cert = generated.cert.der().to_vec();
                let key = Zeroizing::new(generated.signing_key.serialize_der());
                let secret = Zeroizing::new(serde_json::to_string(&SecretIdentity {
                    certificate: hex::encode(&cert),
                    private_key: hex::encode(&key),
                })?);
                entry.set_password(&secret)?;
                Identity {
                    id: hex::encode(Sha256::digest(&cert)),
                    certificate: cert,
                    private_key: key,
                }
            }
            Err(error) => {
                return Err(format!("Cannot access protected device identity: {error}").into())
            }
        };
        let store = Self {
            path,
            config: Mutex::new(config),
            identity,
        };
        store.save()?;
        Ok(store)
    }
    pub fn save(&self) -> Result<()> {
        let config = self.config.lock().map_err(|_| "Settings lock failed")?;
        let data = serde_json::to_vec_pretty(&*config)?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(temporary, &self.path)?;
        Ok(())
    }
    pub fn settings(&self) -> Settings {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .settings
            .clone()
    }
    pub fn trusted(&self) -> Vec<TrustedDevice> {
        self.config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .trusted
            .clone()
    }
    pub fn trust(&self, device: TrustedDevice) -> Result<()> {
        let mut c = self.config.lock().map_err(|_| "Settings lock failed")?;
        c.trusted.retain(|d| d.device.id != device.device.id);
        c.trusted.push(device);
        drop(c);
        self.save()
    }
}

#[cfg(test)]
impl Store {
    pub(crate) fn temporary() -> Self {
        let directory =
            std::env::temp_dir().join(format!("flowlink-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        Self {
            path: directory.join("settings.json"),
            config: Mutex::new(Config::default()),
            identity: Identity {
                certificate: Vec::new(),
                private_key: Zeroizing::new(Vec::new()),
                id: "local".into(),
            },
        }
    }
}
