use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

use crate::Rotation;

pub const BACKEND: &str = "/usr/local/sbin/arcader-install-backend";

#[derive(Clone, Debug)]
pub enum InstallMsg {
    Log(String),
    Progress { fraction: f32, phase: String },
    Done { ok: bool },
}

pub struct InstallJob {
    rx: Receiver<InstallMsg>,
}

impl InstallJob {
    pub fn start(disk: &str, dry_run: bool, rotation: Rotation) -> InstallJob {
        let (tx, rx) = mpsc::channel();
        let disk = disk.to_string();
        let rotate = rotation.xrandr();

        thread::spawn(move || {
            let mut cmd = Command::new("sudo");
            cmd.arg("-n").arg(BACKEND);
            if dry_run {
                cmd.arg("--dry-run");
            }
            cmd.args(["--rotate", rotate]);
            cmd.arg(&disk).stdout(Stdio::piped()).stderr(Stdio::piped());

            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(InstallMsg::Log(format!("failed to launch {BACKEND}: {e}")));
                    let _ = tx.send(InstallMsg::Done { ok: false });
                    return;
                }
            };

            if let Some(err) = child.stderr.take() {
                let tx_err = tx.clone();
                thread::spawn(move || pump(err, &tx_err));
            }
            if let Some(out) = child.stdout.take() {
                pump(out, &tx);
            }

            let ok = child.wait().map(|s| s.success()).unwrap_or(false);
            let _ = tx.send(InstallMsg::Done { ok });
        });

        InstallJob { rx }
    }

    pub fn try_recv(&self) -> Vec<InstallMsg> {
        self.rx.try_iter().collect()
    }
}

fn pump(stream: impl std::io::Read, tx: &Sender<InstallMsg>) {
    for line in BufReader::new(stream).lines().map_while(Result::ok) {
        if let Some((fraction, phase)) = parse_progress(&line) {
            let _ = tx.send(InstallMsg::Progress { fraction, phase });
        } else {
            let _ = tx.send(InstallMsg::Log(line));
        }
    }
}

fn parse_progress(line: &str) -> Option<(f32, String)> {
    let rest = line.trim_start().strip_prefix("@@P ")?;
    let (frac, phase) = rest.trim_start().split_once(char::is_whitespace)?;
    let frac: f32 = frac.parse().ok()?;
    Some((frac.clamp(0.0, 1.0), phase.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::parse_progress;

    #[test]
    fn parses_progress_control_line() {
        let (frac, phase) = parse_progress("@@P 0.4375 Copying system to disk — 45%").unwrap();
        assert!((frac - 0.4375).abs() < 1e-6);
        assert_eq!(phase, "Copying system to disk — 45%");
    }

    #[test]
    fn clamps_out_of_range_fraction() {
        assert_eq!(parse_progress("@@P 1.5 Done").unwrap().0, 1.0);
        assert_eq!(parse_progress("@@P -0.2 Odd").unwrap().0, 0.0);
    }

    #[test]
    fn ordinary_log_lines_are_not_progress() {
        assert!(parse_progress("[3/6] Copying system to disk ...").is_none());
        assert!(parse_progress("Architecture  : amd64").is_none());
        assert!(parse_progress("@@P").is_none());
        assert!(parse_progress("@@P 0.5").is_none());
        assert!(parse_progress("@@P notanumber Phase").is_none());
    }
}
