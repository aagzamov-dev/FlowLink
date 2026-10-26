use crate::AppState;
use serde::Serialize;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Idle,
    Checking,
    Current,
    Available,
    Downloading,
    Ready,
    Installing,
    Error,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    phase: Phase,
    current_version: String,
    version: Option<String>,
    notes: Option<String>,
    downloaded: u64,
    total: Option<u64>,
    checked_at: Option<u64>,
    error: Option<String>,
}

struct Pending {
    status: UpdateStatus,
    update: Option<Update>,
    bytes: Option<Vec<u8>>,
}

impl Pending {
    fn begin_check(&mut self) -> bool {
        if matches!(
            self.status.phase,
            Phase::Checking | Phase::Downloading | Phase::Ready | Phase::Installing
        ) {
            return false;
        }
        self.status.phase = Phase::Checking;
        self.status.error = None;
        true
    }

    fn fail(&mut self, message: String) {
        self.status.phase = if self.bytes.is_some() {
            Phase::Ready
        } else if self.update.is_some() {
            Phase::Available
        } else {
            Phase::Error
        };
        self.status.error = Some(message);
    }
}

pub struct Updates(Mutex<Pending>);

impl Updates {
    pub fn change_controls(
        &self,
        action: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let pending = self.lock();
        if pending.status.phase == Phase::Installing {
            return Err("Sharing is paused while the update is installed.".into());
        }
        action()
    }

    pub fn new(version: String) -> Self {
        Self(Mutex::new(Pending {
            status: UpdateStatus {
                phase: Phase::Idle,
                current_version: version,
                version: None,
                notes: None,
                downloaded: 0,
                total: None,
                checked_at: None,
                error: None,
            },
            update: None,
            bytes: None,
        }))
    }

    fn lock(&self) -> MutexGuard<'_, Pending> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn emit(&self, app: &AppHandle) {
        let status = self.lock().status.clone();
        let _ = app.emit("update-state", status);
    }
}

fn update_error(error: tauri_plugin_updater::Error) -> String {
    use tauri_plugin_updater::Error;
    match error {
        Error::ReleaseNotFound => "The update feed is not available yet. No release may have been published, or the service may be temporarily unavailable. Try again later.".into(),
        Error::TargetNotFound(_) | Error::TargetsNotFound(_) => "This release has no update for this computer yet. Try again later.".into(),
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) | Error::SignedVersionMismatch { .. } | Error::MissingSignedVersion => "The update could not be verified. Nothing was installed. Check for updates again or contact the developer.".into(),
        Error::Reqwest(_) | Error::Network(_) => "Could not reach the update service. Check your internet or VPN connection and try again.".into(),
        _ => format!("Update failed: {error}"),
    }
}

#[tauri::command]
pub fn update_status(updates: State<'_, Updates>) -> UpdateStatus {
    updates.lock().status.clone()
}

#[tauri::command]
pub async fn check_for_updates(app: AppHandle) {
    let updates = app.state::<Updates>();
    if !updates.lock().begin_check() {
        return;
    }
    updates.emit(&app);
    let handle = app.clone();
    let result = match app
        .updater_builder()
        .timeout(Duration::from_secs(20))
        .configure_client(|client| client.connect_timeout(Duration::from_secs(10)))
        .on_before_exit(move || {
            handle.state::<AppState>().0.shutdown();
            handle.cleanup_before_exit();
        })
        .build()
    {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    {
        let mut pending = updates.lock();
        match result {
            Ok(update) => {
                pending.status.phase = if update.is_some() {
                    Phase::Available
                } else {
                    Phase::Current
                };
                pending.status.version = update.as_ref().map(|u| u.version.clone());
                pending.status.notes = update.as_ref().and_then(|u| {
                    u.body
                        .as_ref()
                        .map(|body| body.chars().take(16_000).collect())
                });
                pending.status.checked_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs());
                pending.status.downloaded = 0;
                pending.status.total = None;
                pending.update = update;
            }
            Err(error) => pending.fail(update_error(error)),
        }
    }
    updates.emit(&app);
}

#[tauri::command]
pub async fn download_update(app: AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let mut update = {
        let mut pending = updates.lock();
        if pending.status.phase != Phase::Available {
            return Err("Check for an available update first.".into());
        }
        let update = pending.update.clone().ok_or("No update is available.")?;
        if update.download_url.scheme() != "https" {
            return Err("The update download must use HTTPS.".into());
        }
        pending.status.phase = Phase::Downloading;
        pending.status.error = None;
        pending.status.downloaded = 0;
        pending.status.total = None;
        update
    };
    updates.emit(&app);
    update.timeout = Some(Duration::from_secs(300));
    let mut last_progress = Instant::now();
    let result = update
        .download(
            |chunk, total| {
                {
                    let mut pending = updates.lock();
                    pending.status.downloaded =
                        pending.status.downloaded.saturating_add(chunk as u64);
                    pending.status.total = total.filter(|n| *n > 0);
                }
                if last_progress.elapsed() >= Duration::from_millis(200) {
                    updates.emit(&app);
                    last_progress = Instant::now();
                }
            },
            || {},
        )
        .await;
    {
        let mut pending = updates.lock();
        match result {
            Ok(bytes) => {
                pending.bytes = Some(bytes);
                pending.status.phase = Phase::Ready;
            }
            Err(error) => pending.fail(update_error(error)),
        }
    }
    updates.emit(&app);
    Ok(())
}

