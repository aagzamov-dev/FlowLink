use super::*;
use crate::input::Input;

fn display(device: &str) -> Display {
    Display {
        id: format!("{device}-display"),
        device_id: device.into(),
        name: device.into(),
        number: 1,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 1000.0,
            height: 800.0,
        },
        pixel_width: 1000,
        pixel_height: 800,
        scale: 1.0,
        rotation: 0.0,
        online: true,
    }
}
fn fixture(controller: bool) -> (Arc<Engine>, mpsc::Receiver<Message>) {
    let engine = Engine::new(
        Arc::new(Store::temporary()),
        Native::for_test(),
        vec![display("local"), display("peer")],
    );
    let (tx, rx) = mpsc::channel(32);
    *engine.connection.lock().unwrap() = Some(tx);
    engine.update(|s| {
        s.connected_id = Some("peer".into());
        s.controller_id = Some(if controller { "local" } else { "peer" }.into());
    });
    *engine.token.lock().unwrap() = ControlToken {
        revision: 1,
        handoff: 1,
    };
    engine.initiator.store(true, Ordering::SeqCst);
    engine.ownership(Ownership::Local);
    (engine, rx)
}
fn remote(engine: &Engine) {
    engine.native.lease(false, engine.is_controller()).unwrap();
    engine.ownership(Ownership::Remote);
}
fn process(engine: &Engine, message: Message) {
    let tx = engine.connection.lock().unwrap().clone().unwrap();
    engine
        .process_message(message, true, "peer", &tx, Instant::now(), &mut 0)
        .unwrap();
}
fn edges(engine: &Engine) {
    *engine.return_edges.lock().unwrap() = vec![(
        geometry::Edge {
            local_id: "peer-display".into(),
            remote_id: "local-display".into(),
            side: Side::Right,
            start: 0.0,
            end: 800.0,
            boundary: 1000.0,
        },
        Rect {
            x: 1000.0,
            y: 0.0,
            width: 1000.0,
            height: 800.0,
        },
        display("local").rect,
    )];
}
#[test]
fn saving_settings_releases_capture_and_notifies_peer() {
    let (engine, mut rx) = fixture(true);
    remote(&engine);
    engine.save_settings(engine.store.settings()).unwrap();
    assert_eq!(engine.snapshot().ownership, Ownership::Local);
    assert!(!engine.native.gate.active());
    assert!(matches!(rx.try_recv().unwrap(), Message::Release { .. }));
}
#[test]
fn local_resume_preserves_peer_pause() {
    let (engine, _rx) = fixture(true);
    process(&engine, Message::Pause);
    engine.pause(true).unwrap();
    engine.pause(false).unwrap();
    assert_eq!(engine.snapshot().ownership, Ownership::Paused);
    assert!(!engine.snapshot().settings.paused);
    process(&engine, Message::Resume);
    assert_eq!(engine.snapshot().ownership, Ownership::Local);
}
#[test]
fn hotkey_only_disables_receiver_edge_return() {
    let (engine, mut rx) = fixture(false);
    remote(&engine);
    edges(&engine);
    engine.remote_settings.lock().unwrap().hotkey_only = true;
    engine.mac_position(0.0, 100.0);
    assert_eq!(engine.snapshot().ownership, Ownership::Remote);
    assert!(rx.try_recv().is_err());
}
#[test]
fn disconnect_clears_all_cached_topology() {
    let (engine, _rx) = fixture(false);
    edges(&engine);
    engine.disconnect();
    assert!(engine.return_edges.lock().unwrap().is_empty());
    assert!(engine.host_edges.lock().unwrap().is_empty());
    assert_eq!(engine.snapshot().ownership, Ownership::Disconnected);
}
#[test]
fn released_or_stale_handoff_ack_cannot_recapture_input() {
    let (engine, _rx) = fixture(true);
    let token = engine.current_token();
    engine.ownership(Ownership::Requesting);
    engine.return_local();
    process(&engine, Message::HandoffAck { token });
    assert!(!engine.native.gate.active());
    assert_eq!(engine.snapshot().ownership, Ownership::Local);
}
#[test]
fn old_owner_input_is_ignored_after_main_device_changes() {
    let (engine, _rx) = fixture(true);
    remote(&engine);
    let old = engine.current_token();
    engine.configure_control("peer".into(), true).unwrap();
    assert_eq!(engine.snapshot().controller_id.as_deref(), Some("peer"));
    assert!(!engine.native.gate.active());
    assert!(engine.current_token().revision > old.revision);
    process(
        &engine,
        Message::Input {
            token: old,
            sequence: 1,
            event: Input::Key {
                scan: 30,
                down: true,
            },
        },
    );
    assert!(engine.safety.lock().unwrap().keys.is_empty());
}
#[test]
fn only_current_main_can_change_restricted_control_policy() {
    let (engine, _rx) = fixture(true);
    engine.configure_control("local".into(), false).unwrap();
    process(
        &engine,
        Message::ControlRequest {
            controller_id: "peer".into(),
            from_either: true,
        },
    );
    assert!(engine.is_controller());
    assert!(!engine.snapshot().control_from_either);
    engine.configure_control("peer".into(), false).unwrap();
    assert!(engine.configure_control("local".into(), true).is_err());
}
#[test]
fn locally_delivered_queued_keys_are_not_forwarded_after_handoff() {
    let (engine, mut rx) = fixture(true);
    remote(&engine);
    engine.native_event(NativeEvent::Input {
        event: Input::Key {
            scan: 30,
            down: true,
        },
        captured: false,
        generation: engine.native.gate.generation.load(Ordering::SeqCst),
    });
    assert!(rx.try_recv().is_err());
}
#[test]
fn motion_coalescing_stops_at_buttons_and_control_tokens() {
    let token = ControlToken {
        revision: 1,
        handoff: 1,
    };
    let mut current = Message::Input {
        token,
        sequence: 1,
        event: Input::Motion { dx: 1.0, dy: 2.0 },
    };
    assert!(coalesce_message(
        &mut current,
        &Message::Input {
            token,
            sequence: 2,
            event: Input::Motion { dx: 3.0, dy: 4.0 }
        },
        token
    ));
    assert!(matches!(
        current,
        Message::Input {
            sequence: 2,
            event: Input::Motion { dx: 4.0, dy: 6.0 },
            ..
        }
    ));
    assert!(!coalesce_message(
        &mut current,
        &Message::Input {
            token,
            sequence: 3,
            event: Input::Button {
                button: 0,
                down: true
            }
        },
        token
    ));
    assert!(!coalesce_message(
        &mut current,
        &Message::Input {
            token: ControlToken {
                revision: 2,
                handoff: 1
            },
            sequence: 4,
            event: Input::Motion { dx: 1.0, dy: 1.0 }
        },
        token
    ));
}
#[test]
fn expired_receiver_lease_releases_without_dropping_connection() {
    let (engine, _rx) = fixture(false);
    remote(&engine);
    engine.native.gate.deadline.store(0, Ordering::SeqCst);
    process(
        &engine,
        Message::Input {
            token: engine.current_token(),
            sequence: 1,
            event: Input::Key {
                scan: 30,
                down: true,
            },
        },
    );
    assert_eq!(engine.snapshot().ownership, Ownership::Local);
    assert!(engine.snapshot().connected_id.is_some());
}

