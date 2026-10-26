use crate::model::{Ownership, Preset, Settings};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq))]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Input {
    Motion {
        dx: f64,
        dy: f64,
    },
    Button {
        button: u8,
        down: bool,
    },
    Scroll {
        x: f64,
        y: f64,
        phase: u8,
        momentum: u8,
    },
    Key {
        scan: u16,
        down: bool,
    },
}
impl Input {
    pub fn valid(&self) -> bool {
        match self {
            Self::Motion { dx, dy } => {
                dx.is_finite() && dy.is_finite() && dx.abs() < 10000.0 && dy.abs() < 10000.0
            }
            Self::Scroll {
                x,
                y,
                phase,
                momentum,
            } => {
                x.is_finite()
                    && y.is_finite()
                    && x.abs() < 10000.0
                    && y.abs() < 10000.0
                    && *phase & !0x8f == 0
                    && *momentum <= 3
            }
            Self::Button { button, .. } => *button < 5,
            Self::Key { scan, .. } => *scan <= 0x1ff,
        }
    }
}
#[derive(Debug, Clone)]
pub enum NativeEvent {
    Input {
        event: Input,
        captured: bool,
        generation: u64,
    },
    Position(f64, f64),
    Emergency,
    Switch,
    Failure(String),
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
    fn touchpad_may_begin_phase_is_valid() {
        for phase in [0, 1, 2, 4, 8, 128] {
            assert!(Input::Scroll {
                x: 0.0,
                y: 0.0,
                phase,
                momentum: 0
            }
            .valid());
        }
        assert!(!Input::Scroll {
            x: 0.0,
            y: 0.0,
            phase: 16,
            momentum: 0
        }
        .valid());
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

pub fn modifier(scan: u16) -> u8 {
    match scan {
        0x1d | 0x11d => 1,
        0x2a | 0x36 => 2,
        0x38 | 0x138 => 4,
        0x15b | 0x15c => 8,
        _ => 0,
    }
}

#[derive(Default)]
pub struct ShortcutMapper {
    physical: HashSet<u16>,
    mapped: HashMap<u16, (u16, u8, u8)>,
    emitted_modifiers: HashSet<u16>,
    menu_used: u8,
    menu_deferred: u8,
    custom_modifiers: bool,
}
impl ShortcutMapper {
    pub fn map(
        &mut self,
        scan: u16,
        down: bool,
        settings: &Settings,
        direction: crate::model::Direction,
        terminal: bool,
    ) -> Vec<Input> {
        self.custom_modifiers = settings.preset == Preset::CustomModifiers;
        let mut output = Vec::new();
        let was_emitted = self.emitted_modifiers.contains(&scan);
        let menu_held = self
            .physical
            .iter()
            .fold(0, |mask, scan| mask | modifier(*scan))
            & 12;
        let menu_bit = modifier(scan) & 12;
        if down && menu_bit != 0 && menu_held & menu_bit == 0 {
            self.menu_used &= !menu_bit;
            self.menu_deferred |= menu_bit;
        }
        if down && modifier(scan) == 0 {
            self.menu_used |= menu_held;
        }
        if down {
            self.physical.insert(scan);
        } else {
            self.physical.remove(&scan);
        }
        if modifier(scan) == 0 {
            if down && !self.mapped.contains_key(&scan) {
                let modifiers = self.physical.iter().fold(0, |m, s| m | modifier(*s));
                let explicit = settings.bindings.iter().find(|b| {
                    b.direction == direction
                        && b.source_key == scan
                        && b.source_modifiers == modifiers
                });
                let source = if direction == crate::model::Direction::WindowsToMac {
                    1
                } else {
                    8
                };
                let target = if source == 1 { 8 } else { 1 };
                let common = settings.preset == Preset::WindowsFamiliar
                    && !terminal
                    && (modifiers == source || modifiers == source | 2)
                    && settings
                        .shortcuts
                        .iter()
                        .any(|s| s.scan_code == scan && s.command);
                let language = direction == crate::model::Direction::WindowsToMac
                    && settings.language_switch == "winSpace"
                    && scan == 0x39
                    && modifiers == 8;
                let mapping = explicit
                    .map(|b| (b.target_key, b.source_modifiers, b.target_modifiers))
                    .unwrap_or(if language {
                        (scan, 8, 1)
                    } else if common {
                        (scan, modifiers, (modifiers & !source) | target)
                    } else {
                        (scan, 0, 0)
                    });
                self.mapped.insert(scan, mapping);
            }
            if !down {
                if let Some((target, _, _)) = self.mapped.remove(&scan) {
                    if !self.mapped.values().any(|(key, _, _)| *key == target) {
                        output.push(Input::Key {
                            scan: target,
                            down: false,
                        });
                    }
                }
            }
        }
        let physical_mask = self.physical.iter().fold(0, |m, s| m | modifier(*s));
        let removed = self.mapped.values().fold(0, |m, (_, source, _)| m | source);
        let added = self.mapped.values().fold(0, |m, (_, _, target)| m | target);
        let translate = |mask| {
            if self.custom_modifiers {
                settings.modifier_mapping.translate(mask, direction)
            } else {
                mask
            }
        };
        let desired_mask = translate(physical_mask & !removed) | added;
        let defer_menu =
            direction == crate::model::Direction::MacToWindows && !self.custom_modifiers;
        if down && modifier(scan) == 0 {
            self.menu_deferred = (self.menu_deferred & !menu_held) | (menu_held & !desired_mask);
        }
        let mut desired = self
            .physical
            .iter()
            .copied()
            .filter(|s| modifier(*s) != 0 && modifier(*s) & removed == 0)
            .map(|s| {
                if !self.custom_modifiers || modifier(s) == 2 {
                    return s;
                }
                let right = matches!(s, 0x11d | 0x138 | 0x15c);
                match (translate(modifier(s)), right) {
                    (1, false) => 0x1d,
                    (1, true) => 0x11d,
                    (4, false) => 0x38,
                    (4, true) => 0x138,
                    (8, false) => 0x15b,
                    (8, true) => 0x15c,
                    _ => s,
                }
            })
            .collect::<HashSet<_>>();
        for (bit, key) in [(1, 0x1d), (2, 0x2a), (4, 0x38), (8, 0x15b)] {
            if desired_mask & bit != 0 && !desired.iter().any(|s| modifier(*s) == bit) {
                desired.insert(key);
            }
        }
        if defer_menu {
            desired.retain(|s| modifier(*s) & self.menu_deferred & physical_mask == 0);
        }
        let mut released = self
            .emitted_modifiers
            .difference(&desired)
            .copied()
            .collect::<Vec<_>>();
        let mut pressed = desired
            .difference(&self.emitted_modifiers)
            .copied()
            .collect::<Vec<_>>();
        released.sort_unstable();
        pressed.sort_unstable();
        output.extend(
            released
                .into_iter()
                .map(|scan| Input::Key { scan, down: false }),
        );
        output.extend(
            pressed
                .into_iter()
                .map(|scan| Input::Key { scan, down: true }),
        );
        self.emitted_modifiers = desired;
        if defer_menu
            && !down
            && menu_bit != 0
            && !was_emitted
            && self.menu_used & menu_bit == 0
            && physical_mask & menu_bit == 0
        {
            output.push(Input::Key { scan, down: true });
            output.push(Input::Key { scan, down: false });
        }
        if down && modifier(scan) == 0 {
            if let Some((target, _, _)) = self.mapped.get(&scan) {
                output.push(Input::Key {
                    scan: *target,
                    down: true,
                });
            }
        }
        output
    }
    pub fn pointer_modifiers(&mut self) -> Vec<u16> {
        if self.custom_modifiers {
            return Vec::new();
        }
        self.menu_used |= 12;
        self.menu_deferred = 0;
        let pending = self
            .physical
            .iter()
            .copied()
            .filter(|scan| modifier(*scan) & 12 != 0 && !self.emitted_modifiers.contains(scan))
            .collect::<Vec<_>>();
        self.emitted_modifiers.extend(&pending);
        pending
    }
    pub fn clear(&mut self) {
        self.menu_used = 0;
        self.menu_deferred = 0;
        self.physical.clear();
        self.mapped.clear();
        self.emitted_modifiers.clear();
    }
}

impl Input {
    pub fn merge_motion(&mut self, next: &Self) -> bool {
        if let (Self::Motion { dx, dy }, Self::Motion { dx: x, dy: y }) = (self, next) {
            if (*dx + x).abs() < 10000.0 && (*dy + y).abs() < 10000.0 {
                *dx += x;
                *dy += y;
                return true;
            }
        }
        false
    }
}

#[derive(Default)]
pub struct ScrollAccumulator {
    x: f64,
    y: f64,
}
impl ScrollAccumulator {
    pub fn take(&mut self, x: f64, y: f64) -> (i32, i32) {
        self.x += x;
        self.y += y;
        let result = (self.x.trunc() as i32, self.y.trunc() as i32);
        self.x -= result.0 as f64;
        self.y -= result.1 as f64;
        result
    }
}

#[cfg(test)]
mod mapping_tests {
    use super::*;
    use crate::model::{Direction, ShortcutBinding};
    #[test]
    fn terminal_control_copy_stays_control_until_key_up() {
        let mut mapper = ShortcutMapper::default();
        let settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        mapper.map(0x1d, true, &settings, Direction::WindowsToMac, true);
        let down = mapper.map(0x2e, true, &settings, Direction::WindowsToMac, true);
        assert!(matches!(
            down.as_slice(),
            [Input::Key {
                scan: 0x2e,
                down: true
            }]
        ));
        let up = mapper.map(0x2e, false, &settings, Direction::WindowsToMac, false);
        assert!(matches!(
            up.as_slice(),
            [Input::Key {
                scan: 0x2e,
                down: false
            }]
        ));
        let released = mapper.map(0x1d, false, &settings, Direction::WindowsToMac, false);
        assert!(matches!(
            released.as_slice(),
            [Input::Key {
                scan: 0x1d,
                down: false
            }]
        ));
    }
    #[test]
    fn exact_modifiers_preserve_altgr_combinations() {
        let mut mapper = ShortcutMapper::default();
        let settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        mapper.map(0x1d, true, &settings, Direction::WindowsToMac, false);
        mapper.map(0x138, true, &settings, Direction::WindowsToMac, false);
        let output = mapper.map(0x2e, true, &settings, Direction::WindowsToMac, false);
        assert!(!output
            .iter()
            .any(|k| matches!(k, Input::Key { scan: 0x15b, .. })));
    }
    #[test]
    fn command_copy_never_sends_a_standalone_windows_key() {
        let mut mapper = ShortcutMapper::default();
        let settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        let output = [(0x15b, true), (0x2e, true), (0x2e, false), (0x15b, false)]
            .into_iter()
            .flat_map(|(scan, down)| {
                mapper.map(scan, down, &settings, Direction::MacToWindows, false)
            })
            .collect::<Vec<_>>();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|key| matches!(
            key,
            Input::Key {
                scan: 0x1d | 0x2e,
                ..
            }
        )));
        assert!(mapper
            .map(0x15b, true, &settings, Direction::MacToWindows, false)
            .is_empty());
        let tap = mapper.map(0x15b, false, &settings, Direction::MacToWindows, false);
        assert!(matches!(
            tap.as_slice(),
            [
                Input::Key {
                    scan: 0x15b,
                    down: true
                },
                Input::Key {
                    scan: 0x15b,
                    down: false
                }
            ]
        ));
    }
    #[test]
    fn custom_option_shortcut_does_not_activate_windows_menus() {
        let mut mapper = ShortcutMapper::default();
        let mut settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        settings.bindings.push(ShortcutBinding {
            direction: Direction::MacToWindows,
            source_key: 0x2e,
            source_modifiers: 4,
            target_key: 0x2e,
            target_modifiers: 1,
        });
        let output = [(0x38, true), (0x2e, true), (0x2e, false), (0x38, false)]
            .into_iter()
            .flat_map(|(scan, down)| {
                mapper.map(scan, down, &settings, Direction::MacToWindows, false)
            })
            .collect::<Vec<_>>();
        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|key| matches!(
            key,
            Input::Key {
                scan: 0x1d | 0x2e,
                ..
            }
        )));
    }
    #[test]
    fn deferred_windows_modifier_is_forwarded_before_a_pointer_click() {
        let mut mapper = ShortcutMapper::default();
        let settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        mapper.map(0x15b, true, &settings, Direction::MacToWindows, false);
        assert_eq!(mapper.pointer_modifiers(), vec![0x15b]);
        assert!(mapper.pointer_modifiers().is_empty());
        let up = mapper.map(0x15b, false, &settings, Direction::MacToWindows, false);
        assert!(matches!(
            up.as_slice(),
            [Input::Key {
                scan: 0x15b,
                down: false
            }]
        ));
    }
    #[test]
    fn shift_command_copy_preserves_shift_without_sending_windows_key() {
        let mut mapper = ShortcutMapper::default();
        let settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        let output = [
            (0x2a, true),
            (0x15c, true),
            (0x2e, true),
            (0x2e, false),
            (0x15c, false),
            (0x2a, false),
        ]
        .into_iter()
        .flat_map(|(scan, down)| mapper.map(scan, down, &settings, Direction::MacToWindows, false))
        .collect::<Vec<_>>();
        assert_eq!(output.len(), 6);
        for scan in [0x2a, 0x1d, 0x2e] {
            assert_eq!(
                output
                    .iter()
                    .filter(
                        |key| matches!(key, Input::Key { scan: code, down: true } if *code == scan)
                    )
                    .count(),
                1
            );
            assert_eq!(
                output
                    .iter()
                    .filter(
                        |key| matches!(key, Input::Key { scan: code, down: false } if *code == scan)
                    )
                    .count(),
                1
            );
        }
    }
    #[test]
    fn custom_target_survives_settings_change_until_key_up() {
        let mut mapper = ShortcutMapper::default();
        let mut settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        settings.bindings.push(ShortcutBinding {
            direction: Direction::WindowsToMac,
            source_key: 0x2e,
            source_modifiers: 1,
            target_key: 0x3f,
            target_modifiers: 0,
        });
        mapper.map(0x1d, true, &settings, Direction::WindowsToMac, false);
        let down = mapper.map(0x2e, true, &settings, Direction::WindowsToMac, false);
        assert!(down.iter().any(|k| matches!(
            k,
            Input::Key {
                scan: 0x3f,
                down: true
            }
        )));
        settings.bindings.clear();
        let up = mapper.map(0x2e, false, &settings, Direction::WindowsToMac, true);
        assert!(up.iter().any(|k| matches!(
            k,
            Input::Key {
                scan: 0x3f,
                down: false
            }
        )));
    }
    #[test]
    fn fractional_scroll_accumulates_without_losing_small_movements() {
        let mut scroll = ScrollAccumulator::default();
        assert_eq!(scroll.take(0.25, -0.25), (0, 0));
        assert_eq!(scroll.take(0.25, -0.25), (0, 0));
        assert_eq!(scroll.take(0.25, -0.25), (0, 0));
        assert_eq!(scroll.take(0.25, -0.25), (1, -1));
    }
    #[test]
    fn conflicting_custom_rules_are_rejected() {
        let mut settings = Settings {
            preset: Preset::WindowsFamiliar,
            ..Settings::default()
        };
        let rule = ShortcutBinding {
            direction: Direction::WindowsToMac,
            source_key: 30,
            source_modifiers: 1,
            target_key: 31,
            target_modifiers: 8,
        };
        settings.bindings = vec![rule.clone(), rule];
        assert!(settings.validate().is_err());
    }
}

