use std::fs::File;
use std::io::{BufRead, BufReader};

const STAT_FILE: &str = "/proc/stat";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_decodes_line() {
        let line = "cpu  13935784 23594 4431910 129425881 49388 0 115234 0 0 0";
        let cpu_time = CpuTime::decode(line).unwrap();
        assert_eq!(cpu_time.user, 13935784);
        assert_eq!(cpu_time.nice, 23594);
        assert_eq!(cpu_time.system, 4431910);
        assert_eq!(cpu_time.idle, 129425881);
        assert_eq!(cpu_time.iowait, 49388);
        assert_eq!(cpu_time.irq, 0);
        assert_eq!(cpu_time.soft_irq, 115234);
        assert_eq!(cpu_time.steal, 0);
        assert_eq!(cpu_time.guest, 0);
        assert_eq!(cpu_time.guest_nice, 0);
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CpuTime {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub soft_irq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

impl CpuTime {
    fn decode(input: &str) -> Option<CpuTime> {
        let mut split = input.split_whitespace();
        let _ = split.next(); // cpu
        if let (
            Some(user),
            Some(nice),
            Some(system),
            Some(idle),
            Some(iowait),
            Some(irq),
            Some(soft_irq),
            Some(steal),
            Some(guest),
            Some(guest_nice),
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
        ) {
            Some(CpuTime {
                user: user.parse().unwrap_or(0),
                nice: nice.parse().unwrap_or(0),
                system: system.parse().unwrap_or(0),
                idle: idle.parse().unwrap_or(0),
                iowait: iowait.parse().unwrap_or(0),
                irq: irq.parse().unwrap_or(0),
                soft_irq: soft_irq.parse().unwrap_or(0),
                steal: steal.parse().unwrap_or(0),
                guest: guest.parse().unwrap_or(0),
                guest_nice: guest_nice.parse().unwrap_or(0),
            })
        } else {
            None
        }
    }
}

pub fn times() -> Option<CpuTime> {
    match File::open(STAT_FILE) {
        Ok(f) => {
            let f = BufReader::new(&f);
            let mut lines = f.lines();
            if let Some(Ok(line)) = lines.next() {
                return CpuTime::decode(&line);
            }
        }
        Err(e) => debug!("Error opening {}: {:?}", STAT_FILE, e),
    }
    None
}

pub fn cpu_time() -> Vec<CpuTime> {
    let mut times = Vec::with_capacity(48);
    match File::open(STAT_FILE) {
        Ok(f) => {
            let f = BufReader::new(&f);
            let mut lines = f.lines().enumerate();
            let _ = lines.next();
            for (line_number, line) in lines {
                if let Ok(line) = line {
                    if &line[0..=2] != "cpu" {
                        break;
                    }
                    match CpuTime::decode(&line) {
                        Some(c) => times.push(c),
                        None => debug!(
                            "Error while parsing {}; malformed line {} {}",
                            STAT_FILE, line_number, line
                        ),
                    }
                }
            }
        }
        Err(e) => debug!("Error opening {}: {:?}", STAT_FILE, e),
    }
    times
}