#[tokio::test]
async fn connection_falls_back_to_reachable_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let live = listener.local_addr().unwrap();
    let closed_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed = closed_listener.local_addr().unwrap();
    drop(closed_listener);
    let connected = timeout(
        Duration::from_secs(3),
        connect_routes(vec![(closed, None), (live, None)]),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(connected.peer_addr().unwrap(), live);
}
#[test]
fn stale_capture_events_cannot_cross_into_another_handoff() {
    let (engine, mut rx) = fixture(true);
    remote(&engine);
    let generation = engine.native.gate.generation.load(Ordering::SeqCst);
    engine.return_local();
    while rx.try_recv().is_ok() {}
    remote(&engine);
    engine.native_event(NativeEvent::Input {
        event: Input::Key {
            scan: 30,
            down: true,
        },
        captured: true,
        generation,
    });
    assert!(rx.try_recv().is_err());
    assert!(engine.safety.lock().unwrap().can_handoff());
}

#[test]
fn two_engines_change_main_handoff_and_release_in_both_directions() {
    fn peer(id: &str, other: &str, initiator: bool) -> (Arc<Engine>, mpsc::Receiver<Message>) {
        let mut store = Store::temporary();
        store.identity.id = id.into();
        let engine = Engine::new(
            Arc::new(store),
            Native::for_test(),
            vec![display(id), display(other)],
        );
        let (tx, rx) = mpsc::channel(64);
        *engine.connection.lock().unwrap() = Some(tx);
        *engine.token.lock().unwrap() = ControlToken {
            revision: 1,
            handoff: 0,
        };
        engine.initiator.store(initiator, Ordering::SeqCst);
        engine.update(|s| {
            s.connected_id = Some(other.into());
            s.controller_id = Some("a".into());
        });
        engine.ownership(Ownership::Local);
        (engine, rx)
    }
    fn deliver(from: &Engine, to: &Engine, rx: &mut mpsc::Receiver<Message>) -> usize {
        let tx = to.connection.lock().unwrap().clone().unwrap();
        let mut count = 0;
        while let Ok(message) = rx.try_recv() {
            to.process_message(
                message,
                false,
                &from.store.identity.id,
                &tx,
                Instant::now(),
                &mut 0,
            )
            .unwrap();
            count += 1;
        }
        count
    }
    let (a, mut from_a) = peer("a", "b", true);
    let (b, mut from_b) = peer("b", "a", false);
    let mut pump = || {
        for _ in 0..10 {
            if deliver(&a, &b, &mut from_a) + deliver(&b, &a, &mut from_b) == 0 {
                return;
            }
        }
        panic!("control protocol did not settle");
    };
    a.switch_control().unwrap();
    pump();
    assert!(a.native.gate.capturing());
    assert!(b.native.gate.active());
    assert!(!b.native.gate.capturing());
    b.return_local();
    pump();
    assert!(!a.native.gate.active());
    assert!(!b.native.gate.active());
    b.configure_control("b".into(), true).unwrap();
    pump();
    assert!(b.is_controller());
    assert!(!a.is_controller());
    assert_eq!(a.current_token(), b.current_token());
    b.switch_control().unwrap();
    pump();
    assert!(b.native.gate.capturing());
    assert!(a.native.gate.active());
    assert!(!a.native.gate.capturing());
    b.native_event(NativeEvent::Input {
        event: Input::Key {
            scan: 30,
            down: true,
        },
        captured: true,
        generation: b.native.gate.generation.load(Ordering::SeqCst),
    });
    pump();
    assert!(a.safety.lock().unwrap().keys.contains(&30));
    a.return_local();
    pump();
    assert!(a.safety.lock().unwrap().keys.is_empty());
    assert!(!b.native.gate.active());
}

