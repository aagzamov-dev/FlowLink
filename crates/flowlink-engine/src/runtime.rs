use crate::{
    discovery::{self, Discovery},
    geometry::{self, Side},
    input::{NativeEvent, SessionSafety},
    model::*,
    pairing::{self, Exchange},
    platform::{self, Native},
    protocol::{self, Hello, Message, PairMessage},
    storage::Store,
    transport, Result,
};
#[cfg(target_os = "macos")]
use rand::Rng;
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, oneshot},
    time::timeout,
};

#[cfg(target_os = "macos")]
use tokio::net::TcpListener;

type SecureStream = tokio_rustls::TlsStream<TcpStream>;
struct Peer {
    id: String,
    certificate: Vec<u8>,
    address: SocketAddr,
    new_pair: bool,
    controller: bool,
    local_ip: String,
}
struct Pending {
    id: String,
    sender: oneshot::Sender<String>,
}
pub struct Engine {
    pub store: Arc<Store>,
    pub native: Arc<Native>,
    state: Mutex<Snapshot>,
    connection: Mutex<Option<mpsc::Sender<Message>>>,
    pending: Mutex<Option<Pending>>,
    busy: AtomicBool,
    reconnect_suspended: AtomicBool,
    #[cfg(target_os = "macos")]
    limiter: Mutex<crate::pairing::PairingLimiter>,
    safety: Mutex<SessionSafety>,
    sequence: std::sync::atomic::AtomicU64,
    edge_since: Mutex<Option<(String, Instant)>>,
    return_edges: Mutex<Vec<(geometry::Edge, Rect, Rect)>>,
    shutdown: AtomicBool,
    switch_requested: AtomicBool,
    pub identify_requested: AtomicBool,
    cancel: tokio::sync::Notify,
}
impl Engine {
    pub fn start(directory: &Path) -> Result<Arc<Self>> {
        let store = Arc::new(Store::open(directory)?);
        let settings = store.settings();
        let displays = platform::displays(&store.identity.id)?;
        let (native, events) = Native::start()?;
        native
            .gate
            .emergency_key
            .store(settings.emergency_key, Ordering::SeqCst);
        let engine = Arc::new(Self {
            state: Mutex::new(Snapshot {
                device_id: store.identity.id.clone(),
                os: std::env::consts::OS.into(),
                settings: settings.clone(),
                ownership: if settings.paused {
                    Ownership::Paused
                } else {
                    Ownership::Disconnected
                },
                devices: store
                    .trusted()
                    .into_iter()
                    .map(|mut d| {
                        d.device.online = false;
                        d.device.trusted = true;
                        d.device
                    })
                    .collect(),
                addresses: discovery::addresses(),
                port: PORT,
                displays,
                placements: Vec::new(),
                connected_id: None,
                permissions: platform::permissions(),
                pairing: None,
                error: None,
                status: "Ready to connect".into(),
                latency_ms: None,
                peer_paused: false,
            }),
            store,
            native,
            connection: Mutex::new(None),
            pending: Mutex::new(None),
            busy: AtomicBool::new(false),
            reconnect_suspended: AtomicBool::new(false),
            #[cfg(target_os = "macos")]
            limiter: Mutex::new(crate::pairing::PairingLimiter::default()),
            safety: Mutex::new(SessionSafety::new(settings.paused)),
            sequence: std::sync::atomic::AtomicU64::new(0),
            edge_since: Mutex::new(None),
            return_edges: Mutex::new(Vec::new()),
            shutdown: AtomicBool::new(false),
            switch_requested: AtomicBool::new(false),
            identify_requested: AtomicBool::new(false),
            cancel: tokio::sync::Notify::new(),
        });
        let weak = Arc::downgrade(&engine);
        std::thread::Builder::new()
            .name("flowlink-discovery".into())
            .spawn(move || {
                let mut discovery = None;
                let mut advertised_name = String::new();
                while let Some(engine) = weak.upgrade() {
                    if engine.shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    let settings = engine.store.settings();
                    let name = settings.device_name;
                    let registration_key = format!(
                        "{}:{:?}:{}:{:?}",
                        name,
                        settings.interface,
                        engine.busy.load(Ordering::SeqCst),
                        discovery::addresses()
                            .iter()
                            .map(|a| (&a.interface, &a.ip))
                            .collect::<Vec<_>>()
                    );
                    if discovery.is_none() || registration_key != advertised_name {
                        discovery = None;
                        match Discovery::new(
                            &engine.store.identity.id,
                            &name,
                            std::env::consts::OS,
                            cfg!(target_os = "macos"),
                            engine.busy.load(Ordering::SeqCst),
                            settings.interface.as_deref(),
                        ) {
                            Ok(d) => {
                                discovery = Some(d);
                                advertised_name = registration_key;
                            }
                            Err(e) => {
                                engine.error(format!("Discovery unavailable; connect by IP. {e}"))
                            }
                        }
                    }
                    if let Some(d) = &discovery {
                        if let Ok(event) = d.events.recv_timeout(Duration::from_millis(1000)) {
                            if let mdns_sd::ServiceEvent::ServiceRemoved(_, fullname) = &event {
                                engine.update(|s| {
                                    for device in &mut s.devices {
                                        if fullname.starts_with(&device.id[..16]) {
                                            device.online = false;
                                        }
                                    }
                                });
                            } else if let Some(device) = Discovery::resolve(event) {
                                engine.discovered(device);
                            }
                        }
                    } else {
                        std::thread::sleep(Duration::from_secs(3));
                    }
                }
            })?;
        let weak = Arc::downgrade(&engine);
        std::thread::Builder::new()
            .name("flowlink-input-router".into())
            .spawn(move || loop {
                let Some(engine) = weak.upgrade() else {
                    break;
                };
                if engine.shutdown.load(Ordering::SeqCst) {
                    break;
                }
                match events.recv_timeout(Duration::from_millis(25)) {
                    Ok(event) => engine.native_event(event),
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        let (x, y) = engine.native.cursor();
                        if cfg!(target_os = "windows") {
                            engine.host_position(x, y);
                        }
                    }
                    Err(_) => break,
                }
            })?;
        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.maintenance().await;
        });
        #[cfg(target_os = "macos")]
        {
            let engine_clone = engine.clone();
            tokio::spawn(async move {
                if let Err(e) = engine_clone.clone().listen().await {
                    engine_clone.error(format!("Cannot listen on port {PORT}: {e}"));
                }
            });
        }
        Ok(engine)
    }
    fn update(&self, f: impl FnOnce(&mut Snapshot)) {
        f(&mut self.state.lock().unwrap_or_else(|e| e.into_inner()));
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
    fn error(&self, message: String) {
        self.update(|s| {
            s.error = Some(message);
        });
    }
    fn ownership(&self, next: Ownership) {
        let mut safety = self.safety.lock().unwrap_or_else(|e| e.into_inner());
        if cfg!(target_os = "macos")
            && matches!(
                next,
                Ownership::Local | Ownership::Paused | Ownership::Disconnected
            )
        {
            safety.release();
        }
        if safety.state == next {
            return;
        }
        if next == Ownership::Remote && safety.state == Ownership::Local {
            safety.transition(Ownership::Requesting);
        }
        if next == Ownership::Local && safety.state == Ownership::Remote {
            safety.transition(Ownership::Returning);
        }
        if safety.transition(next.clone()) {
            drop(safety);
            self.update(|s| s.ownership = next);
        } else {
            self.native.release();
            safety.transition(Ownership::Disconnected);
            drop(safety);
            self.update(|s| {
                s.ownership = Ownership::Disconnected;
                s.error =
                    Some("Invalid input ownership transition; control restored locally".into());
            });
        }
    }
    fn send(&self, message: Message) -> Result<()> {
        let sender = self
            .connection
            .lock()
            .map_err(|_| "Connection unavailable")?
            .clone()
            .ok_or("No device connected")?;
        if sender.try_send(message).is_err() {
            self.native.release();
            return Err("Input delivery could not keep up; control returned locally".into());
        }
        Ok(())
    }
    fn discovered(&self, mut device: Device) {
        if device.id == self.store.identity.id {
            return;
        }
        let saved = self
            .store
            .trusted()
            .into_iter()
            .find(|t| t.device.id == device.id);
        if let Some(saved) = saved {
            device.trusted = true;
            device.favorite = saved.device.favorite;
            device.auto_connect = saved.device.auto_connect;
        }
        self.update(|s| {
            if let Some(existing) = s.devices.iter_mut().find(|d| d.id == device.id) {
                *existing = device;
            } else if s.devices.len() < 256 {
                s.devices.push(device);
            }
        });
    }
    pub fn diagnostics(&self) -> Result<String> {
        if !self.store.settings().diagnostics {
            return Err("Enable diagnostics in Settings first".into());
        }
        let s = self.snapshot();
        Ok(serde_json::to_string_pretty(&serde_json::json!({
            "version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,
            "ownership":s.ownership,"connected":s.connected_id.is_some(),"nativeInputHealthy":self.native.gate.healthy.load(Ordering::SeqCst),
            "permissions":s.permissions,"displayCount":s.displays.len(),"addressCount":s.addresses.len(),"trustedDeviceCount":s.devices.iter().filter(|d|d.trusted).count(),
            "roundTripMs":s.latency_ms,"peerPaused":s.peer_paused,"localPaused":s.settings.paused,
        }))?)
    }
    pub fn save_settings(&self, settings: Settings) -> Result<()> {
        settings.validate()?;
        if !settings.paused && self.store.settings().paused {
            let _ = self.send(Message::Resume);
        }
        if settings.paused {
            self.native.release();
            let _ = self.send(Message::Pause);
            self.ownership(Ownership::Paused);
        }
        self.native
            .gate
            .emergency_key
            .store(settings.emergency_key, Ordering::SeqCst);
        self.store
            .config
            .lock()
            .map_err(|_| "Settings unavailable")?
            .settings = settings.clone();
        self.store.save()?;
        if cfg!(target_os = "windows") {
            let _ = self.send(Message::Configuration {
                settings: InputSettings::from_settings(&settings),
            });
        }
        self.update(|s| {
            s.settings = settings.clone();
            if !settings.paused && s.ownership == Ownership::Paused {
                s.ownership = if s.peer_paused {
                    Ownership::Paused
                } else if s.connected_id.is_some() {
                    Ownership::Local
                } else {
                    Ownership::Disconnected
                };
            }
        });
        if !settings.paused {
            let snapshot = self.snapshot();
            self.ownership(if snapshot.peer_paused {
                Ownership::Paused
            } else if snapshot.connected_id.is_some() {
                Ownership::Local
            } else {
                Ownership::Disconnected
            });
        }
        if self.native.gate.active() {
            if settings.allow_local_input {
                self.native.gate.block_local.store(false, Ordering::SeqCst);
            } else {
                self.return_local();
            }
        }
        Ok(())
    }
    pub fn pause(&self, paused: bool) -> Result<()> {
        let mut settings = self.store.settings();
        settings.paused = paused;
        self.save_settings(settings)
    }
    pub fn return_local(&self) {
        self.native.release();
        let _ = self.send(Message::Release);
        self.ownership(if self.store.settings().paused {
            Ownership::Paused
        } else if self.snapshot().connected_id.is_some() {
            Ownership::Local
        } else {
            Ownership::Disconnected
        });
    }
    pub fn identify(&self) {
        self.identify_requested.store(true, Ordering::SeqCst);
        let _ = self.send(Message::Identify);
    }
    pub fn switch_control(&self) -> Result<()> {
        let s = self.snapshot();
        if s.ownership == Ownership::Remote {
            self.return_local();
            return Ok(());
        }
        if s.ownership != Ownership::Local || s.settings.paused || s.peer_paused {
            return Err("Connect and resume sharing first".into());
        }
        if !self
            .safety
            .lock()
            .map_err(|_| "Input state unavailable")?
            .can_handoff()
        {
            return Err("Release held keys and mouse buttons before switching".into());
        }
        let target = s
            .displays
            .iter()
            .find(|d| d.device_id != s.device_id)
            .ok_or("No Mac display connected")?;
        self.ownership(Ownership::Requesting);
        self.send(Message::Handoff {
            display_id: target.id.clone(),
            x: target.rect.x + target.rect.width / 2.0,
            y: target.rect.y + target.rect.height / 2.0,
        })
    }
    pub fn disconnect(&self) {
        self.reconnect_suspended.store(true, Ordering::SeqCst);
        self.clear_connection();
    }
    fn clear_connection(&self) {
        self.native.release();
        let _ = self.send(Message::Goodbye);
        self.connection
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        self.update(|s| {
            s.connected_id = None;
            s.pairing = None;
            s.placements.clear();
            s.displays.retain(|d| d.device_id == s.device_id);
            s.status = "Disconnected".into();
            s.latency_ms = None;
            s.peer_paused = false;
        });
        self.ownership(if self.store.settings().paused {
            Ownership::Paused
        } else {
            Ownership::Disconnected
        });
    }
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.disconnect();
    }
    pub fn set_device(&self, id: &str, favorite: bool, auto_connect: bool) -> Result<()> {
        let mut config = self
            .store
            .config
            .lock()
            .map_err(|_| "Settings unavailable")?;
        let saved = config
            .trusted
            .iter_mut()
            .find(|d| d.device.id == id)
            .ok_or("Device is not paired")?;
        saved.device.favorite = favorite;
        saved.device.auto_connect = auto_connect;
        drop(config);
        self.store.save()?;
        self.update(|s| {
            if let Some(d) = s.devices.iter_mut().find(|d| d.id == id) {
                d.favorite = favorite;
                d.auto_connect = auto_connect;
            }
        });
        Ok(())
    }
    pub fn forget(&self, id: &str) -> Result<()> {
        if self.snapshot().connected_id.as_deref() == Some(id) {
            self.disconnect();
        }
        self.store
            .config
            .lock()
            .map_err(|_| "Settings unavailable")?
            .trusted
            .retain(|d| d.device.id != id);
        self.store.save()?;
        self.update(|s| {
            if let Some(d) = s.devices.iter_mut().find(|d| d.id == id) {
                d.trusted = false;
                d.favorite = false;
                d.auto_connect = false;
            }
        });
        Ok(())
    }
    pub fn answer_pairing(&self, id: &str, answer: String) -> Result<()> {
        let mut pending = self.pending.lock().map_err(|_| "Pairing unavailable")?;
        if pending.as_ref().map(|p| p.id.as_str()) != Some(id) {
            return Err("Pairing request expired".into());
        }
        pending
            .take()
            .ok_or("Pairing request expired")?
            .sender
            .send(answer)
            .map_err(|_| "Pairing request expired".into())
    }
    pub fn cancel_pairing(&self) {
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        self.cancel.notify_waiters();
        self.update(|s| s.pairing = None);
    }
    async fn read_pairing(&self, stream: &mut TcpStream, seconds: u64) -> Result<PairMessage> {
        tokio::select! {result=timeout(Duration::from_secs(seconds),protocol::read(stream))=>result?,_=self.cancel.notified()=>Err("Pairing cancelled".into())}
    }
    async fn prompt(&self, name: String, address: String, awaiting_code: bool) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = oneshot::channel();
        *self.pending.lock().map_err(|_| "Pairing unavailable")? = Some(Pending {
            id: id.clone(),
            sender: tx,
        });
        self.update(|s| {
            s.pairing = Some(PairingPrompt {
                request_id: id,
                name,
                address,
                code: None,
                expires_at: discovery::now() + 120,
                awaiting_code,
            })
        });
        let result = timeout(Duration::from_secs(120), rx).await;
        self.pending
            .lock()
            .map_err(|_| "Pairing unavailable")?
            .take();
        match result {
            Ok(Ok(answer)) if !answer.is_empty() => Ok(answer),
            _ => Err("Pairing cancelled or expired".into()),
        }
    }
    pub fn save_layout(&self, placements: Vec<Placement>) -> Result<()> {
        if !cfg!(target_os = "windows") {
            return Err("Arrange displays on the Windows controller".into());
        }
        let snapshot = self.snapshot();
        let id = snapshot
            .connected_id
            .ok_or("Connect a Mac before arranging displays")?;
        let local = snapshot
            .displays
            .iter()
            .filter(|d| d.device_id == snapshot.device_id)
            .cloned()
            .collect::<Vec<_>>();
        geometry::validate_layout(&local, &placements)?;
        let remote = snapshot
            .displays
            .iter()
            .filter(|d| d.device_id == id)
            .collect::<Vec<_>>();
        if placements.len() != remote.len()
            || placements.iter().any(|p| {
                !remote.iter().any(|d| {
                    d.id == p.display_id
                        && (d.rect.width - p.rect.width).abs() < 1.0
                        && (d.rect.height - p.rect.height).abs() < 1.0
                })
            })
        {
            return Err("Display identity or dimensions changed; reset the layout".into());
        }
        self.return_local();
        let mut config = self
            .store
            .config
            .lock()
            .map_err(|_| "Settings unavailable")?;
        config
            .trusted
            .iter_mut()
            .find(|d| d.device.id == id)
            .ok_or("Device is not paired")?
            .placements = placements.clone();
        if let Some(saved) = config.trusted.iter_mut().find(|d| d.device.id == id) {
            saved.host_display_ids = local.iter().map(|d| d.id.clone()).collect();
        }
        drop(config);
        self.store.save()?;
        self.update(|s| s.placements = placements.clone());
        self.send(Message::Layout {
            placements,
            host_displays: local,
        })
    }
    pub fn connect(self: &Arc<Self>, address: String) -> Result<()> {
        if !cfg!(target_os = "windows") {
            return Err("Start connections from the Windows computer".into());
        }
        if self
            .busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err("A connection or pairing is already active".into());
        }
        self.reconnect_suspended.store(false, Ordering::SeqCst);
        self.update(|s| {
            s.error = None;
            s.status = "Connecting…".into();
        });
        let engine = self.clone();
        tokio::spawn(async move {
            let result = engine.clone().connect_inner(address).await;
            engine.finish_connection(result);
        });
        Ok(())
    }
    fn finish_connection(&self, result: Result<()>) {
        self.native.release();
        self.safety
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .release();
        self.clear_connection();
        self.pending
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        self.busy.store(false, Ordering::SeqCst);
        if let Err(error) = result {
            self.error(error.to_string());
        }
    }
    async fn connect_inner(self: Arc<Self>, address: String) -> Result<()> {
        let target = normalize_address(&address)?;
        let endpoints = timeout(Duration::from_secs(4), tokio::net::lookup_host(&target))
            .await??
            .filter(|a| discovery::is_lan(a.ip()))
            .collect::<Vec<_>>();
        if endpoints.is_empty() {
            return Err("Use a private LAN IP address or hostname".into());
        }
        let mut connected = None;
        let settings = self.store.settings();
        'endpoints: for endpoint in endpoints {
            let last = self
                .store
                .trusted()
                .into_iter()
                .find(|t| t.device.addresses.contains(&endpoint.ip().to_string()))
                .and_then(|t| t.last_local_ip);
            let interfaces = if_addrs::get_if_addrs().unwrap_or_default();
            let mut local = discovery::addresses()
                .into_iter()
                .filter(|a| {
                    settings
                        .interface
                        .as_ref()
                        .is_none_or(|name| &a.interface == name)
                })
                .filter_map(|a| {
                    let ip = a.ip.parse::<std::net::IpAddr>().ok()?;
                    if ip.is_ipv4() != endpoint.is_ipv4() {
                        return None;
                    }
                    let same_network = interfaces.iter().any(|i| {
                        i.ip() == ip
                            && match (&i.addr, endpoint.ip()) {
                                (if_addrs::IfAddr::V4(local), std::net::IpAddr::V4(remote)) => {
                                    u32::from(local.ip) & u32::from(local.netmask)
                                        == u32::from(remote) & u32::from(local.netmask)
                                }
                                (if_addrs::IfAddr::V6(local), std::net::IpAddr::V6(remote)) => {
                                    u128::from(local.ip) & u128::from(local.netmask)
                                        == u128::from(remote) & u128::from(local.netmask)
                                }
                                _ => false,
                            }
                    });
                    if !same_network
                        && settings.interface.is_none()
                        && last.as_deref() != Some(&a.ip)
                    {
                        return None;
                    }
                    Some((
                        last.as_deref() != Some(&a.ip),
                        a.kind != "Ethernet",
                        std::net::SocketAddr::new(ip, 0),
                    ))
                })
                .collect::<Vec<_>>();
            local.sort_by_key(|a| (a.0, a.1));
            let mut routes = local
                .into_iter()
                .map(|(_, _, a)| Some(a))
                .collect::<Vec<_>>();
            if settings.interface.is_none() {
                routes.push(None);
            }
            for route in routes {
                let attempt = async {
                    let socket = if endpoint.is_ipv4() {
                        tokio::net::TcpSocket::new_v4()?
                    } else {
                        tokio::net::TcpSocket::new_v6()?
                    };
                    if let Some(route) = route {
                        socket.bind(route)?;
                    }
                    socket.connect(endpoint).await
                };
                if let Ok(Ok(stream)) = timeout(Duration::from_secs(2), attempt).await {
                    connected = Some(stream);
                    break 'endpoints;
                }
            }
        }
        let mut stream = connected
            .ok_or("Device unreachable. Check Wi-Fi/Ethernet, firewall, and VPN LAN access")?;
        stream.set_nodelay(true)?;
        let peer = stream.peer_addr()?;
        protocol::write(
            &mut stream,
            &Hello {
                version: PROTOCOL_VERSION,
                name: self.store.settings().device_name,
                certificate: hex::encode(&self.store.identity.certificate),
                pair: true,
            },
        )
        .await?;
        let response: PairMessage = self.read_pairing(&mut stream, 125).await?;
        let (cert, new_pair) = match response {
            PairMessage::Known {
                certificate,
                name: _,
            } => {
                let bytes = hex::decode(certificate)?;
                let id = hex::encode(Sha256::digest(&bytes));
                if !self
                    .store
                    .trusted()
                    .iter()
                    .any(|d| d.device.id == id && d.certificate == hex::encode(&bytes))
                {
                    return Err("Unknown or changed identity. Forget the pairing on both computers and pair again".into());
                }
                (bytes, false)
            }
            PairMessage::Ready {
                public,
                certificate,
                name,
            } => {
                if public.len() > 1024 || certificate.len() > 16000 || name.len() > 80 {
                    return Err("Invalid pairing response".into());
                }
                let code = zeroize::Zeroizing::new(
                    self.prompt(name.clone(), peer.to_string(), true).await?,
                );
                let cert = hex::decode(certificate)?;
                let (exchange, our_public) = Exchange::start(&code, true)?;
                let their_public = hex::decode(public)?;
                let key = exchange.finish(&their_public)?;
                let transcript = pairing::transcript(
                    &our_public,
                    &their_public,
                    &self.store.identity.certificate,
                    &cert,
                );
                protocol::write(
                    &mut stream,
                    &PairMessage::Proof {
                        public: hex::encode(our_public),
                        proof: hex::encode(pairing::proof(&key, &transcript, b"controller")),
                    },
                )
                .await?;
                match timeout(
                    Duration::from_secs(5),
                    protocol::read::<PairMessage>(&mut stream),
                )
                .await??
                {
                    PairMessage::Confirm { proof } => {
                        pairing::verify(&key, &transcript, b"receiver", &hex::decode(proof)?)?
                    }
                    PairMessage::Rejected { reason } => return Err(reason.into()),
                    _ => return Err("Unexpected pairing message".into()),
                };
                (cert, true)
            }
            PairMessage::Rejected { reason } => return Err(reason.into()),
            _ => return Err("Unexpected handshake message".into()),
        };
        let local_ip = stream.local_addr()?.ip().to_string();
        let id = hex::encode(Sha256::digest(&cert));
        let connector = tokio_rustls::TlsConnector::from(transport::client(
            &self.store.identity,
            cert.clone(),
        )?);
        let tls = timeout(
            Duration::from_secs(5),
            connector.connect(
                rustls::pki_types::ServerName::try_from("flowlink.local")?,
                stream,
            ),
        )
        .await??;
        self.session(
            SecureStream::Client(tls),
            Peer {
                id,
                certificate: cert,
                address: peer,
                new_pair,
                controller: true,
                local_ip,
            },
        )
        .await
    }
    #[cfg(target_os = "macos")]
    async fn listen(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(("0.0.0.0", PORT)).await?;
        let listener6 = TcpListener::bind(("::", PORT)).await.ok();
        loop {
            let (mut stream, peer) = tokio::select! {result=listener.accept()=>result?,result=async{match &listener6{Some(listener)=>listener.accept().await,None=>std::future::pending().await}}=>result?};
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }
            let local = stream.local_addr()?;
            let settings = self.store.settings();
            let allowed = discovery::is_lan(peer.ip())
                && discovery::is_lan(local.ip())
                && settings.interface.as_ref().is_none_or(|name| {
                    discovery::addresses()
                        .iter()
                        .any(|a| &a.interface == name && a.ip == local.ip().to_string())
                });
            if !allowed {
                continue;
            }
            if self
                .busy
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                tokio::spawn(async move {
                    let _ = timeout(Duration::from_secs(1), async {
                        let _ = protocol::read::<Hello>(&mut stream).await;
                        protocol::write(
                            &mut stream,
                            &PairMessage::Rejected {
                                reason: "Device busy".into(),
                            },
                        )
                        .await
                    })
                    .await;
                });
                continue;
            }
            let engine = self.clone();
            tokio::spawn(async move {
                let result = engine.clone().accept_inner(stream, peer).await;
                engine.finish_connection(result);
            });
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    async fn accept_inner(self: Arc<Self>, mut stream: TcpStream, peer: SocketAddr) -> Result<()> {
        stream.set_nodelay(true)?;
        let hello: Hello = timeout(Duration::from_secs(4), protocol::read(&mut stream)).await??;
        hello.validate()?;
        let cert = hex::decode(&hello.certificate)?;
        let id = hex::encode(Sha256::digest(&cert));
        let trusted = self
            .store
            .trusted()
            .iter()
            .any(|d| d.device.id == id && d.certificate == hello.certificate);
        if self.store.settings().paused {
            protocol::write(
                &mut stream,
                &PairMessage::Rejected {
                    reason: "Sharing is paused on the Mac".into(),
                },
            )
            .await?;
            return Ok(());
        }
        if trusted {
            protocol::write(
                &mut stream,
                &PairMessage::Known {
                    certificate: hex::encode(&self.store.identity.certificate),
                    name: self.store.settings().device_name,
                },
            )
            .await?;
        } else {
            if !self
                .limiter
                .lock()
                .map_err(|_| "Pairing unavailable")?
                .allowed()
            {
                protocol::write(
                    &mut stream,
                    &PairMessage::Rejected {
                        reason: "Too many pairing attempts. Wait two minutes".into(),
                    },
                )
                .await?;
                return Ok(());
            }
            self.limiter
                .lock()
                .map_err(|_| "Pairing unavailable")?
                .failed();
            if self
                .prompt(hello.name.clone(), peer.to_string(), false)
                .await?
                != "approve"
            {
                return Err("Pairing declined".into());
            }
            let code = zeroize::Zeroizing::new(format!(
                "{:06}",
                rand::thread_rng().gen_range(0..1_000_000u32)
            ));
            self.update(|s| {
                if let Some(p) = &mut s.pairing {
                    p.code = Some(code.to_string());
                    p.expires_at = discovery::now() + 120;
                }
            });
            let (exchange, our_public) = Exchange::start(&code, false)?;
            protocol::write(
                &mut stream,
                &PairMessage::Ready {
                    public: hex::encode(&our_public),
                    certificate: hex::encode(&self.store.identity.certificate),
                    name: self.store.settings().device_name,
                },
            )
            .await?;
            let response: PairMessage = self.read_pairing(&mut stream, 120).await?;
            match response {
                PairMessage::Proof { public, proof } => {
                    if public.len() > 1024 || proof.len() > 128 {
                        return Err("Invalid pairing proof".into());
                    }
                    let client_public = hex::decode(public)?;
                    let key = exchange.finish(&client_public)?;
                    let transcript = pairing::transcript(
                        &client_public,
                        &our_public,
                        &cert,
                        &self.store.identity.certificate,
                    );
                    if pairing::verify(&key, &transcript, b"controller", &hex::decode(proof)?)
                        .is_err()
                    {
                        protocol::write(&mut stream,&PairMessage::Rejected{reason:"Pairing code did not match. Approve a new pairing attempt on the Mac".into()}).await?;
                        return Err("Pairing code did not match".into());
                    }
                    protocol::write(
                        &mut stream,
                        &PairMessage::Confirm {
                            proof: hex::encode(pairing::proof(&key, &transcript, b"receiver")),
                        },
                    )
                    .await?;
                }
                _ => return Err("Unexpected pairing message".into()),
            }
        }
        let local_ip = stream.local_addr()?.ip().to_string();
        let acceptor =
            tokio_rustls::TlsAcceptor::from(transport::server(&self.store.identity, cert.clone())?);
        let tls = timeout(Duration::from_secs(5), acceptor.accept(stream)).await??;
        self.session(
            SecureStream::Server(tls),
            Peer {
                id,
                certificate: cert,
                address: peer,
                new_pair: !trusted,
                controller: false,
                local_ip,
            },
        )
        .await
    }
    async fn session(self: Arc<Self>, mut stream: SecureStream, peer_info: Peer) -> Result<()> {
        let Peer {
            id,
            certificate,
            address: peer,
            new_pair,
            controller,
            local_ip,
        } = peer_info;
        let local_displays = platform::displays(&self.store.identity.id)?;
        protocol::write(
            &mut stream,
            &Message::Welcome {
                device_id: self.store.identity.id.clone(),
                name: self.store.settings().device_name,
                displays: local_displays.clone(),
            },
        )
        .await?;
        let response: Message =
            timeout(Duration::from_secs(4), protocol::read(&mut stream)).await??;
        let (name, remote_displays) = match response {
            Message::Welcome {
                device_id,
                displays,
                name,
            } if device_id == id
                && !name.is_empty()
                && name.len() <= 80
                && !name.chars().any(char::is_control)
                && !displays.is_empty()
                && displays.len() <= 32
                && displays.iter().all(|d| {
                    d.device_id == id
                        && d.rect.valid()
                        && d.id.len() < 256
                        && d.scale.is_finite()
                        && d.scale > 0.0
                }) =>
            {
                (name, displays)
            }
            _ => return Err("Invalid peer display inventory".into()),
        };
        if controller && remote_displays.len() != 1 {
            return Err(
                "This release supports one Mac display. Disconnect additional Mac displays".into(),
            );
        }
        if new_pair {
            self.store.trust(TrustedDevice {
                device: Device {
                    id: id.clone(),
                    name: name.clone(),
                    os: if controller { "macos" } else { "windows" }.into(),
                    addresses: vec![peer.ip().to_string()],
                    port: PORT,
                    online: true,
                    busy: false,
                    trusted: true,
                    favorite: false,
                    auto_connect: true,
                    last_seen: discovery::now(),
                },
                certificate: hex::encode(certificate),
                placements: Vec::new(),
                last_local_ip: Some(local_ip.clone()),
                host_display_ids: Vec::new(),
            })?;
        } else {
            let mut config = self
                .store
                .config
                .lock()
                .map_err(|_| "Settings unavailable")?;
            if let Some(d) = config.trusted.iter_mut().find(|d| d.device.id == id) {
                if controller {
                    d.device.addresses = vec![peer.ip().to_string()];
                    d.last_local_ip = Some(local_ip.clone());
                }
                d.device.name = name.clone();
                d.device.last_seen = discovery::now();
            }
            drop(config);
            self.store.save()?;
        }
        let (tx, mut outgoing) = mpsc::channel::<Message>(512);
        *self
            .connection
            .lock()
            .map_err(|_| "Connection unavailable")? = Some(tx.clone());
        let mut all = local_displays.clone();
        all.extend(remote_displays.clone());
        self.update(|s| {
            s.connected_id = Some(id.clone());
            s.peer_paused = false;
            s.pairing = None;
            s.error = None;
            s.displays = all;
            s.status = format!("Connected to {name}");
        });
        let saved = self
            .store
            .trusted()
            .into_iter()
            .find(|t| t.device.id == id)
            .ok_or("Trust was revoked")?;
        self.discovered(saved.device.clone());
        self.ownership(if self.store.settings().paused {
            Ownership::Paused
        } else {
            Ownership::Local
        });
        if controller {
            tx.try_send(Message::Configuration {
                settings: InputSettings::from_settings(&self.store.settings()),
            })?;
            let placements = if geometry::validate_layout(&local_displays, &saved.placements)
                .is_ok()
                && saved.host_display_ids.len() == local_displays.len()
                && local_displays
                    .iter()
                    .all(|d| saved.host_display_ids.contains(&d.id))
                && saved.placements.first().is_some_and(|p| {
                    p.display_id == remote_displays[0].id
                        && (p.rect.width - remote_displays[0].rect.width).abs() < 1.0
                        && (p.rect.height - remote_displays[0].rect.height).abs() < 1.0
                }) {
                saved.placements
            } else {
                Vec::new()
            };
            if !placements.is_empty() {
                self.update(|s| s.placements = placements.clone());
                tx.try_send(Message::Layout {
                    placements,
                    host_displays: local_displays,
                })?;
            } else {
                self.update(|s| {
                    s.status = "Connected · arrange displays to enable edge switching".into()
                });
            }
        }
        let (read, write) = tokio::io::split(stream);
        let (incoming_tx, mut incoming) = mpsc::channel(512);
        let reader = tokio::spawn(async move {
            let mut read = read;
            loop {
                let message = protocol::read::<Message>(&mut read).await;
                let failed = message.is_err();
                if incoming_tx.send(message).await.is_err() || failed {
                    break;
                }
            }
        });
        let writer = tokio::spawn(async move {
            let mut write = write;
            while let Some(message) = outgoing.recv().await {
                let bye = matches!(message, Message::Goodbye);
                timeout(
                    Duration::from_secs(1),
                    protocol::write(&mut write, &message),
                )
                .await??;
                if bye {
                    break;
                }
            }
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        });
        let mut tick = tokio::time::interval(Duration::from_millis(250));
        let mut last = Instant::now();
        let started = Instant::now();
        let mut expected_sequence = 0;
        let mut rate_count = 0u32;
        let mut rate_start = Instant::now();
        let result:Result<()>=async {loop {tokio::select! {
            _=tick.tick()=>{if self.shutdown.load(Ordering::SeqCst)||self.snapshot().connected_id.as_deref()!=Some(&id){break;}
if last.elapsed()>Duration::from_secs(1){return Err("Connection timed out; input returned locally".into());}
if !self.store.trusted().iter().any(|t|t.device.id==id){return Err("Device trust revoked".into());}tx.try_send(Message::Heartbeat{nonce:started.elapsed().as_micros() as u64})?;},
            message=incoming.recv()=>{
                let message=message.ok_or("Connection closed")??;last=Instant::now();self.native.renew();rate_count+=1;if rate_start.elapsed()>=Duration::from_secs(1){rate_start=Instant::now();rate_count=0;}
if rate_count>16000{return Err("Peer message rate exceeded".into());}
                if !self.process_message(message,controller,&id,&tx,started,&mut expected_sequence)? { break; }
            }
        }}Ok(())}.await;
        self.native.release();
        reader.abort();
        writer.abort();
        let _ = reader.await;
        let _ = writer.await;
        result
    }
    fn process_message(
        &self,
        message: Message,
        controller: bool,
        id: &str,
        tx: &mpsc::Sender<Message>,
        started: Instant,
        expected_sequence: &mut u64,
    ) -> Result<bool> {
        match message {
            Message::Heartbeat { nonce } => {
                tx.try_send(Message::Pong { nonce })?;
            }
            Message::Pong { nonce } => {
                let current = started.elapsed().as_micros() as u64;
                if current >= nonce {
                    self.update(|s| s.latency_ms = Some((current - nonce) as f64 / 1000.0));
                }
            }
            Message::Goodbye => return Ok(false),
            Message::Identify => {
                self.identify_requested.store(true, Ordering::SeqCst);
            }
            Message::Pause => {
                self.native.release();
                self.update(|s| s.peer_paused = true);
                self.ownership(Ownership::Paused);
            }
            Message::Resume => {
                self.update(|s| s.peer_paused = false);
                if !self.store.settings().paused {
                    self.ownership(Ownership::Local);
                }
            }
            Message::Release => {
                self.native.release();
                self.ownership(if self.store.settings().paused {
                    Ownership::Paused
                } else {
                    Ownership::Local
                });
            }
            Message::Layout {
                placements,
                host_displays,
            } if !controller => {
                self.native.release();
                self.ownership(Ownership::Local);
                if host_displays.len() > 32
                    || host_displays
                        .iter()
                        .any(|d| d.device_id != id || !d.rect.valid())
                {
                    return Err("Invalid host topology".into());
                }
                geometry::validate_layout(&host_displays, &placements)?;
                let mut returns = Vec::new();
                for edge in geometry::edges(&host_displays, &placements) {
                    let p = placements
                        .iter()
                        .find(|p| p.display_id == edge.remote_id)
                        .ok_or("Unknown target display")?;
                    let actual = platform::displays(&self.store.identity.id)?
                        .into_iter()
                        .find(|d| d.id == p.display_id)
                        .ok_or("Mac display changed")?;
                    returns.push((edge, p.rect.clone(), actual.rect));
                }
                *self
                    .return_edges
                    .lock()
                    .map_err(|_| "Topology unavailable")? = returns;
                self.update(|s| s.placements = placements);
            }
            Message::Configuration { settings } if !controller => {
                let mut config = self
                    .store
                    .config
                    .lock()
                    .map_err(|_| "Settings unavailable")?;
                settings.apply(&mut config.settings)?;
                let settings = config.settings.clone();
                drop(config);
                self.store.save()?;
                self.update(|s| s.settings = settings);
            }
            Message::Handoff { display_id, x, y } if !controller => {
                if self.store.settings().paused || !x.is_finite() || !y.is_finite() {
                    return Err("Invalid or paused handoff".into());
                }
                let display = platform::displays(&self.store.identity.id)?
                    .into_iter()
                    .find(|d| d.id == display_id)
                    .ok_or("Display no longer available")?;
                if x < display.rect.x
                    || x >= display.rect.right()
                    || y < display.rect.y
                    || y >= display.rect.bottom()
                {
                    return Err("Handoff outside display".into());
                }
                self.native.warp(x, y);
                self.native
                    .lease(!self.store.settings().allow_local_input)?;
                self.ownership(Ownership::Remote);
                tx.try_send(Message::HandoffAck)?;
            }
            Message::HandoffAck if controller => {
                if self.snapshot().ownership != Ownership::Requesting {
                    return Err("Unexpected handoff acknowledgement".into());
                }
                if !self
                    .safety
                    .lock()
                    .map_err(|_| "Input state unavailable")?
                    .can_handoff()
                {
                    self.return_local();
                } else {
                    let (x, y) = self.native.cursor();
                    let snapshot = self.snapshot();
                    let anchor = snapshot
                        .displays
                        .iter()
                        .filter(|d| d.device_id == snapshot.device_id)
                        .find(|d| {
                            x >= d.rect.x
                                && x < d.rect.right()
                                && y >= d.rect.y
                                && y < d.rect.bottom()
                        });
                    if let Some(display) = anchor {
                        self.native.warp(
                            display.rect.x + display.rect.width / 2.0,
                            display.rect.y + display.rect.height / 2.0,
                        );
                    }
                    self.native.lease(false)?;
                    self.ownership(Ownership::Remote);
                }
            }
            Message::Input { sequence, event } if !controller => {
                if sequence <= *expected_sequence || !event.valid() {
                    return Err("Invalid input sequence".into());
                }
                *expected_sequence = sequence;
                if self
                    .state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .ownership
                    != Ownership::Remote
                {
                    return Ok(true);
                }
                let config = self
                    .store
                    .config
                    .lock()
                    .map_err(|_| "Settings unavailable")?;
                self.native.inject(&event, &config.settings)?;
                drop(config);
                self.safety
                    .lock()
                    .map_err(|_| "Input state unavailable")?
                    .track(&event);
            }
            Message::Return { x, y } if controller => {
                if !x.is_finite() || !y.is_finite() {
                    return Err("Invalid return coordinates".into());
                }
                let snapshot = self.snapshot();
                if !snapshot
                    .displays
                    .iter()
                    .filter(|d| d.device_id == snapshot.device_id)
                    .any(|d| {
                        x >= d.rect.x && x < d.rect.right() && y >= d.rect.y && y < d.rect.bottom()
                    })
                {
                    return Err("Return display unavailable".into());
                }
                self.native.release();
                self.native.warp(x, y);
                self.ownership(Ownership::Local);
                *self
                    .edge_since
                    .lock()
                    .map_err(|_| "Edge state unavailable")? =
                    Some(("cooldown".into(), Instant::now()));
                tx.try_send(Message::ReturnAck)?;
            }
            Message::ReturnAck if !controller => {
                self.native.release();
                self.ownership(Ownership::Local);
            }
            _ => return Err("Unexpected message for this device role".into()),
        }
        Ok(true)
    }
    fn native_event(&self, event: NativeEvent) {
        match event {
            NativeEvent::Switch => {
                if self
                    .state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .ownership
                    == Ownership::Remote
                {
                    self.return_local();
                } else {
                    self.switch_requested.store(true, Ordering::SeqCst);
                }
            }
            NativeEvent::Emergency => {
                self.native.release();
                let _ = self.pause(true);
            }
            NativeEvent::Failure(message) => {
                self.return_local();
                self.error(message);
            }
            NativeEvent::Input(input) => {
                let state = self
                    .state
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .ownership
                    .clone();
                self.safety
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .track(&input);
                if self.switch_requested.load(Ordering::SeqCst)
                    && self
                        .safety
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .can_handoff()
                {
                    self.switch_requested.store(false, Ordering::SeqCst);
                    if let Err(e) = self.switch_control() {
                        self.error(e.to_string());
                    }
                }
                if cfg!(target_os = "windows") && state == Ownership::Remote {
                    let sequence = self.sequence.fetch_add(1, Ordering::SeqCst) + 1;
                    if let Err(error) = self.send(Message::Input {
                        sequence,
                        event: input,
                    }) {
                        self.return_local();
                        self.error(error.to_string());
                    }
                }
            }
            NativeEvent::Position(x, y) => {
                if cfg!(target_os = "windows") {
                    self.host_position(x, y);
                } else {
                    self.mac_position(x, y);
                }
            }
        }
    }
    fn host_position(&self, x: f64, y: f64) {
        let s = self.snapshot();
        if s.ownership != Ownership::Local
            || s.settings.paused
            || s.peer_paused
            || s.settings.hotkey_only
            || s.connected_id.is_none()
        {
            return;
        }
        if !self
            .safety
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .can_handoff()
        {
            return;
        }
        let local = s
            .displays
            .iter()
            .filter(|d| d.device_id == s.device_id)
            .cloned()
            .collect::<Vec<_>>();
        let hit = geometry::edges(&local, &s.placements)
            .into_iter()
            .find(|e| e.hit(x, y));
        let mut since = self.edge_since.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((id, time)) = &*since {
            if id == "cooldown" && time.elapsed() < Duration::from_millis(350) {
                return;
            }
        }
        let Some(edge) = hit else {
            *since = None;
            return;
        };
        let edge_id = format!("{}:{:?}", edge.local_id, edge.side);
        if since.as_ref().is_none_or(|(id, _)| id != &edge_id) {
            *since = Some((edge_id, Instant::now()));
        }
        if since.as_ref().is_some_and(|(_, t)| {
            t.elapsed() < Duration::from_millis(s.settings.edge_resistance_ms)
        }) {
            return;
        }
        let Some(placement) = s.placements.iter().find(|p| p.display_id == edge.remote_id) else {
            return;
        };
        let Some(target) = s
            .displays
            .iter()
            .find(|d| d.id == edge.remote_id && d.device_id != s.device_id)
        else {
            return;
        };
        let (tx, ty) = edge.entry(edge.fraction(x, y), &placement.rect, &target.rect);
        self.ownership(Ownership::Requesting);
        if let Err(e) = self.send(Message::Handoff {
            display_id: target.id.clone(),
            x: tx,
            y: ty,
        }) {
            self.return_local();
            self.error(e.to_string());
        }
        *since = None;
    }
    fn mac_position(&self, x: f64, y: f64) {
        if self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .ownership
            != Ownership::Remote
            || !self
                .safety
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .can_handoff()
        {
            return;
        }
        let edges = self.return_edges.lock().unwrap_or_else(|e| e.into_inner());
        for (edge, virtual_, actual) in edges.iter() {
            let vx = virtual_.x + (x - actual.x) / actual.width * virtual_.width;
            let vy = virtual_.y + (y - actual.y) / actual.height * virtual_.height;
            let crossed = match edge.side {
                Side::Right => x <= actual.x + 1.0,
                Side::Left => x >= actual.right() - 2.0,
                Side::Bottom => y <= actual.y + 1.0,
                Side::Top => y >= actual.bottom() - 2.0,
            };
            let along = match edge.side {
                Side::Left | Side::Right => vy,
                _ => vx,
            };
            if crossed && along > edge.start + 2.0 && along < edge.end - 2.0 {
                let (rx, ry) = match edge.side {
                    Side::Right => (edge.boundary - 4.0, vy),
                    Side::Left => (edge.boundary + 3.0, vy),
                    Side::Bottom => (vx, edge.boundary - 4.0),
                    Side::Top => (vx, edge.boundary + 3.0),
                };
                self.native.release();
                self.ownership(Ownership::Returning);
                let _ = self.send(Message::Return { x: rx, y: ry });
                break;
            }
        }
    }
    async fn maintenance(self: Arc<Self>) {
        let mut tick = tokio::time::interval(Duration::from_secs(3));
        let mut retry = Instant::now() - Duration::from_secs(5);
        let mut retry_delay = Duration::from_secs(2);
        let mut previous_displays = String::new();
        loop {
            tick.tick().await;
            if self.shutdown.load(Ordering::SeqCst) {
                break;
            }
            let mut permissions = platform::permissions();
            permissions.filtering &= self.native.gate.healthy.load(Ordering::SeqCst);
            if (!permissions.accessibility || !permissions.monitoring) && self.native.gate.active()
            {
                self.return_local();
                self.error("Mac input permission was revoked".into());
            }
            let displays = platform::displays(&self.store.identity.id).unwrap_or_default();
            let topology = serde_json::to_string(&displays).unwrap_or_default();
            if !previous_displays.is_empty() && previous_displays != topology {
                self.return_local();
                self.update(|s| {
                    s.placements.clear();
                    s.status = "Displays changed · review arrangement".into();
                });
            }
            previous_displays = topology;
            self.update(|s| {
                s.permissions = permissions;
                s.addresses = discovery::addresses();
                s.displays.retain(|d| d.device_id != s.device_id);
                s.displays.extend(displays);
            });
            if self.snapshot().connected_id.is_some() {
                retry_delay = Duration::from_secs(2);
            }
            if cfg!(target_os = "windows")
                && !self.busy.load(Ordering::SeqCst)
                && !self.reconnect_suspended.load(Ordering::SeqCst)
                && !self.store.settings().paused
                && retry.elapsed() > retry_delay
            {
                retry = Instant::now();
                retry_delay = (retry_delay * 2).min(Duration::from_secs(30));
                let mut devices = self.store.trusted();
                devices
                    .sort_by_key(|d| (!d.device.favorite, std::cmp::Reverse(d.device.last_seen)));
                if let Some(device) = devices.into_iter().find(|d| d.device.auto_connect) {
                    let live = self
                        .snapshot()
                        .devices
                        .into_iter()
                        .find(|d| d.id == device.device.id && d.online)
                        .unwrap_or(device.device);
                    if let Some(address) = live.addresses.first() {
                        let _ = self.connect(if address.contains(':') {
                            format!("[{}]:{}", address, live.port)
                        } else {
                            format!("{}:{}", address, live.port)
                        });
                    }
                }
            }
        }
    }
}
pub fn normalize_address(address: &str) -> Result<String> {
    let address = address.trim();
    if address.is_empty()
        || address.len() > 253
        || address
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '/' | '\\' | '@' | '?' | '#'))
    {
        return Err("Enter a local IP address or hostname, optionally followed by :port".into());
    }
    if let Ok(ip) = address.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, PORT).to_string());
    }
    if address.contains(':') {
        if let Ok(socket) = address.parse::<SocketAddr>() {
            return Ok(socket.to_string());
        }
        let (host, port) = address.rsplit_once(':').ok_or("Invalid address")?;
        if host.is_empty() || port.parse::<u16>().ok().filter(|p| *p > 0).is_none() {
            return Err("Invalid connection port".into());
        }
        return Ok(address.into());
    }
    Ok(format!("{address}:{PORT}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_addresses() {
        assert_eq!(
            normalize_address("192.168.1.4").unwrap(),
            "192.168.1.4:47831"
        );
        assert_eq!(
            normalize_address("macbook.local:5000").unwrap(),
            "macbook.local:5000"
        );
        assert_eq!(normalize_address("fd00::1").unwrap(), "[fd00::1]:47831");
        assert!(normalize_address("https://host").is_err());
        assert!(normalize_address("host:0").is_err());
    }
}
