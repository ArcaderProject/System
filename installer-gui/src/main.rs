#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod disks;
mod install;
mod net;
mod theme;
mod ui;

use eframe::egui;

use disks::Disk;
use install::{InstallJob, InstallMsg};
use net::{ConnectJob, Network, ScanJob};

pub const BANNER_PNG: &[u8] = include_bytes!("../assets/banner.png");
pub const BACKGROUND_PNG: &[u8] = include_bytes!("../assets/background.png");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Welcome,
    SelectDisk,
    Display,
    Keyboard,
    Network,
    Confirm,
    Installing,
    Done,
    Failed,
}

impl Step {
    pub const INDICATOR: [(&'static str, Step); 7] = [
        ("Welcome", Step::Welcome),
        ("Disk", Step::SelectDisk),
        ("Display", Step::Display),
        ("Keyboard", Step::Keyboard),
        ("Wi-Fi", Step::Network),
        ("Confirm", Step::Confirm),
        ("Install", Step::Installing),
    ];
}

#[derive(Clone, Copy)]
pub struct KeyboardLayout {
    pub code: &'static str,
    pub label: &'static str,
}

pub const LAYOUTS: [KeyboardLayout; 16] = [
    KeyboardLayout { code: "us", label: "English (US)" },
    KeyboardLayout { code: "gb", label: "English (UK)" },
    KeyboardLayout { code: "de", label: "German" },
    KeyboardLayout { code: "fr", label: "French" },
    KeyboardLayout { code: "es", label: "Spanish" },
    KeyboardLayout { code: "it", label: "Italian" },
    KeyboardLayout { code: "pt", label: "Portuguese" },
    KeyboardLayout { code: "nl", label: "Dutch" },
    KeyboardLayout { code: "se", label: "Swedish" },
    KeyboardLayout { code: "no", label: "Norwegian" },
    KeyboardLayout { code: "dk", label: "Danish" },
    KeyboardLayout { code: "fi", label: "Finnish" },
    KeyboardLayout { code: "pl", label: "Polish" },
    KeyboardLayout { code: "ru", label: "Russian" },
    KeyboardLayout { code: "ch", label: "Swiss" },
    KeyboardLayout { code: "ca", label: "Canadian (Fr)" },
];

#[derive(Clone, Debug, Default, PartialEq)]
pub enum NetStatus {
    #[default]
    Idle,
    Scanning,
    Connecting,
    Connected(String),
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Normal,
    Inverted,
}

impl Rotation {
    pub const ALL: [Rotation; 2] = [Rotation::Normal, Rotation::Inverted];

    pub fn xrandr(self) -> &'static str {
        match self {
            Rotation::Normal => "normal",
            Rotation::Inverted => "inverted",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Rotation::Normal => "Landscape",
            Rotation::Inverted => "Upside down",
        }
    }
}

pub(crate) struct Textures {
    pub(crate) banner: egui::TextureHandle,
    pub(crate) background: egui::TextureHandle,
}

pub struct InstallerApp {
    pub(crate) step: Step,
    pub(crate) disks: Vec<Disk>,
    pub(crate) disk_error: Option<String>,
    pub(crate) selected: usize,
    pub(crate) rotation: Rotation,
    pub(crate) keyboard: usize,
    pub(crate) target_disk: Option<String>,
    pub(crate) confirm_ack: bool,
    pub(crate) dry_run: bool,

    pub(crate) networks: Vec<Network>,
    pub(crate) net_selected: Option<usize>,
    pub(crate) net_password: String,
    pub(crate) net_status: NetStatus,
    pub(crate) net_connected: Option<String>,
    pub(crate) scan_job: Option<ScanJob>,
    pub(crate) connect_job: Option<ConnectJob>,
    pub(crate) connecting_ssid: Option<String>,

    pub(crate) job: Option<InstallJob>,
    pub(crate) progress: f32,
    pub(crate) phase: String,
    pub(crate) log: Vec<String>,
    pub(crate) failed_reason: Option<String>,

    pub(crate) focus: ui::Focus,
    pub(crate) last_step: Step,

    pub(crate) textures: Option<Textures>,
}

impl InstallerApp {
    fn new(dry_run: bool) -> Self {
        let (disks, disk_error) = match disks::list_disks() {
            Ok(d) => (d, None),
            Err(e) => (Vec::new(), Some(e)),
        };
        let selected = disks.iter().position(|d| !d.is_live).unwrap_or(0);
        Self {
            step: Step::Welcome,
            disks,
            disk_error,
            selected,
            rotation: Rotation::Normal,
            keyboard: LAYOUTS.iter().position(|l| l.code == "us").unwrap_or(0),
            target_disk: None,
            confirm_ack: false,
            dry_run,
            networks: Vec::new(),
            net_selected: None,
            net_password: String::new(),
            net_status: NetStatus::Idle,
            net_connected: None,
            scan_job: None,
            connect_job: None,
            connecting_ssid: None,
            job: None,
            progress: 0.0,
            phase: String::new(),
            log: Vec::new(),
            failed_reason: None,
            focus: ui::Focus::default(),
            last_step: Step::Done,
            textures: None,
        }
    }

