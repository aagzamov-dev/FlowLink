use serde::{Deserialize, Serialize};

pub const PORT: u16 = 47831;
pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Ownership {
    Local,
    Requesting,
    Remote,
    Returning,
    Paused,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Preset {
    WindowsFamiliar,
    MacNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shortcut {
    pub scan_code: u16,
    pub command: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub schema: u32,
    pub device_name: String,
    pub paused: bool,
    pub preset: Preset,
    pub allow_local_input: bool,
    pub pointer_speed: f64,
    pub reverse_scroll: bool,
    pub edge_resistance_ms: u64,
    pub hotkey_only: bool,
    pub terminal_apps: Vec<String>,
    pub shortcuts: Vec<Shortcut>,
    pub interface: Option<String>,
    pub diagnostics: bool,
    pub emergency_key: u16,
    pub language_switch: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: 1,
            device_name: hostname::get()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "My computer".into()),
            paused: false,
            preset: Preset::WindowsFamiliar,
            allow_local_input: true,
            pointer_speed: 1.0,
            reverse_scroll: false,
            edge_resistance_ms: 100,
            hotkey_only: false,
            terminal_apps: vec!["com.apple.Terminal".into(), "com.googlecode.iterm2".into()],
            shortcuts: [0x2e, 0x2f, 0x2d, 0x1e, 0x2c, 0x1f, 0x21, 0x18, 0x19, 0x31]
                .into_iter()
                .map(|scan_code| Shortcut {
                    scan_code,
                    command: true,
                })
                .collect(),
            interface: None,
            diagnostics: false,
            emergency_key: 1,
            language_switch: "winSpace".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.schema != 1 {
            return Err("Unsupported settings version".into());
        }
        if self.device_name.trim().is_empty()
            || self.device_name.len() > 80
            || self.device_name.chars().any(char::is_control)
        {
            return Err("Device name must contain 1–80 printable bytes".into());
        }
        if !self.pointer_speed.is_finite()
            || !(0.25..=3.0).contains(&self.pointer_speed)
            || self.edge_resistance_ms > 1000
        {
            return Err("Invalid pointer settings".into());
        }
        if !["winSpace", "ctrlSpace"].contains(&self.language_switch.as_str()) {
            return Err("Unsupported language shortcut".into());
        }
        if ![1, 0x58].contains(&self.emergency_key) {
            return Err("Choose Escape or F12 for emergency release".into());
        }
        if self.terminal_apps.len() > 64
            || self.terminal_apps.iter().any(|s| s.len() > 200)
            || self.shortcuts.len() > 64
        {
            return Err("Too many shortcut rules".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl Rect {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
    pub fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|v| v.is_finite() && v.abs() < 100_000.0)
            && self.width > 0.0
            && self.height > 0.0
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub id: String,
    pub device_id: String,
    pub name: String,
    pub number: u32,
    pub rect: Rect,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub scale: f64,
    pub rotation: f64,
    pub online: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Placement {
    pub display_id: String,
    pub rect: Rect,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    pub interface: String,
    pub ip: String,
    pub kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub os: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub online: bool,
    pub busy: bool,
    pub trusted: bool,
    pub favorite: bool,
    pub auto_connect: bool,
    pub last_seen: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDevice {
    pub device: Device,
    pub certificate: String,
    pub placements: Vec<Placement>,
    #[serde(default)]
    pub last_local_ip: Option<String>,
    #[serde(default)]
    pub host_display_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub accessibility: bool,
    pub monitoring: bool,
    pub filtering: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingPrompt {
    pub request_id: String,
    pub name: String,
    pub address: String,
    pub code: Option<String>,
    pub expires_at: u64,
    pub awaiting_code: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub device_id: String,
    pub os: String,
    pub settings: Settings,
    pub ownership: Ownership,
    pub devices: Vec<Device>,
    pub addresses: Vec<Address>,
    pub port: u16,
    pub displays: Vec<Display>,
    pub placements: Vec<Placement>,
    pub connected_id: Option<String>,
    pub permissions: Permissions,
    pub pairing: Option<PairingPrompt>,
    pub error: Option<String>,
    pub status: String,
    pub latency_ms: Option<f64>,
    pub peer_paused: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputSettings {
    pub preset: Preset,
    pub pointer_speed: f64,
    pub reverse_scroll: bool,
    pub terminal_apps: Vec<String>,
    pub shortcuts: Vec<Shortcut>,
    pub language_switch: String,
}
impl InputSettings {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            preset: s.preset.clone(),
            pointer_speed: s.pointer_speed,
            reverse_scroll: s.reverse_scroll,
            terminal_apps: s.terminal_apps.clone(),
            shortcuts: s.shortcuts.clone(),
            language_switch: s.language_switch.clone(),
        }
    }
    pub fn apply(self, s: &mut Settings) -> std::result::Result<(), String> {
        let mut next = s.clone();
        next.preset = self.preset;
        next.pointer_speed = self.pointer_speed;
        next.reverse_scroll = self.reverse_scroll;
        next.terminal_apps = self.terminal_apps;
        next.shortcuts = self.shortcuts;
        next.language_switch = self.language_switch;
        next.validate()?;
        *s = next;
        Ok(())
    }
}