#[test]
fn receiver_can_release_control_without_changing_restricted_main() {
    let (engine, mut rx) = fixture(false);
    engine.update(|s| s.control_from_either = false);
    remote(&engine);
    engine.switch_control().unwrap();
    assert_eq!(engine.snapshot().ownership, Ownership::Local);
    assert_eq!(engine.snapshot().controller_id.as_deref(), Some("peer"));
    assert!(!engine.native.gate.active());
    assert!(matches!(rx.try_recv().unwrap(), Message::Release { .. }));
}
#[test]
fn paused_receiver_declines_late_handoff_and_keeps_connection() {
    let (engine, mut rx) = fixture(false);
    engine.pause(true).unwrap();
    while rx.try_recv().is_ok() {}
    let mut token = engine.current_token();
    token.handoff += 1;
    process(
        &engine,
        Message::Handoff {
            token,
            display_id: "local-display".into(),
            x: 20.0,
            y: 20.0,
        },
    );
    assert_eq!(engine.snapshot().ownership, Ownership::Paused);
    assert_eq!(engine.snapshot().connected_id.as_deref(), Some("peer"));
    assert!(!engine.native.gate.active());
    assert!(matches!(rx.try_recv().unwrap(), Message::Release { .. }));
}
#[test]
fn manual_switch_uses_the_selected_peer_display() {
    let (engine, mut rx) = fixture(true);
    engine.update(|s| {
        let mut second = display("peer");
        second.id = "second".into();
        second.rect.x = 1000.0;
        s.placements = vec![Placement {
            display_id: second.id.clone(),
            rect: second.rect.clone(),
        }];
        s.displays.push(second);
    });
    engine.switch_control().unwrap();
    assert!(
        matches!(rx.try_recv().unwrap(), Message::Handoff { display_id, x: 1500.0, .. } if display_id == "second")
    );
}