#[cfg(test)]
mod modifier_mapping_tests {
    use super::*;
    use crate::model::{Direction, ModifierMapping};

    #[test]
    fn modifiers_map_both_directions_and_release_without_stuck_keys() {
        let settings = Settings::default();
        for (mac, windows) in [
            (0x15b, 0x1d),
            (0x38, 0x15b),
            (0x1d, 0x38),
            (0x15c, 0x11d),
            (0x138, 0x15c),
            (0x11d, 0x138),
        ] {
            for (direction, source, target) in [
                (Direction::MacToWindows, mac, windows),
                (Direction::WindowsToMac, windows, mac),
            ] {
                let mut mapper = ShortcutMapper::default();
                assert_eq!(
                    mapper.map(source, true, &settings, direction, false),
                    vec![Input::Key {
                        scan: target,
                        down: true
                    }]
                );
                assert_eq!(
                    mapper.map(0x2e, true, &settings, direction, false),
                    vec![Input::Key {
                        scan: 0x2e,
                        down: true
                    }]
                );
                assert!(mapper.pointer_modifiers().is_empty());
                assert_eq!(
                    mapper.map(0x2e, false, &settings, direction, false),
                    vec![Input::Key {
                        scan: 0x2e,
                        down: false
                    }]
                );
                assert_eq!(
                    mapper.map(source, false, &settings, direction, false),
                    vec![Input::Key {
                        scan: target,
                        down: false
                    }]
                );
                assert!(mapper.emitted_modifiers.is_empty());
            }
        }
    }