    pub(crate) fn refresh_disks(&mut self) {
        match disks::list_disks() {
            Ok(d) => {
                self.selected = d.iter().position(|x| !x.is_live).unwrap_or(0);
                self.disks = d;
                self.disk_error = None;
            }
            Err(e) => self.disk_error = Some(e),
        }
    }

    pub(crate) fn selected_disk(&self) -> Option<&Disk> {
        self.disks.get(self.selected).filter(|d| !d.is_live)
    }

    pub(crate) fn keyboard_code(&self) -> &'static str {
        LAYOUTS.get(self.keyboard).map_or("us", |l| l.code)
    }

    pub(crate) fn keyboard_label(&self) -> &'static str {
        LAYOUTS.get(self.keyboard).map_or("English (US)", |l| l.label)
    }

    pub(crate) fn set_keyboard(&mut self, index: usize) {
        if index >= LAYOUTS.len() {
            return;
        }
        self.keyboard = index;
        let _ = std::process::Command::new("setxkbmap")
            .arg(LAYOUTS[index].code)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    pub(crate) fn start_scan(&mut self) {
        self.networks.clear();
        self.net_selected = None;
        self.net_status = NetStatus::Scanning;
        self.scan_job = Some(ScanJob::start());
    }

    pub(crate) fn start_connect(&mut self) {
        let Some(net) = self.net_selected.and_then(|i| self.networks.get(i)).cloned() else {
            return;
        };
        let password = net.secured.then(|| self.net_password.clone());
        self.connecting_ssid = Some(net.ssid.clone());
        self.net_status = NetStatus::Connecting;
        self.connect_job = Some(ConnectJob::start(&net.ssid, password));
    }

    fn pump_net(&mut self) {
        if let Some(job) = &self.scan_job {
            if let Some(res) = job.poll() {
                match res {
                    Ok(nets) => {
                        self.networks = nets;
                        if self.net_status == NetStatus::Scanning {
                            self.net_status = NetStatus::Idle;
                        }
                    }
                    Err(e) => self.net_status = NetStatus::Failed(e),
                }
                self.scan_job = None;
            }
        }
        if let Some(job) = &self.connect_job {
            if let Some(res) = job.poll() {
                let ssid = self.connecting_ssid.take().unwrap_or_default();
                self.net_status = match res {
                    Ok(()) => {
                        self.net_connected = Some(ssid.clone());
                        NetStatus::Connected(ssid)
                    }
                    Err(e) => NetStatus::Failed(e),
                };
                self.connect_job = None;
            }
        }
    }

    pub(crate) fn begin_install(&mut self) {
        let Some(disk) = self.selected_disk().map(|d| d.name.clone()) else {
            return;
        };
        self.target_disk = Some(disk.clone());
        self.log.clear();
        self.progress = 0.0;
        self.phase = "Starting…".to_string();
        self.failed_reason = None;
        self.job = Some(InstallJob::start(&disk, self.dry_run, self.rotation, self.keyboard_code()));
        self.step = Step::Installing;
    }

    fn pump_install(&mut self) {
        let Some(job) = &self.job else { return };
        for msg in job.try_recv() {
            match msg {
                InstallMsg::Log(line) => {
                    self.log.push(line);
                    if self.log.len() > 4000 {
                        self.log.drain(0..1000);
                    }
                }
                InstallMsg::Progress { fraction, phase } => {
                    self.progress = self.progress.max(fraction);
                    self.phase = phase;
                }
                InstallMsg::Done { ok } => {
                    if ok {
                        self.progress = 1.0;
                        self.step = Step::Done;
                    } else {
                        self.failed_reason = Some(
                            self.log
                                .iter()
                                .rev()
                                .find(|l| !l.trim().is_empty())
                                .cloned()
                                .unwrap_or_else(|| "The installer backend exited with an error.".into()),
                        );
                        self.step = Step::Failed;
                    }
                    self.job = None;
                }
            }
        }
    }

    pub(crate) fn textures(&mut self, ctx: &egui::Context) -> &Textures {
        self.textures.get_or_insert_with(|| Textures {
            banner: load_texture(ctx, "banner", BANNER_PNG),
            background: load_texture(ctx, "background", BACKGROUND_PNG),
        })
    }
}

impl eframe::App for InstallerApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.02, 0.03, 0.05, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pump_install();
        self.pump_net();
        let busy = self.step == Step::Installing
            || self.scan_job.is_some()
            || self.connect_job.is_some();
        if busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }
        ui::draw(self, ctx);
    }
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("embedded PNG should decode")
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_flat_samples().as_slice());
    ctx.load_texture(name, color, egui::TextureOptions::LINEAR)
}

fn main() -> eframe::Result<()> {
    let dry_run = std::env::args().any(|a| a == "--dry-run")
        || std::env::var("ARCADER_INSTALL_DRY_RUN").is_ok();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Arcader Installer")
            .with_app_id("arcader-installer")
            .with_fullscreen(true)
            .with_decorations(false)
            .with_min_inner_size([900.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Arcader Installer",
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(InstallerApp::new(dry_run)))
        }),
    )
}
