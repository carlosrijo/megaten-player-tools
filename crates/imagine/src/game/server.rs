//! Which server a running client is playing on. The game folder's `VersionData.txt` (the launcher's server list) names
//! each server with its address:
//!
//! ```text
//! [versions]
//! title = NewMoon
//! server = 54.39.131.171:10666
//! tag = newmoon
//!
//! title = Local Server
//! server = 127.0.0.1:10666
//! tag = local
//! ```
//!
//! and the client keeps a TCP connection open to that host while a character is in game (the channel server, port
//! 14666 on NewMoon: same host, another port, so only the address is compared). Checked 2026-11-25 on both of the
//! user's clients: each was connected to 54.39.131.171:14666, the "NewMoon" entry.

use super::client_reader::ClientProcess;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use std::path::Path;

/// One entry of VersionData.txt.
#[derive(Debug, Clone, PartialEq)]
pub struct GameServer {
    pub title: String,
    pub tag: String,
    /// "host:port" as written
    pub server: String,
}

impl GameServer {
    /// The NewMoon server: its entry's tag or title says so ("newmoon", "NewMoon", "New Moon").
    pub fn is_newmoon(&self) -> bool {
        let squash = |s: &str| s.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_ascii_lowercase();
        squash(&self.tag) == "newmoon" || squash(&self.title) == "newmoon"
    }
}

/// The server list of VersionData.txt: every `title` starts an entry; `server` and `tag` belong to the last one.
pub fn parse_version_data(text: &str) -> Vec<GameServer> {
    let mut res: Vec<GameServer> = Vec::new();
    let mut in_versions = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_versions = line.eq_ignore_ascii_case("[versions]");
            continue;
        }
        if !in_versions {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_string());
        match k.as_str() {
            "title" => res.push(GameServer { title: v, tag: String::new(), server: String::new() }),
            "server" => {
                if let Some(e) = res.last_mut() {
                    e.server = v;
                }
            }
            "tag" => {
                if let Some(e) = res.last_mut() {
                    e.tag = v;
                }
            }
            _ => {}
        }
    }
    res
}

/// The addresses an entry's "host:port" stands for (a host name is looked up).
fn addresses_of(server: &str) -> Vec<IpAddr> {
    let host = server.rsplit_once(':').map(|(h, _)| h).unwrap_or(server).trim();
    if let Ok(ip) = host.parse::<IpAddr>() {
        return vec![ip];
    }
    if host.is_empty() {
        return Vec::new();
    }
    (host, 0).to_socket_addrs().map(|it| it.map(|a| a.ip()).collect()).unwrap_or_default()
}

/// The server the client is connected to, from its folder's VersionData.txt and its open connections. None when the
/// file is missing, the connections can't be read, or none of them goes to a listed server.
pub fn connected_server(client: &ClientProcess) -> Option<GameServer> {
    let dir = client.exe_path.as_deref().and_then(Path::parent)?;
    let text = std::fs::read(dir.join("VersionData.txt")).ok()?;
    let servers = parse_version_data(&String::from_utf8_lossy(&text));
    if servers.is_empty() {
        return None;
    }
    let remote = sys::remote_addresses(client.pid)?;
    servers.into_iter().find(|s| addresses_of(&s.server).iter().any(|a| matches!(a, IpAddr::V4(v4) if remote.contains(v4))))
}

mod sys {
    use super::Ipv4Addr;
    use windows::Win32::NetworkManagement::IpHelper::{GetExtendedTcpTable, TCP_TABLE_OWNER_PID_CONNECTIONS};

    /// The IPv4 addresses a process has TCP connections to (GetExtendedTcpTable: a count, then rows of six u32:
    /// state, local address, local port, remote address, remote port, owning pid; addresses in network order).
    pub fn remote_addresses(pid: u32) -> Option<Vec<Ipv4Addr>> {
        const AF_INET: u32 = 2;
        let mut size = 0u32;
        unsafe {
            let _ = GetExtendedTcpTable(None, &mut size, false, AF_INET, TCP_TABLE_OWNER_PID_CONNECTIONS, 0);
        }
        if size == 0 {
            return None;
        }
        // a connection can open between the two calls: leave room for a few more
        size += 24 * 16;
        let mut buf = vec![0u32; (size as usize).div_ceil(4)];
        let ok = unsafe { GetExtendedTcpTable(Some(buf.as_mut_ptr() as *mut _), &mut size, false, AF_INET, TCP_TABLE_OWNER_PID_CONNECTIONS, 0) };
        if ok != 0 {
            return None;
        }
        let n = buf[0] as usize;
        let mut res = Vec::new();
        for i in 0..n {
            let row = 1 + i * 6;
            if row + 6 > buf.len() {
                break;
            }
            if buf[row + 5] == pid {
                res.push(Ipv4Addr::from(buf[row + 3].to_le_bytes()));
            }
        }
        Some(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_launcher_list() {
        let t = "[versions]\r\ntitle = NewMoon\r\nserver = 54.39.131.171:10666\r\ntag = newmoon\r\n\r\ntitle = Local Server\r\nserver = 127.0.0.1:10666\r\ntag = local\r\n\r\n[newmoon]\r\nwebaccess.sdat\r\n";
        let s = parse_version_data(t);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], GameServer { title: "NewMoon".into(), tag: "newmoon".into(), server: "54.39.131.171:10666".into() });
        assert!(s[0].is_newmoon());
        assert!(!s[1].is_newmoon());
        assert_eq!(addresses_of(&s[1].server), vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))]);
    }
}
