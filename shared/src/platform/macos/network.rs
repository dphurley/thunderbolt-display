use std::net::Ipv4Addr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceInfo {
    pub name: String,
    pub ipv4: Ipv4Addr,
    pub is_up: bool,
    pub is_running: bool,
}

pub fn list_active_ipv4_interfaces() -> Vec<InterfaceInfo> {
    let mut results = Vec::new();

    unsafe {
        let mut addrs: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut addrs) != 0 {
            return results;
        }

        let mut current = addrs;
        while !current.is_null() {
            let ifaddr = &*current;
            if !ifaddr.ifa_addr.is_null() && (*ifaddr.ifa_addr).sa_family as i32 == libc::AF_INET {
                let name = ifaddr.ifa_name;
                if !name.is_null() {
                    let interface_name = std::ffi::CStr::from_ptr(name)
                        .to_string_lossy()
                        .to_string();

                    let sockaddr_in: libc::sockaddr_in = *(ifaddr.ifa_addr as *const libc::sockaddr_in);
                    let ipv4 = Ipv4Addr::from(u32::from_be(sockaddr_in.sin_addr.s_addr));

                    let flags = ifaddr.ifa_flags as i32;
                    let is_up = flags & libc::IFF_UP != 0;
                    let is_running = flags & libc::IFF_RUNNING != 0;

                    if !ipv4.is_loopback() {
                        results.push(InterfaceInfo {
                            name: interface_name,
                            ipv4,
                            is_up,
                            is_running,
                        });
                    }
                }
            }

            current = ifaddr.ifa_next;
        }

        libc::freeifaddrs(addrs);
    }

    results
}

pub fn choose_preferred_interface(interfaces: &[InterfaceInfo]) -> Option<InterfaceInfo> {
    if let Some(interface) = interfaces.iter().find(|interface| {
        interface.is_up
            && interface.is_running
            && interface.name == "bridge0"
    }) {
        return Some(interface.clone());
    }

    if let Some(interface) = interfaces.iter().find(|interface| {
        interface.is_up
            && interface.is_running
            && interface.name.starts_with("en")
            && interface.ipv4.octets()[0] == 169
    }) {
        return Some(interface.clone());
    }

    interfaces
        .iter()
        .find(|interface| interface.is_up && interface.is_running)
        .cloned()
}

pub fn detect_preferred_interface() -> Option<InterfaceInfo> {
    let interfaces = list_active_ipv4_interfaces();
    choose_preferred_interface(&interfaces)
}

pub fn detect_preferred_ipv4() -> Option<Ipv4Addr> {
    detect_preferred_interface().map(|interface| interface.ipv4)
}

#[cfg(test)]
mod tests {
    use super::{choose_preferred_interface, InterfaceInfo};
    use std::net::Ipv4Addr;

    fn interface(name: &str, ip: [u8; 4], is_up: bool, is_running: bool) -> InterfaceInfo {
        InterfaceInfo {
            name: name.to_string(),
            ipv4: Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]),
            is_up,
            is_running,
        }
    }

    #[test]
    fn prefers_bridge0() {
        let interfaces = vec![
            interface("en0", [10, 0, 0, 5], true, true),
            interface("bridge0", [169, 254, 10, 2], true, true),
        ];

        let chosen = choose_preferred_interface(&interfaces).unwrap();
        assert_eq!(chosen.name, "bridge0");
    }

    #[test]
    fn falls_back_to_active_interface() {
        let interfaces = vec![
            interface("en0", [10, 0, 0, 5], true, true),
            interface("en1", [192, 168, 1, 2], false, false),
        ];

        let chosen = choose_preferred_interface(&interfaces).unwrap();
        assert_eq!(chosen.name, "en0");
    }

    #[test]
    fn prefers_link_local_on_en() {
        let interfaces = vec![
            interface("en0", [10, 0, 0, 5], true, true),
            interface("en5", [169, 254, 1, 20], true, true),
        ];

        let chosen = choose_preferred_interface(&interfaces).unwrap();
        assert_eq!(chosen.name, "en5");
    }
}
