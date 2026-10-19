#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use flowlink_engine::{
    model::{Placement, Settings, Snapshot},
    Engine,
};
use std::sync::{atomic::Ordering, Arc};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};
use tauri_plugin_autostart::ManagerExt;
struct AppState(Arc<Engine>);
type CommandResult<T> = std::result::Result<T, String>;
#[tauri::command]
fn snapshot(state: tauri::State<AppState>) -> Snapshot {
    state.0.snapshot()
}
#[tauri::command]
fn diagnostics(state: tauri::State<AppState>) -> CommandResult<String> {
    state.0.diagnostics().map_err(|e| e.to_string())
}
#[tauri::command]
fn connect(state: tauri::State<AppState>, address: String) -> CommandResult<()> {
    state.0.connect(address).map_err(|e| e.to_string())
}
#[tauri::command]
fn disconnect(state: tauri::State<AppState>) {
    state.0.disconnect();
}
#[tauri::command]
fn pause(state: tauri::State<AppState>, paused: bool) -> CommandResult<()> {
    state.0.pause(paused).map_err(|e| e.to_string())
}
#[tauri::command]
fn return_local(state: tauri::State<AppState>) {
    state.0.return_local();
}
#[tauri::command]
fn switch_control(state: tauri::State<AppState>) -> CommandResult<()> {
    state.0.switch_control().map_err(|e| e.to_string())
}
#[tauri::command]
fn save_settings(state: tauri::State<AppState>, settings: Settings) -> CommandResult<()> {
    state.0.save_settings(settings).map_err(|e| e.to_string())
}
#[tauri::command]
fn save_layout(state: tauri::State<AppState>, placements: Vec<Placement>) -> CommandResult<()> {
    state.0.save_layout(placements).map_err(|e| e.to_string())
}
#[tauri::command]
fn cancel_pairing(state: tauri::State<AppState>) {
    state.0.cancel_pairing();
}
#[tauri::command]
fn answer_pairing(state: tauri::State<AppState>, id: String, answer: String) -> CommandResult<()> {
    state
        .0
        .answer_pairing(&id, answer)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn set_device(
    state: tauri::State<AppState>,
    id: String,
    favorite: bool,
    auto_connect: bool,
) -> CommandResult<()> {
    state
        .0
        .set_device(&id, favorite, auto_connect)
        .map_err(|e| e.to_string())
}
#[tauri::command]
fn forget_device(state: tauri::State<AppState>, id: String) -> CommandResult<()> {
    state.0.forget(&id).map_err(|e| e.to_string())
}
#[tauri::command]
fn copy_address(state: tauri::State<AppState>, ip: String, with_port: bool) -> CommandResult<()> {
    let s = state.0.snapshot();
    if !s.addresses.iter().any(|a| a.ip == ip) {
        return Err("Address is no longer assigned to this computer".into());
    }
    let text = if with_port {
        if ip.contains('%') {
            format!("[{}]:{}", ip, s.port)
        } else {
            match ip.parse::<std::net::IpAddr>() {
                Ok(ip) => std::net::SocketAddr::new(ip, s.port).to_string(),
                Err(_) => return Err("Invalid address".into()),
            }
        }
    } else {
        ip
    };
    flowlink_engine::platform::copy_text(&text).map_err(|e| e.to_string())
}
#[tauri::command]
fn request_permissions() {
    flowlink_engine::platform::request_permissions();
}
#[tauri::command]
fn identify(state: tauri::State<AppState>) {
    state.0.identify();
}
#[tauri::command]
fn open_main(app: tauri::AppHandle, page: String) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("navigate", page);
    }
    if let Some(w) = app.get_webview_window("quick") {
        let _ = w.hide();
    }
}
#[tauri::command]
fn autostart_enabled(app: tauri::AppHandle) -> CommandResult<bool> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}
#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enabled: bool) -> CommandResult<()> {
    if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    }
    .map_err(|e| e.to_string())
}
fn show_identifiers(app: &tauri::AppHandle, engine: &Engine) {
    for display in engine
        .snapshot()
        .displays
        .into_iter()
        .filter(|d| d.device_id == engine.store.identity.id)
    {
        let label = format!("identify-{}", display.number);
        if app.get_webview_window(&label).is_some() {
            continue;
        }
        if let Ok(w) = tauri::WebviewWindowBuilder::new(
            app,
            &label,
            tauri::WebviewUrl::App(format!("identify.html?number={}", display.number).into()),
        )
        .title("Identify display")
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .inner_size(128.0, 128.0)
        .position(
            display.rect.x + display.rect.width / 2.0 - 64.0,
            display.rect.y + display.rect.height / 2.0 - 64.0,
        )
        .build()
        {
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(3));
                let _ = w.close();
            });
        }
    }
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            open_main(app.clone(), "devices".into())
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
        .setup(|app| {
            let directory = app.path().app_config_dir()?;
            let engine = tauri::async_runtime::block_on(async { Engine::start(&directory) })
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            app.manage(AppState(engine.clone()));
            if !std::env::args().any(|arg| arg == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }
            let open = MenuItem::with_id(app, "open", "Open FlowLink", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause / Resume", true, None::<&str>)?;
            let release =
                MenuItem::with_id(app, "release", "Return to Windows", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit FlowLink", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &pause, &release, &quit])?;
            let mut rgba = vec![0u8; 32 * 32 * 4];
            for y in 5..27 {
                for x in 7..25 {
                    if x < 11 || y < 9 || (14..18).contains(&y) && x < 22 {
                        let index = (y * 32 + x) * 4;
                        rgba[index..index + 4].copy_from_slice(&[57, 112, 242, 255]);
                    }
                }
            }
            let tray = TrayIconBuilder::with_id("flowlink")
                .icon(tauri::image::Image::new_owned(rgba, 32, 32))
                .tooltip("FlowLink · local control")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    let state = app.state::<AppState>();
                    match event.id.as_ref() {
                        "open" => open_main(app.clone(), "devices".into()),
                        "pause" => {
                            let _ = state.0.pause(!state.0.snapshot().settings.paused);
                        }
                        "release" => state.0.return_local(),
                        "quit" => {
                            state.0.shutdown();
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        position,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("quick") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                if let Ok(Some(monitor)) =
                                    w.monitor_from_point(position.x, position.y)
                                {
                                    let area = monitor.work_area();
                                    let scale = monitor.scale_factor();
                                    let width = 370.0 * scale;
                                    let height = 540.0 * scale;
                                    let x = (position.x - width / 2.0).clamp(
                                        area.position.x as f64,
                                        (area.position.x as f64 + area.size.width as f64 - width)
                                            .max(area.position.x as f64),
                                    );
                                    let y = if position.y
                                        > area.position.y as f64 + area.size.height as f64 / 2.0
                                    {
                                        position.y - height - 12.0
                                    } else {
                                        position.y + 12.0
                                    };
                                    let y = y.clamp(
                                        area.position.y as f64,
                                        (area.position.y as f64 + area.size.height as f64 - height)
                                            .max(area.position.y as f64),
                                    );
                                    let _ = w.set_position(tauri::PhysicalPosition::new(
                                        x as i32, y as i32,
                                    ));
                                }
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(app)?;
            let handle = app.handle().clone();
            std::thread::Builder::new()
                .name("flowlink-ui-state".into())
                .spawn(move || {
                    let mut previous = String::new();
                    let mut pairing_id = String::new();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        let state = engine.snapshot();
                        if let Some(pairing) = &state.pairing {
                            if pairing_id != pairing.request_id {
                                pairing_id = pairing.request_id.clone();
                                if let Some(window) = handle.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        } else {
                            pairing_id.clear();
                        }
                        let label = match state.ownership {
                            flowlink_engine::model::Ownership::Remote => {
                                "FlowLink · controlling Mac"
                            }
                            flowlink_engine::model::Ownership::Paused => "FlowLink · paused",
                            _ => "FlowLink · local control",
                        };
                        let _ = tray.set_tooltip(Some(label));
                        if engine.identify_requested.swap(false, Ordering::SeqCst) {
                            show_identifiers(&handle, &engine);
                        }
                        if handle
                            .webview_windows()
                            .values()
                            .any(|w| w.is_visible().unwrap_or(false))
                        {
                            if let Ok(serialized) = serde_json::to_string(&state) {
                                if serialized != previous {
                                    previous = serialized;
                                    let _ = handle.emit("engine-state", &state);
                                }
                            }
                        }
                    }
                })?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. }
                if window.label() == "main" || window.label() == "quick" =>
            {
                api.prevent_close();
                let _ = window.hide();
            }
            tauri::WindowEvent::Focused(false) if window.label() == "quick" => {
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            diagnostics,
            connect,
            disconnect,
            pause,
            return_local,
            switch_control,
            save_settings,
            save_layout,
            answer_pairing,
            cancel_pairing,
            set_device,
            forget_device,
            copy_address,
            request_permissions,
            identify,
            open_main,
            autostart_enabled,
            set_autostart
        ])
        .build(tauri::generate_context!())
        .expect(
            "FlowLink could not initialize. Check protected credential storage and installation.",
        )
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                if let Some(state) = app.try_state::<AppState>() {
                    state.0.shutdown();
                }
            }
        });
}
