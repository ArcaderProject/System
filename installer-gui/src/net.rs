use std::collections::HashMap;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Clone, Debug)]
pub struct Network {
    pub ssid: String,
    pub signal: u8,
    pub secured: bool,
    pub active: bool,
}

pub struct ScanJob {
    rx: Receiver<Result<Vec<Network>, String>>,
}

impl ScanJob {
    pub fn start() -> ScanJob {
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let _ = nmcli(&["device", "wifi", "rescan"]).status();
            let _ = tx.send(scan());
        });
        ScanJob { rx }
    }

    pub fn poll(&self) -> Option<Result<Vec<Network>, String>> {
        self.rx.try_recv().ok()
    }
}

pub struct ConnectJob {
    rx: Receiver<Result<(), String>>,
}

impl ConnectJob {
    pub fn start(ssid: &str, password: Option<String>) -> ConnectJob {
        let (tx, rx) = mpsc::channel();
        let ssid = ssid.to_string();
        thread::spawn(move || {
            let mut args = vec!["device".to_string(), "wifi".to_string(), "connect".to_string(), ssid];
            if let Some(pw) = password {
                args.push("password".to_string());
                args.push(pw);
            }
            let refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let res = match nmcli(&refs).output() {
                Ok(o) if o.status.success() => Ok(()),
                Ok(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
                Err(e) => Err(format!("nmcli not available: {e}")),
            };
            let _ = tx.send(res);
        });
        ConnectJob { rx }
    }

    pub fn poll(&self) -> Option<Result<(), String>> {
        self.rx.try_recv().ok()
    }
}

fn nmcli(args: &[&str]) -> Command {
    let mut cmd = Command::new("sudo");
    cmd.arg("-n").arg("nmcli").args(args);
    cmd
}

fn scan() -> Result<Vec<Network>, String> {
    let out = nmcli(&["-t", "-f", "ACTIVE,SIGNAL,SECURITY,SSID", "device", "wifi", "list"])
        .output()
        .map_err(|e| format!("nmcli not available: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut best: HashMap<String, Network> = HashMap::new();
    for line in text.lines() {
        let f = split_terse(line);
        if f.len() < 4 {
            continue;
        }
        let ssid = f[3..].join(":");
        if ssid.is_empty() {
            continue;
        }
        let net = Network {
            active: f[0] == "yes",
            signal: f[1].trim().parse().unwrap_or(0),
            secured: !f[2].trim().is_empty() && f[2] != "--",
            ssid: ssid.clone(),
        };
        best.entry(ssid)
            .and_modify(|e| {
                if net.signal > e.signal {
                    e.signal = net.signal;
                    e.secured = net.secured;
                }
                e.active |= net.active;
            })
            .or_insert(net);
    }

    let mut nets: Vec<Network> = best.into_values().collect();
    nets.sort_by(|a, b| b.active.cmp(&a.active).then(b.signal.cmp(&a.signal)));
    Ok(nets)
}

fn split_terse(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    cur.push(n);
                }
            }
            ':' => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}
