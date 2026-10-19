use crate::{
    input::{Input, NativeEvent},
    model::{Display, Permissions, Settings},
    Result,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
    time::{Duration, Instant},
};
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

pub struct Gate {
    pub remote: AtomicBool,
    pub block_local: AtomicBool,
    pub healthy: AtomicBool,
    pub deadline: AtomicU64,
    pub emergency_key: AtomicU16,
    pub sender: SyncSender<NativeEvent>,
}
impl Gate {
    pub fn push(&self, event: NativeEvent) {
        if self.sender.try_send(event).is_err() {
            self.remote.store(false, Ordering::SeqCst);
            self.block_local.store(false, Ordering::SeqCst);
            self.healthy.store(false, Ordering::SeqCst);
        }
    }
    pub fn stop(&self) {
        self.remote.store(false, Ordering::SeqCst);
        self.block_local.store(false, Ordering::SeqCst);
    }
    pub fn active(&self) -> bool {
        self.remote.load(Ordering::SeqCst) && clock() < self.deadline.load(Ordering::SeqCst)
    }
}
pub fn clock() -> u64 {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u64
}
pub struct Native {
    pub gate: Arc<Gate>,
    #[cfg(target_os = "macos")]
    inner: macos::Mac,
    #[cfg(target_os = "windows")]
    inner: windows::Windows,
}
impl Native {
    pub fn start() -> Result<(Arc<Self>, Receiver<NativeEvent>)> {
        let (tx, rx) = sync_channel(2048);
        let gate = Arc::new(Gate {
            remote: AtomicBool::new(false),
            block_local: AtomicBool::new(false),
            healthy: AtomicBool::new(false),
            deadline: AtomicU64::new(0),
            emergency_key: AtomicU16::new(1),
            sender: tx,
        });
        #[cfg(target_os = "macos")]
        let inner = macos::Mac::start(gate.clone())?;
        #[cfg(target_os = "windows")]
        let inner = windows::Windows::start(gate.clone())?;
        let native = Arc::new(Self { gate, inner });
        let weak = Arc::downgrade(&native);
        std::thread::Builder::new()
            .name("flowlink-safety".into())
            .spawn(move || {
                let mut last_wall = std::time::SystemTime::now();
                while let Some(n) = weak.upgrade() {
                    let now = std::time::SystemTime::now();
                    let interrupted = now
                        .duration_since(last_wall)
                        .map(|d| d > Duration::from_secs(1))
                        .unwrap_or(true);
                    last_wall = now;
                    if n.gate.remote.load(Ordering::SeqCst)
                        && (!n.gate.active()
                            || !n.gate.healthy.load(Ordering::SeqCst)
                            || interrupted
                            || !n.inner.session_available())
                    {
                        n.release();
                        n.gate.push(NativeEvent::Failure(
                            "Input ownership expired; control restored locally".into(),
                        ));
                    }
                    drop(n);
                    std::thread::sleep(Duration::from_millis(25));
                }
            })?;
        Ok((native, rx))
    }
    pub fn lease(&self, block_local: bool) -> Result<()> {
        let p = permissions();
        if !p.accessibility
            || !p.monitoring
            || (block_local && !p.filtering)
            || !self.gate.healthy.load(Ordering::SeqCst)
            || !self.inner.session_available()
        {
            return Err("Native input permissions or input hook unavailable".into());
        }
        self.gate.deadline.store(clock() + 1000, Ordering::SeqCst);
        self.gate.block_local.store(block_local, Ordering::SeqCst);
        self.gate.remote.store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn renew(&self) {
        self.gate.deadline.store(clock() + 1000, Ordering::SeqCst);
    }
    pub fn release(&self) {
        self.gate.stop();
        self.inner.release();
    }
    pub fn warp(&self, x: f64, y: f64) {
        self.inner.warp(x, y);
    }
    pub fn inject(&self, event: &Input, settings: &Settings) -> Result<()> {
        if !self.gate.active() {
            return Err("Remote input lease expired".into());
        }
        self.inner.inject(event, settings)
    }
    pub fn cursor(&self) -> (f64, f64) {
        self.inner.cursor()
    }
}
pub fn displays(device_id: &str) -> Result<Vec<Display>> {
    #[cfg(target_os = "macos")]
    return macos::displays(device_id);
    #[cfg(target_os = "windows")]
    return windows::displays(device_id);
}
pub fn permissions() -> Permissions {
    #[cfg(target_os = "macos")]
    return macos::permissions();
    #[cfg(target_os = "windows")]
    return Permissions {
        accessibility: true,
        monitoring: true,
        filtering: true,
    };
}
pub fn request_permissions() {
    #[cfg(target_os = "macos")]
    macos::request_permissions();
}
pub fn foreground_app() -> Option<String> {
    #[cfg(target_os = "macos")]
    return macos::foreground_app();
    #[cfg(target_os = "windows")]
    return None;
}
pub fn copy_text(text: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::copy_text(text);
    #[cfg(target_os = "windows")]
    return windows::copy_text(text);
}

pub fn interface_kind(name: &str, _index: Option<u32>) -> String {
    #[cfg(target_os = "macos")]
    return macos::interface_kind(name);
    #[cfg(target_os = "windows")]
    return windows::interface_kind(name, _index);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queue_overflow_immediately_stops_capture_and_filtering() {
        let (tx, _rx) = sync_channel(1);
        let gate = Gate {
            remote: AtomicBool::new(true),
            block_local: AtomicBool::new(true),
            healthy: AtomicBool::new(true),
            deadline: AtomicU64::new(clock() + 1000),
            emergency_key: AtomicU16::new(1),
            sender: tx,
        };
        gate.push(NativeEvent::Position(0.0, 0.0));
        gate.push(NativeEvent::Position(1.0, 1.0));
        assert!(!gate.remote.load(Ordering::SeqCst));
        assert!(!gate.block_local.load(Ordering::SeqCst));
        assert!(!gate.healthy.load(Ordering::SeqCst));
    }
    #[test]
    fn expired_gate_does_not_suppress_input_even_before_watchdog_runs() {
        let (tx, _rx) = sync_channel(1);
        let gate = Gate {
            remote: AtomicBool::new(true),
            block_local: AtomicBool::new(true),
            healthy: AtomicBool::new(true),
            deadline: AtomicU64::new(0),
            emergency_key: AtomicU16::new(1),
            sender: tx,
        };
        assert!(!gate.active());
    }
}