#[test]
fn switch_shortcut_waits_for_the_arrow_key_as_well_as_modifiers() {
    let (engine, mut rx) = fixture(true);
    engine.native.set_local_input_held(true);
    engine.native_event(NativeEvent::Switch);
    engine.native_event(NativeEvent::Input {
        event: Input::Key {
            scan: 0x2a,
            down: false,
        },
        captured: false,
        generation: 0,
    });
    assert!(engine.switch_requested.load(Ordering::SeqCst));
    assert!(rx.try_recv().is_err());
    engine.native.set_local_input_held(false);
    engine.native_event(NativeEvent::Input {
        event: Input::Key {
            scan: 0x14d,
            down: false,
        },
        captured: false,
        generation: 0,
    });
    assert!(matches!(rx.try_recv().unwrap(), Message::Handoff { .. }));
    assert!(!engine.switch_requested.load(Ordering::SeqCst));
}

#[test]
fn connect_without_runtime_returns_error_without_changing_state() {
    let (engine, _rx) = fixture(true);
    let before = engine.snapshot();
    let error = engine.connect("192.168.1.2".into()).unwrap_err();
    assert!(error.to_string().contains("runtime unavailable"));
    assert!(!engine.busy.load(Ordering::SeqCst));
    assert_eq!(engine.snapshot(), before);
}

#[tokio::test]
async fn invalid_connection_address_does_not_start_pairing() {
    let (engine, _rx) = fixture(true);
    let before = engine.snapshot();
    assert!(engine.connect("".into()).is_err());
    assert!(!engine.busy.load(Ordering::SeqCst));
    assert_eq!(engine.snapshot(), before);
}

#[tokio::test]
async fn listeners_accept_ipv4_and_ipv6_on_the_same_port() {
    let ipv4 = bind_listener("127.0.0.1:0".parse().unwrap()).unwrap();
    let port = ipv4.local_addr().unwrap().port();
    let ipv6 = bind_listener(SocketAddr::from(([0u16; 8], port))).unwrap();
    let (client4, accepted4) = tokio::join!(
        TcpStream::connect(("127.0.0.1", port)),
        timeout(Duration::from_secs(2), ipv4.accept()),
    );
    assert!(client4.is_ok());
    assert!(accepted4.unwrap().is_ok());
    let (client6, accepted6) = tokio::join!(
        TcpStream::connect(("::1", port)),
        timeout(Duration::from_secs(2), ipv6.accept()),
    );
    assert!(client6.is_ok());
    assert!(accepted6.unwrap().is_ok());
}

#[tokio::test]
async fn listener_recovers_after_port_conflict_and_releases_port_on_shutdown() {
    let blocker = bind_listener("0.0.0.0:0".parse().unwrap()).unwrap();
    let port = blocker.local_addr().unwrap().port();
    let (engine, _rx) = fixture(true);
    let task = tokio::spawn(engine.clone().listen(port));
    timeout(Duration::from_secs(2), async {
        while engine.snapshot().error.is_none() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(engine
        .snapshot()
        .error
        .unwrap()
        .contains("Retrying automatically"));
    drop(blocker);
    timeout(Duration::from_secs(5), async {
        while engine.snapshot().error.is_some() {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(bind_listener(SocketAddr::from(([0, 0, 0, 0], port))).is_err());
    engine.shutdown();
    timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert!(bind_listener(SocketAddr::from(([0, 0, 0, 0], port))).is_ok());
}
