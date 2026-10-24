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
#[cfg_attr(test, allow(dead_code))]
mod macos;
#[cfg(target_os = "windows")]
#[cfg_attr(test, allow(dead_code))]
mod windows;

pub struct Gate {
    pub remote: AtomicBool,
    pub capture: AtomicBool,
    pub generation: AtomicU64,
    pub fault: AtomicBool,
    pub block_local: AtomicBool,
    pub healthy: AtomicBool,
    pub deadline: AtomicU64,
    pub emergency_key: AtomicU16,
    pub sender: SyncSender<NativeEvent>,
}
impl Gate {
    pub fn push(&self, event: NativeEvent) {
        if self.sender.try_send(event).is_err() {
            self.stop();
            self.fault.store(true, Ordering::SeqCst);
        }
    }
    pub fn capturing(&self) -> bool {
        self.active() && self.capture.load(Ordering::SeqCst)
    }
    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.capture.store(false, Ordering::SeqCst);
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
    input_operation: std::sync::Mutex<()>,
    #[cfg(test)]
    inner: TestNative,
    pub gate: Arc<Gate>,
    #[cfg(all(target_os = "macos", not(test)))]
    inner: macos::Mac,
    #[cfg(all(target_os = "windows", not(test)))]
    inner: windows::Windows,
}
impl Native {
    pub fn start() -> Result<(Arc<Self>, Receiver<NativeEvent>)> {
        let (tx, rx) = sync_channel(2048);
        let gate = Arc::new(Gate {
            capture: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            fault: AtomicBool::new(false),
            remote: AtomicBool::new(false),
            block_local: AtomicBool::new(false),
            healthy: AtomicBool::new(false),
            deadline: AtomicU64::new(0),
            emergency_key: AtomicU16::new(1),
            sender: tx,
        });
        #[cfg(all(target_os = "macos", not(test)))]
        let inner = macos::Mac::start(gate.clone())?;
        #[cfg(all(target_os = "windows", not(test)))]
        let inner = windows::Windows::start(gate.clone())?;
        #[cfg(test)]
        let inner = TestNative::default();
        let native = Arc::new(Self {
            gate,
            inner,
            input_operation: std::sync::Mutex::new(()),
        });
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
                        n.gate.fault.store(true, Ordering::SeqCst);
                    }
                    drop(n);
                    std::thread::sleep(Duration::from_millis(25));
                }
            })?;
        Ok((native, rx))
    }
    pub fn lease(&self, block_local: bool, capture: bool) -> Result<()> {
        let _operation = self
            .input_operation
            .lock()
            .map_err(|_| "Input unavailable")?;
        if self.inner.has_local_input() {
            return Err("Release held keys and buttons on both devices before switching".into());
        }
        let p = permissions();
        if !p.accessibility
            || !p.monitoring
            || (block_local && !p.filtering)
            || !self.gate.healthy.load(Ordering::SeqCst)
            || !self.inner.session_available()
            || self.gate.fault.load(Ordering::SeqCst)
        {
            return Err("Native input permissions or input hook unavailable".into());
        }
        self.gate.generation.fetch_add(1, Ordering::SeqCst);
        self.gate.capture.store(capture, Ordering::SeqCst);
        self.gate.deadline.store(clock() + 1000, Ordering::SeqCst);
        self.gate.block_local.store(block_local, Ordering::SeqCst);
        self.gate.remote.store(true, Ordering::SeqCst);
        Ok(())
    }
    pub fn renew(&self) {
        if self.gate.active() {
            self.gate.deadline.store(clock() + 1000, Ordering::SeqCst);
        }
    }
    pub fn has_local_input(&self) -> bool {
        self.inner.has_local_input()
    }
    pub fn release(&self) {
        self.gate.stop();
        let _operation = self
            .input_operation
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.gate.stop();
        self.inner.release();
    }
    pub fn warp(&self, x: f64, y: f64) {
        self.inner.warp(x, y);
    }
    pub fn inject(&self, event: &Input, settings: &Settings) -> Result<()> {
        let _operation = self
            .input_operation
            .lock()
            .map_err(|_| "Input unavailable")?;
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
#[cfg(not(test))]
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
            capture: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            fault: AtomicBool::new(false),
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
        assert!(gate.healthy.load(Ordering::SeqCst));
        assert!(gate.fault.load(Ordering::SeqCst));
    }
    #[test]
    fn expired_gate_does_not_suppress_input_even_before_watchdog_runs() {
        let (tx, _rx) = sync_channel(1);
        let gate = Gate {
            capture: AtomicBool::new(false),
            generation: AtomicU64::new(0),
            fault: AtomicBool::new(false),
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

#[cfg(test)]
pub fn permissions() -> Permissions {
    Permissions {
        accessibility: true,
        monitoring: true,
        filtering: true,
    }
}
#[cfg(test)]
#[derive(Default)]
struct TestNative {
    physical_held: AtomicBool,
    cursor: std::sync::Mutex<(f64, f64)>,
}
#[cfg(test)]
impl TestNative {
    fn session_available(&self) -> bool {
        true
    }
    fn has_local_input(&self) -> bool {
        self.physical_held.load(Ordering::SeqCst)
    }
    fn release(&self) {}
    fn warp(&self, x: f64, y: f64) {
        *self.cursor.lock().unwrap() = (x, y);
    }
    fn cursor(&self) -> (f64, f64) {
        *self.cursor.lock().unwrap()
    }
    fn inject(&self, _: &Input, _: &Settings) -> Result<()> {
        Ok(())
    }
}
#[cfg(test)]
impl Native {
    pub(crate) fn set_local_input_held(&self, held: bool) {
        self.inner.physical_held.store(held, Ordering::SeqCst);
    }
    pub(crate) fn for_test() -> Arc<Self> {
        let (sender, _rx) = sync_channel(32);
        Arc::new(Self {
            input_operation: std::sync::Mutex::new(()),
            gate: Arc::new(Gate {
                generation: AtomicU64::new(0),
                remote: AtomicBool::new(false),
                capture: AtomicBool::new(false),
                fault: AtomicBool::new(false),
                healthy: AtomicBool::new(true),
                block_local: AtomicBool::new(false),
                deadline: AtomicU64::new(0),
                emergency_key: AtomicU16::new(1),
                sender,
            }),
            inner: TestNative::default(),
        })
    }
}
