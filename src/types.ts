export type Ownership =
  "local" | "requesting" | "remote" | "returning" | "paused" | "disconnected";
export type Preset = "windowsFamiliar" | "macNative";
export interface Shortcut {
  scanCode: number;
  command: boolean;
}
export type Direction = "windowsToMac" | "macToWindows";
export interface ShortcutBinding {
  direction: Direction;
  sourceKey: number;
  sourceModifiers: number;
  targetKey: number;
  targetModifiers: number;
}
export interface Settings {
  schema: number;
  deviceName: string;
  paused: boolean;
  preset: Preset;
  allowLocalInput: boolean;
  pointerSpeed: number;
  reverseScroll: boolean;
  edgeResistanceMs: number;
  hotkeyOnly: boolean;
  terminalApps: string[];
  shortcuts: Shortcut[];
  bindings: ShortcutBinding[];
  interface: string | null;
  diagnostics: boolean;
  autoCheckUpdates: boolean;
  emergencyKey: number;
  languageSwitch: "winSpace" | "ctrlSpace";
}
export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface Display {
  id: string;
  deviceId: string;
  name: string;
  number: number;
  rect: Rect;
  pixelWidth: number;
  pixelHeight: number;
  scale: number;
  rotation: number;
  online: boolean;
}
export interface Placement {
  displayId: string;
  rect: Rect;
}
export interface Address {
  interface: string;
  ip: string;
  kind: string;
}
export interface Device {
  id: string;
  name: string;
  os: string;
  addresses: string[];
  port: number;
  online: boolean;
  busy: boolean;
  trusted: boolean;
  favorite: boolean;
  autoConnect: boolean;
  lastSeen: number;
}
export interface PairingPrompt {
  requestId: string;
  name: string;
  address: string;
  code: string | null;
  expiresAt: number;
  awaitingCode: boolean;
}
export interface Snapshot {
  deviceId: string;
  os: string;
  settings: Settings;
  ownership: Ownership;
  devices: Device[];
  addresses: Address[];
  port: number;
  displays: Display[];
  placements: Placement[];
  connectedId: string | null;
  permissions: {
    accessibility: boolean;
    monitoring: boolean;
    filtering: boolean;
  };
  pairing: PairingPrompt | null;
  error: string | null;
  status: string;
  latencyMs: number | null;
  peerPaused: boolean;
  controllerId: string | null;
  controlFromEither: boolean;
}
export type Page = "devices" | "displays" | "settings";
export const preview: Snapshot = {
  deviceId: "",
  os: "preview",
  settings: {
    schema: 1,
    deviceName: "This computer",
    paused: false,
    preset: "windowsFamiliar",
    allowLocalInput: true,
    pointerSpeed: 1,
    reverseScroll: false,
    edgeResistanceMs: 100,
    hotkeyOnly: false,
    terminalApps: ["com.apple.Terminal", "com.googlecode.iterm2"],
    shortcuts: [],
    bindings: [],
    interface: null,
    diagnostics: false,
    autoCheckUpdates: true,
    emergencyKey: 1,
    languageSwitch: "winSpace",
  },
  ownership: "disconnected",
  devices: [],
  addresses: [],
  port: 47831,
  displays: [],
  placements: [],
  connectedId: null,
  permissions: { accessibility: false, monitoring: false, filtering: false },
  pairing: null,
  error: null,
  status: "Desktop app required",
  latencyMs: null,
  peerPaused: false,
  controllerId: null,
  controlFromEither: true,
};

export type Run = <T = void>(
  command: string,
  args?: Record<string, unknown>,
) => Promise<T | undefined>;
