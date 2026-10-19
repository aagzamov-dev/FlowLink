use crate::model::{Ownership, Preset, Settings};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Input {
    Motion { dx: f64, dy: f64 },
    Button { button: u8, down: bool },
    Scroll { x: f64, y: f64 },
    Key { scan: u16, down: bool },
}
impl Input {
    pub fn valid(&self) -> bool {
        match self {
            Self::Motion { dx, dy } => {
                dx.is_finite() && dy.is_finite() && dx.abs() < 10000.0 && dy.abs() < 10000.0
            }
            Self::Scroll { x, y } => {
                x.is_finite() && y.is_finite() && x.abs() < 10000.0 && y.abs() < 10000.0
            }
            Self::Button { button, .. } => *button < 5,
            Self::Key { scan, .. } => *scan <= 0x1ff,
        }
    }
}
#[derive(Debug, Clone)]
pub enum NativeEvent {
    Input(Input),
    Position(f64, f64),
    Emergency,
    Switch,
    Failure(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectedKey {
    pub scan: u16,
    pub command: bool,
    pub down: bool,
}

#[derive(Default)]
pub struct KeyMapper {
    held: HashMap<u16, InjectedKey>,
    ctrl: HashSet<u16>,
    pub local_held: HashSet<u16>,
}
impl KeyMapper {
    pub fn map(
        &mut self,
        scan: u16,
        down: bool,
        settings: &Settings,
        terminal: bool,
    ) -> InjectedKey {
        if !down {
            self.ctrl.remove(&scan);
            return self
                .held
                .remove(&scan)
                .map(|k| InjectedKey { down: false, ..k })
                .unwrap_or(InjectedKey {
                    scan,
                    command: false,
                    down: false,
                });
        }
        if let Some(k) = self.held.get(&scan) {
            return k.clone();
        }
        if scan == 0x1d || scan == 0x11d {
            self.ctrl.insert(scan);
        }
        let command = settings.preset == Preset::WindowsFamiliar
            && !terminal
            && !self.ctrl.is_empty()
            && settings
                .shortcuts
                .iter()
                .any(|s| s.scan_code == scan && s.command);
        let key = InjectedKey {
            scan,
            command,
            down,
        };
        self.held.insert(scan, key.clone());
        key
    }
    pub fn release_all(&mut self) -> Vec<InjectedKey> {
        self.ctrl.clear();
        self.held
            .drain()
            .map(|(_, k)| InjectedKey { down: false, ..k })
            .collect()
    }
}

pub struct SessionSafety {
    pub state: Ownership,
    pub keys: HashSet<u16>,
    pub buttons: HashSet<u8>,
    last_progress: Instant,
}
impl SessionSafety {
    pub fn new(paused: bool) -> Self {
        Self {
            state: if paused {
                Ownership::Paused
            } else {
                Ownership::Disconnected
            },
            keys: HashSet::new(),
            buttons: HashSet::new(),
            last_progress: Instant::now(),
        }
    }
    pub fn transition(&mut self, next: Ownership) -> bool {
        let legal = matches!(
            (&self.state, &next),
            (_, Ownership::Paused | Ownership::Disconnected)
                | (
                    Ownership::Disconnected | Ownership::Paused,
                    Ownership::Local
                )
                | (Ownership::Local, Ownership::Requesting)
                | (Ownership::Requesting, Ownership::Remote | Ownership::Local)
                | (Ownership::Remote, Ownership::Returning)
                | (Ownership::Returning, Ownership::Local)
        );
        if legal {
            self.state = next;
            self.last_progress = Instant::now();
        }
        legal
    }
    pub fn progress(&mut self) {
        self.last_progress = Instant::now();
    }
    pub fn expired(&self) -> bool {
        matches!(
            self.state,
            Ownership::Remote | Ownership::Requesting | Ownership::Returning
        ) && self.last_progress.elapsed() > Duration::from_secs(1)
    }
    pub fn track(&mut self, input: &Input) {
        match input {
            Input::Key { scan, down } => {
                if *down {
                    self.keys.insert(*scan);
                } else {
                    self.keys.remove(scan);
                }
            }
            Input::Button { button, down } => {
                if *down {
                    self.buttons.insert(*button);
                } else {
                    self.buttons.remove(button);
                }
            }
            _ => {}
        }
    }
    pub fn can_handoff(&self) -> bool {
        self.keys.is_empty() && self.buttons.is_empty()
    }
    pub fn release(&mut self) -> Vec<Input> {
        self.keys
            .drain()
            .map(|scan| Input::Key { scan, down: false })
            .chain(self.buttons.drain().map(|button| Input::Button {
                button,
                down: false,
            }))
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_direct_remote_transition() {
        let mut s = SessionSafety::new(false);
        assert!(!s.transition(Ownership::Remote));
        assert!(s.transition(Ownership::Local));
        assert!(s.transition(Ownership::Requesting));
        assert!(s.transition(Ownership::Remote));
    }
    #[test]
    fn key_up_keeps_original_mapping() {
        let mut m = KeyMapper::default();
        let s = Settings::default();
        m.map(0x1d, true, &s, false);
        assert!(m.map(0x2e, true, &s, false).command);
        assert!(m.map(0x2e, false, &s, true).command);
    }
    #[test]
    fn terminal_ctrl_c_is_not_copy() {
        let mut m = KeyMapper::default();
        let s = Settings::default();
        m.map(0x1d, true, &s, true);
        assert!(!m.map(0x2e, true, &s, true).command);
    }
    #[test]
    fn all_held_inputs_released_on_loss() {
        let mut s = SessionSafety::new(false);
        s.track(&Input::Key {
            scan: 29,
            down: true,
        });
        s.track(&Input::Button {
            button: 0,
            down: true,
        });
        assert!(!s.can_handoff());
        assert_eq!(s.release().len(), 2);
        assert!(s.can_handoff());
    }
    #[test]
    fn malformed_motion_rejected() {
        assert!(!Input::Motion {
            dx: f64::NAN,
            dy: 0.0
        }
        .valid());
    }
}

#[derive(Default)]
pub struct ClickTracker {
    last: [Option<(f64, f64, std::time::Instant, u32)>; 5],
}
impl ClickTracker {
    pub fn count(
        &mut self,
        button: u8,
        down: bool,
        x: f64,
        y: f64,
        now: std::time::Instant,
        interval: Duration,
    ) -> u32 {
        let Some(slot) = self.last.get_mut(button as usize) else {
            return 1;
        };
        if !down {
            return slot.map(|(_, _, _, n)| n).unwrap_or(1);
        }
        let count = slot
            .filter(|(px, py, time, _)| {
                now.saturating_duration_since(*time) <= interval
                    && (x - px).abs() <= 4.0
                    && (y - py).abs() <= 4.0
            })
            .map(|(_, _, _, n)| n.saturating_add(1).min(3))
            .unwrap_or(1);
        *slot = Some((x, y, now, count));
        count
    }
}
#[cfg(test)]
mod click_tests {
    use super::*;
    #[test]
    fn double_click_requires_same_button_position_and_interval() {
        let mut c = ClickTracker::default();
        let t = std::time::Instant::now();
        let d = Duration::from_millis(500);
        assert_eq!(c.count(0, true, 10.0, 10.0, t, d), 1);
        assert_eq!(c.count(0, false, 10.0, 10.0, t, d), 1);
        assert_eq!(
            c.count(0, true, 10.0, 10.0, t + Duration::from_millis(100), d),
            2
        );
        assert_eq!(c.count(1, true, 10.0, 10.0, t, d), 1);
        assert_eq!(
            c.count(0, true, 100.0, 10.0, t + Duration::from_millis(200), d),
            1
        );
    }
}
