use std::collections::HashMap;
use std::fs::{read_dir, read_link};
use std::path::{Path, PathBuf};
use std::net::IpAddr;

pub struct Connections {
    procfs_path: PathBuf,
}

// enum Family {

// }

// tcp4 = ("tcp", socket.AF_INET, socket.SOCK_STREAM)
// tcp6 = ("tcp6", socket.AF_INET6, socket.SOCK_STREAM)
// udp4 = ("udp", socket.AF_INET, socket.SOCK_DGRAM)
// udp6 = ("udp6", socket.AF_INET6, socket.SOCK_DGRAM)
// unix = ("unix", socket.AF_UNIX, None)

pub enum ConnType {
    All,   // (tcp4, tcp6, udp4, udp6, unix),
    Tcp,   // (tcp4, tcp6),
    Tcp4,  // (tcp4,),
    Tcp6,  // (tcp6,),
    Udp,   // (udp4, udp6),
    Udp4,  // (udp4,),
    Udp6,  // (udp6,),
    Unix,  // (unix,),
    Inet,  // (tcp4, tcp6, udp4, udp6),
    Inet4, // (tcp4, udp4),
    Inet6, // (tcp6, udp6),
}

pub struct Target {
    addr: IpAddr,
    port: u16,
}
pub struct Connection {
    fd: u16,
    family: String,
    conn_type: ConnType,
    laddr: Target,
    raddr: Target,
    status: String,
    pid: u16,
}

impl Connections {
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
        // ret = set()
        // for f, family, type_ in self.tmap[kind]:
        //     if family in (socket.AF_INET, socket.AF_INET6):
        //         ls = self.process_inet(
        //             "%s/net/%s" % (self._procfs_path, f),
        //             family, type_, inodes, filter_pid=pid)
        //     else:
        //         ls = self.process_unix(
        //             "%s/net/%s" % (self._procfs_path, f),
        //             family, inodes, filter_pid=pid)
        //     for fd, family, type_, laddr, raddr, status, bound_pid in ls:
        //         if pid:
        //             conn = _common.pconn(fd, family, type_, laddr, raddr,
        //                                  status)
        //         else:
        //             conn = _common.sconn(fd, family, type_, laddr, raddr,
        //                                  status, bound_pid)
        //         ret.add(conn)
        // return list(ret)
        unimplemented!()
    }

    //         inodes = defaultdict(list)
    //         for fd in os.listdir("%s/%s/fd" % (self._procfs_path, pid)):
    //             try:
    //                 inode = readlink("%s/%s/fd/%s" % (self._procfs_path, pid, fd))
    //             except OSError as err:
    //                 # ENOENT == file which is gone in the meantime;
    //                 # os.stat('/proc/%s' % self.pid) will be done later
    //                 # to force NSP (if it's the case)
    //                 if err.errno in (errno.ENOENT, errno.ESRCH):
    //                     continue
    //                 elif err.errno == errno.EINVAL:
    //                     # not a link
    //                     continue
    //                 else:
    //                     raise
    //             else:
    //                 if inode.startswith('socket:['):
    //                     # the process is using a socket
    //                     inode = inode[8:][:-1]
    //                     inodes[inode].append((pid, int(fd)))
    //         return inodes
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
        let mut path = self.procfs_path.clone();
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