    #[test]
    fn mapping_is_configurable_and_rejects_ambiguous_assignments() {
        let mut settings = Settings {
            modifier_mapping: ModifierMapping {
                command: 8,
                option: 4,
                control: 1,
            },
            ..Settings::default()
        };
        settings.validate().unwrap();
        let mut mapper = ShortcutMapper::default();
        assert_eq!(
            mapper.map(0x15b, true, &settings, Direction::MacToWindows, false),
            vec![Input::Key {
                scan: 0x15b,
                down: true
            }]
        );
        settings.modifier_mapping.command = 1;
        assert!(settings.validate().is_err());
    }

    #[test]
    fn explicit_shortcuts_override_custom_modifiers_and_restore_held_modifier() {
        let mut settings = Settings::default();
        settings.bindings.push(crate::model::ShortcutBinding {
            direction: Direction::MacToWindows,
            source_key: 0x2e,
            source_modifiers: 8,
            target_key: 0x2f,
            target_modifiers: 4,
        });
        let mut mapper = ShortcutMapper::default();
        mapper.map(0x15b, true, &settings, Direction::MacToWindows, false);
        assert_eq!(
            mapper.map(0x2e, true, &settings, Direction::MacToWindows, false),
            vec![
                Input::Key {
                    scan: 0x1d,
                    down: false
                },
                Input::Key {
                    scan: 0x38,
                    down: true
                },
                Input::Key {
                    scan: 0x2f,
                    down: true
                }
            ]
        );
        assert_eq!(
            mapper.map(0x2e, false, &settings, Direction::MacToWindows, false),
            vec![
                Input::Key {
                    scan: 0x2f,
                    down: false
                },
                Input::Key {
                    scan: 0x38,
                    down: false
                },
                Input::Key {
                    scan: 0x1d,
                    down: true
                }
            ]
        );
        mapper.map(0x15b, false, &settings, Direction::MacToWindows, false);
        assert!(mapper.emitted_modifiers.is_empty());
    }
}