#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let updates = app.state::<Updates>();
    let (update, bytes) = {
        let mut pending = updates.lock();
        if pending.status.phase != Phase::Ready {
            return Err("Download and verify the update first.".into());
        }
        let update = pending.update.clone().ok_or("No update is available.")?;
        let bytes = pending.bytes.take().ok_or("Download the update again.")?;
        pending.status.phase = Phase::Installing;
        pending.status.error = None;
        (update, bytes)
    };
    updates.emit(&app);
    let handle = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let engine = &handle.state::<AppState>().0;
        let result = engine
            .pause(true)
            .map_err(|e| e.to_string())
            .and_then(|()| {
                engine.disconnect();
                update.install(&bytes).map_err(update_error)
            });
        (result, bytes)
    })
    .await;
    match result {
        Ok((Ok(()), _)) => {
            app.state::<AppState>().0.shutdown();
            app.request_restart();
        }
        Ok((Err(error), bytes)) => {
            let mut pending = updates.lock();
            pending.bytes = Some(bytes);
            pending.fail(error);
        }
        Err(_) => updates.lock().fail(
            "Installation was interrupted. Download the update again. Sharing remains paused."
                .into(),
        ),
    }
    updates.emit(&app);
    Ok(())
}

pub fn start_automatic_checks(app: AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(30)).await;
        loop {
            if app
                .state::<AppState>()
                .0
                .snapshot()
                .settings
                .auto_check_updates
            {
                check_for_updates(app.clone()).await;
            }
            tokio::time::sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_settings_load_without_an_update_preference() {
        let mut saved = serde_json::to_value(flowlink_engine::model::Settings::default()).unwrap();
        saved.as_object_mut().unwrap().remove("autoCheckUpdates");
        let settings: flowlink_engine::model::Settings = serde_json::from_value(saved).unwrap();
        assert!(settings.auto_check_updates);
    }

    fn fixture_updater() -> tauri_plugin_updater::Updater {
        let mut context = tauri::test::mock_context(tauri::test::noop_assets());
        context.package_info_mut().version = "0.2.0".parse().unwrap();
        context.config_mut().plugins.0.insert(
            "updater".into(),
            serde_json::json!({"pubkey": include_str!("../tests/fixtures/update.pub").trim()}),
        );
        let app = tauri::test::mock_builder()
            .plugin(tauri_plugin_updater::Builder::new().build())
            .build(context)
            .unwrap();
        app.updater_builder()
            .pubkey(include_str!("../tests/fixtures/update.pub").trim())
            .endpoints(vec!["https://example.com/latest.json".parse().unwrap()])
            .unwrap()
            .target("test")
            .build()
            .unwrap()
    }

    fn fixture_manifest(version: &str) -> serde_json::Value {
        serde_json::json!({"version": version, "platforms": {"test": {
            "url": "https://example.com/update.tar.gz",
            "signature": include_str!("../tests/fixtures/update.txt.sig").trim()
        }}})
    }

    #[test]
    fn official_verifier_rejects_tampering_and_relabelled_versions() {
        let updater = fixture_updater();
        let update = updater
            .restore_update(fixture_manifest("0.2.1"))
            .unwrap()
            .unwrap();
        let payload = include_bytes!("../tests/fixtures/update.txt");
        assert!(!payload.contains(&b'\r'));
        update.verify(payload).unwrap();
        let crlf_payload = String::from_utf8_lossy(payload).replace("\n", "\r\n");
        assert!(update.verify(crlf_payload.as_bytes()).is_err());
        assert!(update.verify(b"modified package").is_err());
        let relabelled = updater
            .restore_update(fixture_manifest("0.2.2"))
            .unwrap()
            .unwrap();
        assert!(matches!(
            relabelled.verify(payload),
            Err(tauri_plugin_updater::Error::SignedVersionMismatch { .. })
        ));
        assert!(updater
            .restore_update(fixture_manifest("0.0.0"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn install_blocks_local_resume_until_failure_is_recovered() {
        let updates = Updates::new("0.2.0".into());
        updates.lock().status.phase = Phase::Installing;
        let called = std::cell::Cell::new(false);
        assert!(updates
            .change_controls(|| {
                called.set(true);
                Ok(())
            })
            .is_err());
        assert!(!called.get());
        updates.lock().fail("failed install".into());
        assert!(updates
            .change_controls(|| {
                called.set(true);
                Ok(())
            })
            .is_ok());
        assert!(called.get());
    }

    #[test]
    fn checks_cannot_discard_downloads_or_overlap_operations() {
        let updates = Updates::new("0.2.0".into());
        let mut pending = updates.lock();
        assert!(pending.begin_check());
        for phase in [
            Phase::Checking,
            Phase::Downloading,
            Phase::Ready,
            Phase::Installing,
        ] {
            pending.status.phase = phase;
            assert!(!pending.begin_check());
        }
        pending.status.phase = Phase::Current;
        assert!(pending.begin_check());
    }

    #[test]
    fn failed_install_preserves_verified_download_for_retry() {
        let updates = Updates::new("0.2.0".into());
        let mut pending = updates.lock();
        pending.bytes = Some(vec![1, 2, 3]);
        pending.status.phase = Phase::Installing;
        pending.fail("disk full".into());
        assert_eq!(pending.status.phase, Phase::Ready);
        assert_eq!(pending.bytes.as_deref(), Some([1, 2, 3].as_slice()));
        assert_eq!(pending.status.error.as_deref(), Some("disk full"));
    }

    #[test]
    fn unavailable_feed_and_bad_signatures_are_not_reported_as_up_to_date() {
        let updates = Updates::new("0.2.0".into());
        let mut pending = updates.lock();
        pending.begin_check();
        pending.fail(update_error(tauri_plugin_updater::Error::ReleaseNotFound));
        assert_eq!(pending.status.phase, Phase::Error);
        assert!(pending.status.checked_at.is_none());
        assert!(
            update_error(tauri_plugin_updater::Error::MissingSignedVersion)
                .contains("Nothing was installed")
        );
    }
}
