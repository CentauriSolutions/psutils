use std::collections::HashMap;
use std::fs::{read_dir, read_link, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::net::IpAddr;
use std::u16;

use data_encoding;
// use hex;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn it_decodes_ipv4() {
        // "0500000A:0016" -> ("10.0.0.5", 22)
        let input = "0500000A:0016";
        let family = Socket::AF_INET;

        let expected = Target { addr: IpAddr::V4("10.0.0.5".parse::<Ipv4Addr>().unwrap()), port: 22};
        assert_eq!(Connections::decode_address(input, &family), Some(expected));
    }

    #[ignore]
    #[test]
    fn it_decodes_ipv6() {
        // "0000000000000000FFFF00000100007F:9E49" -> ("::ffff:127.0.0.1", 40521)
        let input = "0000000000000000FFFF00000100007F:9E49";
        let family = Socket::AF_INET6;
        let expected = Target {
            addr: IpAddr::V6("::ffff:127.0.0.1".parse::<Ipv6Addr>().unwrap()),
            port: 40521,
        };
        assert_eq!(Connections::decode_address(input, &family), Some(expected));
    }
}

pub struct Connections {
    procfs_path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConnType {
    All,   // (tcp4, tcp6, udp4, udp6, unix),
    Tcp,   // (tcp4, tcp6),
    Tcp4,  // (tcp4,),let file = BufReader::new(&f);
    Tcp6,  // (tcp6,),
    Udp,   // (udp4, udp6),
    Udp4,  // (udp4,),
    Udp6,  // (udp6,),
    Unix,  // (unix,),
    Inet,  // (tcp4, tcp6, udp4, udp6),
    Inet4, // (tcp4, udp4),
    Inet6, // (tcp6, udp6),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    addr: IpAddr,
    port: u16,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Connection {
    fd: u16,
    family: Socket,
    conn_type: ConnType,
    laddr: Target,
    raddr: Target,
    status: Option<State>,
    pid: u16,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    CONN_ESTABLISHED,
    CONN_SYN_SENT,
    CONN_SYN_RECV,
    CONN_FIN_WAIT1,
    CONN_FIN_WAIT2,
    CONN_TIME_WAIT,
    CONN_CLOSE,
    CONN_CLOSE_WAIT,
    CONN_LAST_ACK,
    CONN_LISTEN,
    CONN_CLOSING,
}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Socket {
    AF_INET,
    AF_INET6,
    AF_UNIX,
    SOCK_STREAM,
    SOCK_DGRAM,
}

impl Connections {
    fn tcp_statuses() -> HashMap<&'static str, State> {
        // https://github.com/torvalds/linux/blob/master/include/net/tcp_states.h
        hashmap!{
            "01" => State::CONN_ESTABLISHED,
            "02" => State::CONN_SYN_SENT,
            "03" => State::CONN_SYN_RECV,
            "04" => State::CONN_FIN_WAIT1,
            "05" => State::CONN_FIN_WAIT2,
            "06" => State::CONN_TIME_WAIT,
            "07" => State::CONN_CLOSE,
            "08" => State::CONN_CLOSE_WAIT,
            "09" => State::CONN_LAST_ACK,
            "0A" => State::CONN_LISTEN,
            "0B" => State::CONN_CLOSING,
        }
    }

    fn types(kind: &ConnType) -> Vec<(&'static str, Socket, Option<Socket>)> {
        let tcp4 = ("tcp", Socket::AF_INET, Some(Socket::SOCK_STREAM));
        let tcp6 = ("tcp6", Socket::AF_INET6, Some(Socket::SOCK_STREAM));
        let udp4 = ("udp", Socket::AF_INET, Some(Socket::SOCK_DGRAM));
        let udp6 = ("udp6", Socket::AF_INET6, Some(Socket::SOCK_DGRAM));
        let unix = ("unix", Socket::AF_UNIX, None);
        match kind {
            &ConnType::All => {
                // (tcp4, tcp6, udp4, udp6, unix),
                vec![tcp4, tcp6, udp4, udp6, unix]
            }
            &ConnType::Tcp => {
                // (tcp4, tcp6),
                vec![tcp4, tcp6]
            }
            &ConnType::Tcp4 => {
                // (tcp4,),
                vec![tcp4]
            }
            &ConnType::Tcp6 => {
                // (tcp6,),
                vec![tcp6]
            }
            &ConnType::Udp => {
                // (udp4, udp6),
                vec![udp4, udp6]
            }
            &ConnType::Udp4 => {
                // (udp4,),
                vec![udp4]
            }
            &ConnType::Udp6 => {
                // (udp6,),
                vec![udp6]
            }
            &ConnType::Unix => {
                // (unix,),
                vec![unix]
            }
            &ConnType::Inet => {
                // (tcp4, tcp6, udp4, udp6),
                vec![tcp4, tcp6, udp4, udp6]
            }
            &ConnType::Inet4 => {
                // (tcp4, udp4),
                vec![tcp4, udp4]
            }
            &ConnType::Inet6 => {
                // (tcp6, udp6),
                vec![tcp6, udp6]
            }
        }
    }

    pub fn retrieve(kind: &ConnType, pid: Option<u16>) -> Vec<Connection> {
        let procfs_path = PathBuf::from("/proc");
        let connection = Connections { procfs_path };
        let inodes = match pid {
            Some(pid) => {
                let i = connection.get_proc_inodes(pid);
                println!("proc inodes: {:?}", i);
                if i.is_empty() {
                    return vec![];
                }
                i
            }
            None => connection.get_all_inodes(),
        };
        println!("Inodes: {:?}", inodes);
        let mut ret = Vec::new();
        for (f, family, _type) in Connections::types(kind) {
            let proc_path = {
                let mut p: PathBuf = connection.procfs_path.clone();
                p.push("net");
                p.push(f);
                p
            };
            ret = if vec![Socket::AF_INET, Socket::AF_INET6].contains(&family) {
                match _type {
                    Some(t) => Connections::process_inet(&proc_path, family, &t, &inodes, pid, kind),
                    None => {
                        println!("Error with socket types");
                        continue;
                    }
                }
            } else {
                unimplemented!()
                //         ls = self.process_unix(
                //             "%s/net/%s" % (self._procfs_path, f),
                //             family, inodes, filter_pid=pid)
            };
        }
        ret
    }

    fn process_inet<P: AsRef<Path>>(
        file: &P,
        family: Socket,
        _type: &Socket,
        inodes: &HashMap<String, Vec<(u16, u16)>>,
        filter_pid: Option<u16>,
        kind: &ConnType,
    ) -> Vec<Connection> {
        let mut connections = vec![];
        let file = file.as_ref();
        if file.ends_with("6") && !file.exists() {
            // IPV6 is not supported by the machine
            return connections;
        }
        if let Ok(f) = File::open(file) {
            let f = BufReader::new(&f);
            let mut lines = f.lines().enumerate();
            let _ = lines.next(); // skip the first line
            for (line_number, line) in lines {
                if let Ok(line) = line {
                    let mut split = line.split(" ");
                    if let (
                        _,
                        Some(mut laddr),
                        Some(mut raddr),
                        Some(status),
                        _,
                        _,
                        _,
                        _,
                        _,
                        Some(inode),
                        _,
                    ) = (
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                        split.next(),
                    ) {
                        if inodes.contains_key(inode) {
                            let (pid, fd) = inodes[inode][0];
                            if filter_pid.is_none() || filter_pid != Some(pid) {
                                continue;
                            }
                            let status: Option<State> = if _type == &Socket::SOCK_STREAM {
                                Connections::tcp_statuses().get(status).map(|s| s.clone())
                            } else {
                                None
                            };
                            if let (Some(raddr), Some(laddr)) = (
                                Connections::decode_address(raddr, &family),
                                Connections::decode_address(laddr, &family),
                            ) {
                                connections.push(
                                    Connection {
                                        fd,
                                        family,
                                        conn_type: *kind,
                                        laddr,
                                        raddr,
                                        status,
                                        pid,
                                    }
                                )
                            }
                        }
                    } else {
                        println!(
                            "Error while parsing {}; malformed line {} {}",
                            file.display(),
                            line_number,
                            line
                        );
                    }
                }
            }
        };
        connections
    }

    /// Accept an "ip:port" address as displayed in /proc/net/*
    /// and convert it into a human readable form, like:
    /// "0500000A:0016" -> ("10.0.0.5", 22)
    /// "0000000000000000FFFF00000100007F:9E49" -> ("::ffff:127.0.0.1", 40521)
    /// The IP address portion is a little or big endian four-byte
    /// hexadecimal number; that is, the least significant byte is listed
    /// first, so we need to reverse the order of the bytes to convert it
    /// to an IP address.
    /// The port is represented as a two-byte hexadecimal number.
    /// Reference:
    /// http://linuxdevcenter.com/pub/a/linux/2000/11/16/LinuxAdmin.html
    fn decode_address(addr: &str, family: &Socket) -> Option<Target> {
        let mut split = addr.split(':');
        if let (Some(ip), Some(port)) = (split.next(), split.next()) {
            println!("Starting port: {}", port);
            let port: u16 = match u16::from_str_radix(&port, 16) {
                Ok(p) => p,
                Err(_) => return None,
            };
            println!("Bytes: {:?}", ip);

            if let Ok(mut ip) = data_encoding::base16::decode(ip.as_bytes()) {
                match family {
                    &Socket::AF_INET => {
                        ip.reverse();
                        println!("Encoded: {:?}", ip);
                        let ip: String = vec_to_s(ip, ".");
                        println!("Stringed: {:?}", ip);
                        if let Ok(addr) = ip.parse() {
                            return Some(Target {addr, port});
                        }
                    }
                    &Socket::AF_INET6 => {
                        // if let Ok(ip) = vec_to_s(ip, "::").parse() {
                        //     return Some((ip, port))
                        // }
                        unimplemented!()
                    },
                    _ => unreachable!(),
                }
            }
        }

        None
    }

    fn get_proc_inodes(&self, pid: u16) -> HashMap<String, Vec<(u16, u16)>> {
        let mut inodes = HashMap::new();
        let mut path = self.procfs_path.clone();
        path.push(format!("{}", pid));
        path.push("fd");
        for dirent in read_dir(&path).unwrap() {
            let dirent = dirent.unwrap();
            println!("dirent: {:?}", dirent);
            let path = dirent.path();
            match read_link(&path) {
                Ok(l) => {
                    let l = l.to_string_lossy();
                    println!(
                        "Link: {} -> {:?} (socket: [{}])",
                        path.display(),
                        l,
                        l.starts_with("socket:[")
                    );
                    if l.starts_with("socket:[") {
                        let mut inode: String = l[8..].into();
                        inode.pop();
                        println!("About to push inode: {}", inode);
                        let entry = inodes.entry(inode).or_insert_with(|| vec![]);
                        entry.push((
                            pid,
                            path.file_name()
                                .unwrap()
                                .to_string_lossy()
                                .parse::<u16>()
                                .unwrap(),
                        ));
                    }
                }
                Err(_e) => {
                    // println!("Error: {:?}", e);
                }
            }
        }
        inodes
    }

    fn get_all_inodes(&self) -> HashMap<String, Vec<(u16, u16)>> {
        let mut inodes = HashMap::new();
        let path = self.procfs_path.clone();
        for dirent in read_dir(&path).unwrap() {
            if dirent.is_err() {
                continue;
            }
            let dirent = dirent.unwrap();
            let part = dirent.path();
            let part = part.file_name().unwrap().to_string_lossy();
            if let Ok(pid_part) = part.parse::<u16>() {
                for (key, mut value) in &mut self.get_proc_inodes(pid_part) {
                    let inode = inodes.entry(key.clone()).or_insert_with(|| vec![]);
                    inode.append(&mut value);
                }
            }
        }
        inodes
    }
}

fn vec_to_s(v: Vec<u8>, join_char: &str) -> String {
    let s = v.iter()
        .map(|c| format!("{}", c))
        .collect::<Vec<String>>()
        .join(join_char);
    println!("s: {:?}", s);
    s
}
