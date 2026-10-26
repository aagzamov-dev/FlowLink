use crate::{
    model::{Address, Device, PORT},
    Result,
};
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{collections::HashMap, net::IpAddr};
pub const SERVICE: &str = "_flowlink._tcp.local.";
pub fn addresses() -> Vec<Address> {
    let mut addresses = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter(|a| {
            a.is_oper_up()
                && !a.is_loopback()
                && !["lo", "awdl", "llw"]
                    .iter()
                    .any(|prefix| a.name.starts_with(prefix))
                && is_lan(a.ip())
        })
        .map(|a| {
            let name = a.name.clone();
            Address {
                kind: crate::platform::interface_kind(&name, a.index),
                interface: name,
                ip: match a.ip() {
                    IpAddr::V6(ip) if ip.is_unicast_link_local() => {
                        format!("{}%{}", ip, a.index.unwrap_or(0))
                    }
                    ip => ip.to_string(),
                },
            }
        })
        .collect::<Vec<_>>();
    addresses.sort_by_key(|a| {
        (
            a.kind != "Ethernet",
            a.kind != "Wi-Fi",
            a.ip.contains(':'),
            a.interface.clone(),
            a.ip.clone(),
        )
    });
    addresses.dedup_by(|a, b| a.ip == b.ip && a.interface == b.interface);
    addresses
}
pub fn is_lan(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_private()
                || ip.is_link_local()
                || (ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1]))
        }
        IpAddr::V6(ip) => ip.is_unique_local() || ip.is_unicast_link_local(),
    }
}
pub struct Discovery {
    daemon: ServiceDaemon,
    pub events: mdns_sd::Receiver<ServiceEvent>,
    registration: Option<String>,
}
impl Discovery {
    pub fn new(
        id: &str,
        name: &str,
        os: &str,
        receiver: bool,
        busy: bool,
        interface: Option<&str>,
        addresses: &[Address],
    ) -> Result<Self> {
        let daemon = ServiceDaemon::new()?;
        let local = addresses
            .iter()
            .filter(|a| interface.is_none_or(|name| a.interface == name))
            .collect::<Vec<_>>();
        daemon.disable_interface(IfKind::All)?;
        for name in local
            .iter()
            .map(|a| a.interface.clone())
            .collect::<std::collections::HashSet<_>>()
        {
            daemon.enable_interface(IfKind::Name(name))?;
        }
        let events = daemon.browse(SERVICE)?;
        let registration = if receiver {
            let properties = HashMap::from([
                ("id".into(), id.into()),
                ("name".into(), name.into()),
                ("os".into(), os.into()),
                ("version".into(), crate::model::PROTOCOL_VERSION.to_string()),
                ("busy".into(), if busy { "1" } else { "0" }.into()),
            ]);
            let ips = local
                .iter()
                .filter_map(|a| a.ip.parse::<IpAddr>().ok())
                .collect::<Vec<_>>();
            let service = ServiceInfo::new(
                SERVICE,
                &id[..16],
                &format!("flowlink-{}.local.", &id[..16]),
                ips.as_slice(),
                PORT,
                Some(properties),
            )?;
            let full = service.get_fullname().to_string();
            daemon.register(service)?;
            Some(full)
        } else {
            None
        };
        Ok(Self {
            daemon,
            events,
            registration,
        })
    }
    pub fn resolve(event: ServiceEvent) -> Option<Device> {
        if let ServiceEvent::ServiceResolved(info) = event {
            let id = info.get_property_val_str("id")?.to_string();
            if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let name = info
                .get_property_val_str("name")
                .unwrap_or("FlowLink device")
                .chars()
                .filter(|c| !c.is_control())
                .take(80)
                .collect();
            let mut addresses = info
                .get_addresses()
                .iter()
                .filter(|a| is_lan(a.to_ip_addr()))
                .map(|a| a.to_string())
                .collect::<Vec<_>>();
            addresses.sort_by_key(|a| (a.contains(':'), a.clone()));
            if addresses.is_empty() {
                return None;
            }
            Some(Device {
                id,
                name,
                os: info.get_property_val_str("os").unwrap_or("unknown").into(),
                addresses,
                port: info.get_port(),
                online: true,
                busy: info.get_property_val_str("busy") == Some("1"),
                trusted: false,
                favorite: false,
                auto_connect: false,
                last_seen: now(),
            })
        } else {
            None
        }
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        if let Some(name) = &self.registration {
            let _ = self.daemon.unregister(name);
        }
        let _ = self.daemon.shutdown();
    }
}
pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_addresses_are_not_lan() {
        assert!(!is_lan("8.8.8.8".parse().unwrap()));
        assert!(is_lan("192.168.1.2".parse().unwrap()));
        assert!(!is_lan("127.0.0.1".parse().unwrap()));
        assert!(is_lan("100.64.0.1".parse().unwrap()));
        assert!(is_lan("10.10.0.2".parse().unwrap()));
        assert!(is_lan("fd00::123".parse().unwrap()));
        assert!(!is_lan("100.128.0.1".parse().unwrap()));
    }
}
