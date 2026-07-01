use std::process::Command;

use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct Disk {
    pub name: String,
    pub size: String,
    pub model: String,
    pub tran: String,
    pub rotational: bool,
    pub is_live: bool,
}

impl Disk {
    pub fn kind(&self) -> DiskKind {
        if self.tran == "usb" {
            DiskKind::Usb
        } else if self.rotational {
            DiskKind::Hdd
        } else {
            DiskKind::Ssd
        }
    }

    pub fn transport_badge(&self) -> String {
        if self.tran.is_empty() {
            "DISK".to_string()
        } else {
            self.tran.to_uppercase()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiskKind {
    Hdd,
    Ssd,
    Usb,
}

#[derive(Deserialize)]
struct LsblkRoot {
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Deserialize)]
struct LsblkDevice {
    name: String,
    #[serde(default)]
    size: Option<NumOrStr>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    tran: Option<String>,
    #[serde(default)]
    rota: Option<BoolIsh>,
    #[serde(rename = "type", default)]
    dtype: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum NumOrStr {
    Num(u64),
    Str(String),
}

impl NumOrStr {
    fn as_u64(&self) -> u64 {
        match self {
            NumOrStr::Num(n) => *n,
            NumOrStr::Str(s) => s.trim().parse().unwrap_or(0),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BoolIsh {
    Bool(bool),
    Str(String),
    Num(u8),
}

impl BoolIsh {
    fn truthy(&self) -> bool {
        match self {
            BoolIsh::Bool(b) => *b,
            BoolIsh::Num(n) => *n != 0,
            BoolIsh::Str(s) => s == "1" || s.eq_ignore_ascii_case("true"),
        }
    }
}

pub fn list_disks() -> Result<Vec<Disk>, String> {
    let live_dev = detect_live_device();

    let out = Command::new("lsblk")
        .args(["-J", "-b", "-d", "-o", "NAME,SIZE,MODEL,TRAN,ROTA,TYPE"])
        .output()
        .map_err(|e| format!("failed to run lsblk: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "lsblk exited with status {}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let parsed: LsblkRoot = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("could not parse lsblk output: {e}"))?;

    let mut disks: Vec<Disk> = parsed
        .blockdevices
        .into_iter()
        .filter(|d| d.dtype.as_deref() == Some("disk") && !d.name.starts_with("zram"))
        .map(|d| Disk {
            is_live: live_dev.as_deref() == Some(d.name.as_str()),
            size: human_size(d.size.as_ref().map(NumOrStr::as_u64).unwrap_or(0)),
            model: d.model.unwrap_or_default().trim().to_string(),
            tran: d.tran.unwrap_or_default(),
            rotational: d.rota.map(|b| b.truthy()).unwrap_or(true),
            name: d.name,
        })
        .collect();

    disks.sort_by(|a, b| a.is_live.cmp(&b.is_live).then(a.name.cmp(&b.name)));
    Ok(disks)
}

fn detect_live_device() -> Option<String> {
    let part = run_first_line(Command::new("findmnt").args(["-no", "SOURCE", "/run/live/medium"]))?;
    run_first_line(Command::new("lsblk").args(["-no", "pkname", &part]))
}

fn run_first_line(cmd: &mut Command) -> Option<String> {
    let out = cmd.output().ok().filter(|o| o.status.success())?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map(str::to_string)
}

fn human_size(bytes: u64) -> String {
    if bytes == 0 {
        return "-".to_string();
    }
    const UNITS: [&str; 6] = ["B", "K", "M", "G", "T", "P"];
    let mut val = bytes as f64;
    let mut unit = 0;
    while val >= 1024.0 && unit < UNITS.len() - 1 {
        val /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[unit])
    } else {
        format!("{val:.1}{}", UNITS[unit])
    }
}
