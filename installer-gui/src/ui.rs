use eframe::egui::{
    self, Align, Align2, Color32, Id, Key, Layout, Modifiers, Rect, RichText, Sense, Stroke, Vec2,
};

use crate::disks::Disk;
use crate::net::Network;
use crate::theme;
use crate::{InstallerApp, NetStatus, Rotation, Step};

const CONTENT_W: f32 = 720.0;

#[derive(Clone, Copy)]
enum Dir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Default)]
pub(crate) struct Focus {
    idx: usize,
    nav: Vec<Rect>,
    building: Vec<Rect>,
    pending: bool,
    pointer_moved: bool,
    text_focus: bool,
}

impl Focus {
    fn begin(&mut self) {
        self.building.clear();
        self.text_focus = false;
    }

    fn end(&mut self) {
        self.nav = std::mem::take(&mut self.building);
        self.pending = false;
    }

    fn reset(&mut self) {
        self.idx = 0;
        self.pending = true;
    }

    fn step(&mut self, delta: i32) {
        let n = self.nav.len() as i32;
        if n == 0 {
            return;
        }
        let cur = (self.idx as i32).clamp(0, n - 1);
        self.idx = (cur + delta).rem_euclid(n) as usize;
        self.pending = true;
    }

    fn navigate(&mut self, dir: Dir) {
        let Some(cur) = self.nav.get(self.idx).copied() else {
            return;
        };
        let cc = cur.center();
        let mut best: Option<(usize, (f32, f32))> = None;
        for (i, r) in self.nav.iter().enumerate() {
            if i == self.idx {
                continue;
            }
            let c = r.center();
            let in_dir = match dir {
                Dir::Up => c.y < cc.y - 1.0,
                Dir::Down => c.y > cc.y + 1.0,
                Dir::Left => c.x < cc.x - 1.0,
                Dir::Right => c.x > cc.x + 1.0,
            };
            if !in_dir {
                continue;
            }
            let (primary, misalign) = match dir {
                Dir::Up | Dir::Down => {
                    let overlap = r.right().min(cur.right()) - r.left().max(cur.left());
                    let misalign = if overlap > 0.0 { 0.0 } else { (c.x - cc.x).abs() };
                    ((c.y - cc.y).abs(), misalign)
                }
                Dir::Left | Dir::Right => {
                    let overlap = r.bottom().min(cur.bottom()) - r.top().max(cur.top());
                    let misalign = if overlap > 0.0 { 0.0 } else { (c.y - cc.y).abs() };
                    ((c.x - cc.x).abs(), misalign)
                }
            };
            let key = (misalign, primary);
            if best.map_or(true, |(_, b)| key < b) {
                best = Some((i, key));
            }
        }
        if let Some((i, _)) = best {
            self.idx = i;
            self.pending = true;
        }
    }

    fn register(&mut self, ui: &egui::Ui, resp: egui::Response) -> egui::Response {
        let ord = self.building.len();
        self.building.push(resp.rect);
        if resp.gained_focus() {
            self.idx = ord;
        }
        if resp.hovered() && self.pointer_moved {
            self.idx = ord;
        }
        if ord == self.idx {
            if !resp.has_focus() {
                resp.request_focus();
            }
            if self.pending {
                resp.scroll_to_me(Some(Align::Center));
            }
            ui.painter().rect_stroke(
                resp.rect.expand(3.0),
                theme::SQUARE,
                Stroke::new(3.0, theme::ACCENT_HI),
            );
        }
        resp
    }
}

pub fn draw(app: &mut InstallerApp, ctx: &egui::Context) {
    handle_keys(app, ctx);

    if app.step != app.last_step {
        app.focus.reset();
        app.last_step = app.step;
        if app.step == Step::Keyboard {
            app.set_keyboard(app.keyboard);
        }
        if app.step == Step::Network {
            app.start_scan();
        }
    }
    app.focus.pointer_moved = ctx.input(|i| i.pointer.delta() != Vec2::ZERO);
    app.focus.begin();

    let screen = ctx.screen_rect();
    let background = app.textures(ctx).background.clone();

    let painter = ctx.layer_painter(egui::LayerId::background());
    draw_cover(&painter, &background, screen);
    painter.rect_filled(screen, 0.0, Color32::from_rgba_premultiplied(12, 6, 6, 205));
    theme::draw_scanlines(&painter, screen);

    let w = CONTENT_W.min(screen.width() - 64.0);
    let max_h = screen.height() - 140.0;
    egui::Area::new(Id::new("arcader-wizard"))
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .movable(false)
        .show(ctx, |ui| {
            egui::Frame::none()
                .fill(theme::PANEL)
                .stroke(Stroke::new(2.0, theme::ACCENT))
                .rounding(theme::SQUARE)
                .inner_margin(egui::Margin::same(36.0))
                .show(ui, |ui| {
                    ui.set_width(w);
                    ui.set_max_height(max_h);
                    ui.vertical_centered(|ui| step_indicator(ui, app.step));
                    ui.add_space(18.0);
                    match app.step {
                        Step::Welcome => welcome(app, ui),
                        Step::SelectDisk => select_disk(app, ui),
                        Step::Display => display(app, ui),
                        Step::Keyboard => keyboard(app, ui),
                        Step::Network => network(app, ui),
                        Step::Confirm => confirm(app, ui),
                        Step::Installing => installing(app, ui),
                        Step::Done => done(app, ui),
                        Step::Failed => failed(app, ui),
                    }
                });
        });

    app.focus.end();
}

fn step_indicator(ui: &mut egui::Ui, current: Step) {
    use egui_phosphor::regular;
    let active = match current {
        Step::Done | Step::Failed => Step::Installing,
        other => other,
    };
    let active_idx = Step::INDICATOR
        .iter()
        .position(|(_, s)| *s == active)
        .unwrap_or(0);

    ui.horizontal(|ui| {
        for (i, (label, _)) in Step::INDICATOR.iter().enumerate() {
            let is_active = i == active_idx;
            let (fg, marker) = if is_active {
                (theme::ACCENT_HI, regular::CIRCLE)
            } else if i < active_idx {
                (theme::ACCENT, regular::CHECK_CIRCLE)
            } else {
                (theme::MUTED, regular::CIRCLE)
            };
            ui.label(RichText::new(marker).color(fg).size(16.0).strong());
            ui.label(
                RichText::new(label.to_uppercase())
                    .color(if is_active { theme::TEXT } else { theme::MUTED })
                    .size(14.0)
                    .strong(),
            );
            if i + 1 < Step::INDICATOR.len() {
                ui.label(RichText::new("──").color(theme::MUTED));
            }
        }
    });
}

fn welcome(app: &mut InstallerApp, ui: &mut egui::Ui) {
    let banner = app.textures(ui.ctx()).banner.clone();
    ui.vertical_centered(|ui| {
        let size = banner.size_vec2();
        let scale = (ui.available_width().min(620.0) / size.x).min(1.0);
        ui.add_space(10.0);
        ui.image((banner.id(), size * scale));
        ui.add_space(22.0);
        ui.label(RichText::new("Welcome to the Arcader installer").size(26.0).strong());
        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "This assistant installs the Arcader kiosk system onto an internal disk. \
                 Once finished, the machine boots straight into arcade mode and the USB \
                 stick is no longer needed.",
            )
            .size(16.0)
            .color(theme::MUTED),
        );
        if app.dry_run {
            ui.add_space(10.0);
            ui.label(RichText::new("Dry-run mode - no disk will be modified.").color(theme::ACCENT));
        }
        ui.add_space(26.0);
        let start = primary_button(ui, "Start installation");
        if app.focus.register(ui, start).clicked() {
            app.step = Step::SelectDisk;
        }
    });
    keyboard_hint(ui, "Enter - continue");
}

fn select_disk(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Select installation disk").size(24.0).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new("Choose the internal disk to install onto. It will be erased.")
            .color(theme::MUTED),
    );
    ui.add_space(16.0);

    if let Some(err) = &app.disk_error {
        ui.colored_label(theme::DANGER, format!("Could not list disks: {err}"));
        ui.add_space(10.0);
        let retry = secondary_button(ui, "Retry");
        if app.focus.register(ui, retry).clicked() {
            app.refresh_disks();
        }
        return;
    }

    if app.disks.is_empty() {
        ui.label(RichText::new("No disks detected.").color(theme::MUTED));
    }

    if app.disks.len() > 7 {
        let list_h = (ui.available_height() - 132.0).max(200.0);
        egui::ScrollArea::vertical()
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| disk_list(app, ui));
    } else {
        disk_list(app, ui);
    }

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back");
        if app.focus.register(ui, back).clicked() {
            app.step = Step::Welcome;
        }
        let refresh = secondary_button(ui, "Refresh");
        if app.focus.register(ui, refresh).clicked() {
            app.refresh_disks();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let enabled = app.selected_disk().is_some();
            let cont = primary_button_enabled(ui, "Continue", enabled);
            if app.focus.register(ui, cont).clicked() {
                app.step = Step::Display;
            }
        });
    });
    keyboard_hint(ui, "↑ ↓ ← → / Tab  move    Enter / Space  select    Esc  back");
}

fn disk_list(app: &mut InstallerApp, ui: &mut egui::Ui) {
    for i in 0..app.disks.len() {
        let disk = app.disks[i].clone();
        disk_card(app, ui, i, &disk);
        ui.add_space(12.0);
    }
}

fn disk_card(app: &mut InstallerApp, ui: &mut egui::Ui, index: usize, disk: &Disk) {
    let selected = index == app.selected && !disk.is_live;
    let (fill, stroke) = card_colors(selected);
    let tint = if disk.is_live { theme::MUTED } else { theme::ACCENT };

    let inner = egui::Frame::none()
        .fill(fill)
        .rounding(theme::SQUARE)
        .inner_margin(egui::Margin::same(16.0))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(theme::icon_for(disk.kind())).size(38.0).color(tint));
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("/dev/{}", disk.name)).size(19.0).strong());
                        badge(ui, &disk.transport_badge(), theme::ACCENT);
                        if disk.is_live {
                            badge(ui, "LIVE USB - EXCLUDED", theme::DANGER);
                        }
                    });
                    let model = if disk.model.is_empty() { "Unknown model" } else { &disk.model };
                    ui.label(RichText::new(model).color(theme::MUTED).size(14.0));
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(&disk.size).size(20.0).strong());
                });
            });
        });

    if !disk.is_live {
        let id = ui.make_persistent_id(("disk-row", index));
        let resp = ui
            .interact(inner.response.rect, id, Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        let resp = app.focus.register(ui, resp);
        if resp.clicked() {
            app.selected = index;
        }
        if resp.double_clicked() {
            app.selected = index;
            app.step = Step::Display;
        }
    }
}

fn display(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Screen orientation").size(24.0).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new("Choose how the picture is rotated on the cabinet's monitor.")
            .color(theme::MUTED),
    );
    ui.add_space(18.0);

    let tw = (ui.available_width() - 12.0) * 0.5;
    ui.horizontal(|ui| {
        for rot in Rotation::ALL {
            orientation_tile(app, ui, rot, tw);
        }
    });
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back");
        if app.focus.register(ui, back).clicked() {
            app.step = Step::SelectDisk;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let cont = primary_button(ui, "Continue");
            if app.focus.register(ui, cont).clicked() {
                app.step = Step::Keyboard;
            }
        });
    });
    keyboard_hint(ui, "↑ ↓ ← → / Tab  move    Enter / Space  select    Esc  back");
}

fn orientation_tile(app: &mut InstallerApp, ui: &mut egui::Ui, rot: Rotation, width: f32) {
    let selected = app.rotation == rot;
    let (fill, stroke) = card_colors(selected);
    let tint = if selected { theme::ACCENT } else { theme::MUTED };

    let inner = egui::Frame::none()
        .fill(fill)
        .rounding(theme::SQUARE)
        .inner_margin(egui::Margin::same(14.0))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(width);
            ui.vertical_centered(|ui| {
                let (rect, _) = ui.allocate_exact_size(Vec2::new(140.0, 96.0), Sense::hover());
                draw_monitor(ui.painter(), rect, rot, tint);
                ui.add_space(8.0);
                ui.label(
                    RichText::new(rot.label())
                        .size(15.0)
                        .strong()
                        .color(if selected { theme::TEXT } else { theme::MUTED }),
                );
            });
        });

    let id = ui.make_persistent_id(("orient", rot.xrandr()));
    let resp = ui
        .interact(inner.response.rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let resp = app.focus.register(ui, resp);
    if resp.clicked() {
        app.rotation = rot;
    }
    if resp.double_clicked() {
        app.rotation = rot;
        app.step = Step::Keyboard;
    }
}

fn keyboard(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Keyboard layout").size(24.0).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new("Pick the layout that matches the keyboard wired to the cabinet.")
            .color(theme::MUTED),
    );
    ui.add_space(16.0);

    let list_h = (ui.available_height() - 120.0).max(200.0);
    egui::ScrollArea::vertical()
        .max_height(list_h)
        .auto_shrink([false, false])
        .show(ui, |ui| keyboard_list(app, ui));

    ui.add_space(12.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back");
        if app.focus.register(ui, back).clicked() {
            app.step = Step::Display;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let cont = primary_button(ui, "Continue");
            if app.focus.register(ui, cont).clicked() {
                app.step = Step::Network;
            }
        });
    });
    keyboard_hint(ui, "↑ ↓ ← → / Tab  move    Enter / Space  select    Esc  back");
}

fn keyboard_list(app: &mut InstallerApp, ui: &mut egui::Ui) {
    for i in 0..crate::LAYOUTS.len() {
        keyboard_row(app, ui, i);
        ui.add_space(8.0);
    }
}

fn keyboard_row(app: &mut InstallerApp, ui: &mut egui::Ui, index: usize) {
    let layout = crate::LAYOUTS[index];
    let selected = app.keyboard == index;
    let (fill, stroke) = card_colors(selected);
    let tint = if selected { theme::ACCENT } else { theme::MUTED };

    let inner = egui::Frame::none()
        .fill(fill)
        .rounding(theme::SQUARE)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(egui_phosphor::regular::KEYBOARD).size(22.0).color(tint));
                ui.add_space(10.0);
                ui.label(RichText::new(layout.label).size(16.0).strong());
                if selected {
                    badge(ui, "SELECTED", theme::ACCENT);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(layout.code.to_uppercase()).size(14.0).color(theme::MUTED));
                });
            });
        });

    let id = ui.make_persistent_id(("kbd-row", index));
    let resp = ui
        .interact(inner.response.rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let resp = app.focus.register(ui, resp);
    if resp.clicked() {
        app.set_keyboard(index);
    }
    if resp.double_clicked() {
        app.set_keyboard(index);
        app.step = Step::Network;
    }
}

fn draw_monitor(painter: &egui::Painter, area: Rect, rot: Rotation, tint: Color32) {
    let screen = Rect::from_center_size(
        area.center(),
        Vec2::new(area.width(), area.width() * 0.62),
    );
    painter.rect(screen, theme::SQUARE, theme::PANEL, Stroke::new(2.0, tint));

    let t = 5.0;
    let bar = match rot {
        Rotation::Normal => {
            Rect::from_min_max(screen.left_top(), egui::pos2(screen.right(), screen.top() + t))
        }
        Rotation::Inverted => Rect::from_min_max(
            egui::pos2(screen.left(), screen.bottom() - t),
            screen.right_bottom(),
        ),
    };
    painter.rect_filled(bar, theme::SQUARE, tint);
}

fn network(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.label(RichText::new("Wi-Fi").size(24.0).strong());
    ui.add_space(4.0);
    ui.label(
        RichText::new("Optionally connect so the cabinet can fetch updates. You can skip this.")
            .color(theme::MUTED),
    );
    ui.add_space(12.0);

    match &app.net_status {
        NetStatus::Scanning => status_line(ui, egui_phosphor::regular::MAGNIFYING_GLASS, "Scanning…", theme::MUTED),
        NetStatus::Connecting => status_line(ui, egui_phosphor::regular::SPINNER_GAP, "Connecting…", theme::AMBER),
        NetStatus::Connected(ssid) => {
            status_line(ui, egui_phosphor::regular::CHECK_CIRCLE, &format!("Connected to {ssid}"), theme::ACCENT)
        }
        NetStatus::Failed(e) => status_line(ui, egui_phosphor::regular::WARNING, e, theme::AMBER),
        NetStatus::Idle => {}
    }
    ui.add_space(8.0);

    if app.networks.is_empty() && app.scan_job.is_none() {
        ui.label(RichText::new("No networks found.").color(theme::MUTED));
    }

    if app.networks.len() > 6 {
        let h = (ui.available_height() - 180.0).max(160.0);
        egui::ScrollArea::vertical()
            .max_height(h)
            .auto_shrink([false, false])
            .show(ui, |ui| network_list(app, ui));
    } else {
        network_list(app, ui);
    }

    if let Some(net) = app.net_selected.and_then(|i| app.networks.get(i)).cloned() {
        if net.secured && !matches!(app.net_status, NetStatus::Connected(_)) {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Password").color(theme::MUTED).size(14.0));
                let edit = egui::TextEdit::singleline(&mut app.net_password)
                    .password(false)
                    .desired_width(320.0)
                    .hint_text("network password");
                let resp = ui.add(edit);
                let submitted = resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                app.focus.text_focus |= resp.has_focus();
                app.focus.register(ui, resp);
                let connect = primary_button(ui, "Connect");
                if app.focus.register(ui, connect).clicked() || submitted {
                    app.start_connect();
                }
            });
        } else if !net.secured && !matches!(app.net_status, NetStatus::Connected(_)) {
            ui.add_space(8.0);
            let connect = primary_button(ui, &format!("Connect to {}", net.ssid));
            if app.focus.register(ui, connect).clicked() {
                app.start_connect();
            }
        }
    }

    ui.add_space(14.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back");
        if app.focus.register(ui, back).clicked() {
            app.step = Step::Keyboard;
        }
        let rescan = secondary_button(ui, "Rescan");
        if app.focus.register(ui, rescan).clicked() {
            app.start_scan();
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if app.net_connected.is_some() { "Continue" } else { "Skip" };
            let cont = primary_button(ui, label);
            if app.focus.register(ui, cont).clicked() {
                app.step = Step::Confirm;
            }
        });
    });
    keyboard_hint(ui, "↑ ↓ ← → / Tab  move    Enter / Space  select    Esc  back");
}

fn network_list(app: &mut InstallerApp, ui: &mut egui::Ui) {
    for i in 0..app.networks.len() {
        let net = app.networks[i].clone();
        network_row(app, ui, i, &net);
        ui.add_space(8.0);
    }
}

fn network_row(app: &mut InstallerApp, ui: &mut egui::Ui, index: usize, net: &Network) {
    let selected = app.net_selected == Some(index);
    let (fill, stroke) = card_colors(selected);
    let tint = if selected { theme::ACCENT } else { theme::MUTED };

    let inner = egui::Frame::none()
        .fill(fill)
        .rounding(theme::SQUARE)
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .stroke(stroke)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(RichText::new(signal_icon(net.signal)).size(22.0).color(tint));
                ui.add_space(10.0);
                ui.label(RichText::new(&net.ssid).size(16.0).strong());
                if net.secured {
                    ui.label(RichText::new(egui_phosphor::regular::LOCK).size(14.0).color(theme::MUTED));
                }
                if net.active {
                    badge(ui, "CONNECTED", theme::ACCENT);
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(format!("{}%", net.signal)).size(14.0).color(theme::MUTED));
                });
            });
        });

    let id = ui.make_persistent_id(("wifi-row", index));
    let resp = ui
        .interact(inner.response.rect, id, Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    let resp = app.focus.register(ui, resp);
    if resp.clicked() {
        if app.net_selected != Some(index) {
            app.net_password.clear();
        }
        app.net_selected = Some(index);
    }
}

fn signal_icon(signal: u8) -> &'static str {
    use egui_phosphor::regular;
    match signal {
        67..=100 => regular::WIFI_HIGH,
        34..=66 => regular::WIFI_MEDIUM,
        1..=33 => regular::WIFI_LOW,
        _ => regular::WIFI_NONE,
    }
}

fn status_line(ui: &mut egui::Ui, icon: &str, text: &str, color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(16.0).color(color));
        ui.add_space(4.0);
        ui.label(RichText::new(text).size(14.0).color(color));
    });
}

fn confirm(app: &mut InstallerApp, ui: &mut egui::Ui) {
    let Some(disk) = app.selected_disk().cloned() else {
        app.step = Step::SelectDisk;
        return;
    };
    let firmware = if std::path::Path::new("/sys/firmware/efi").exists() {
        "UEFI"
    } else {
        "BIOS"
    };

    ui.label(RichText::new("Confirm installation").size(24.0).strong());
    ui.add_space(12.0);
    ui.colored_label(
        theme::AMBER,
        RichText::new(format!(
            "{}  All data on /dev/{} will be permanently erased.",
            egui_phosphor::regular::WARNING,
            disk.name
        ))
        .size(17.0)
        .strong(),
    );
    ui.add_space(18.0);

    let wifi = app.net_connected.clone().unwrap_or_else(|| "Not configured".to_string());
    summary_row(ui, "Target disk", &format!("/dev/{}  ({})", disk.name, disk.size));
    summary_row(ui, "Model", if disk.model.is_empty() { "Unknown" } else { &disk.model });
    summary_row(ui, "Orientation", app.rotation.label());
    summary_row(ui, "Keyboard", app.keyboard_label());
    summary_row(ui, "Wi-Fi", &wifi);
    summary_row(ui, "Architecture", std::env::consts::ARCH);
    summary_row(ui, "Firmware", firmware);

    ui.add_space(18.0);
    let check = ui.checkbox(
        &mut app.confirm_ack,
        RichText::new(format!("I understand this will erase /dev/{} and install Arcader.", disk.name))
            .size(15.0),
    );
    app.focus.register(ui, check);

    ui.add_space(20.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back");
        if app.focus.register(ui, back).clicked() {
            app.step = Step::Network;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let erase = danger_button_enabled(ui, "Erase disk & install", app.confirm_ack);
            if app.focus.register(ui, erase).clicked() {
                app.begin_install();
            }
        });
    });
    keyboard_hint(ui, "↑ ↓ ← → / Tab  move    Space  toggle    Enter  install    Esc  back");
}

fn installing(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.add(egui::Spinner::new().size(22.0).color(theme::ACCENT));
        ui.add_space(6.0);
        ui.label(RichText::new("Installing Arcader…").size(24.0).strong());
    });
    ui.add_space(6.0);
    let phase = if app.phase.is_empty() { "Preparing…" } else { &app.phase };
    ui.label(RichText::new(phase).color(theme::MUTED).size(15.0));
    ui.add_space(14.0);

    ui.add(
        egui::ProgressBar::new(app.progress)
            .desired_height(16.0)
            .fill(theme::ACCENT)
            .rounding(theme::SQUARE)
            .text(format!("{:.0}%", app.progress * 100.0)),
    );
    ui.add_space(16.0);

    ui.label(RichText::new("Log").color(theme::MUTED).size(13.0));
    log_view(ui, &app.log, 300.0);
    ui.add_space(6.0);
    ui.label(
        RichText::new("Do not power off or remove the USB stick during installation.")
            .color(theme::MUTED)
            .italics(),
    );
}

fn done(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(6.0);
        ui.label(RichText::new(egui_phosphor::regular::CHECK_CIRCLE).size(72.0).color(theme::ACCENT));
        ui.add_space(6.0);
        ui.label(RichText::new("INSTALLATION COMPLETE").size(26.0).strong());
        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "Remove the USB stick, then reboot. The machine will start directly into Arcader.",
            )
            .size(16.0)
            .color(theme::MUTED),
        );
        if app.dry_run {
            ui.add_space(8.0);
            ui.label(RichText::new("Dry run - nothing was actually written.").color(theme::ACCENT));
        }
        ui.add_space(24.0);
        let reboot_btn = primary_button(ui, "Reboot now");
        if app.focus.register(ui, reboot_btn).clicked() {
            reboot();
        }
    });
    keyboard_hint(ui, "Enter - reboot");
}

fn failed(app: &mut InstallerApp, ui: &mut egui::Ui) {
    ui.label(
        RichText::new(format!("{}  Installation failed", egui_phosphor::regular::X_CIRCLE))
            .size(24.0)
            .color(theme::DANGER)
            .strong(),
    );
    ui.add_space(10.0);
    if let Some(reason) = &app.failed_reason {
        ui.colored_label(theme::DANGER, reason);
    }
    ui.add_space(14.0);
    ui.label(RichText::new("Log").color(theme::MUTED).size(13.0));
    log_view(ui, &app.log, 300.0);
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        let back = secondary_button(ui, "Back to disk selection");
        if app.focus.register(ui, back).clicked() {
            app.refresh_disks();
            app.confirm_ack = false;
            app.step = Step::SelectDisk;
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let reboot_btn = secondary_button(ui, "Reboot");
            if app.focus.register(ui, reboot_btn).clicked() {
                reboot();
            }
        });
    });
}

fn card_colors(selected: bool) -> (Color32, Stroke) {
    if selected {
        (theme::CARD_SELECTED, Stroke::new(2.0, theme::ACCENT))
    } else {
        (theme::CARD, Stroke::new(1.0, Color32::from_rgba_premultiplied(255, 255, 255, 18)))
    }
}

fn summary_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(theme::MUTED).size(15.0));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(value).size(15.0).strong());
        });
    });
}

fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::none()
        .fill(Color32::TRANSPARENT)
        .rounding(theme::SQUARE)
        .stroke(Stroke::new(1.0, color))
        .inner_margin(egui::Margin::symmetric(8.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).size(11.0).strong());
        });
}

fn log_view(ui: &mut egui::Ui, log: &[String], height: f32) {
    egui::Frame::none()
        .fill(Color32::from_rgba_premultiplied(0, 0, 0, 140))
        .rounding(theme::SQUARE)
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::ScrollArea::vertical()
                .max_height(height)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in log {
                        ui.label(RichText::new(line).monospace().size(12.5).color(theme::MUTED));
                    }
                });
        });
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    primary_button_enabled(ui, text, true)
}

fn primary_button_enabled(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).size(16.0).strong().color(Color32::BLACK))
        .fill(theme::ACCENT)
        .rounding(theme::SQUARE)
        .min_size(Vec2::new(180.0, 44.0));
    ui.add_enabled(enabled, btn)
}

fn secondary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).size(15.0).color(theme::TEXT))
        .fill(theme::CARD)
        .stroke(Stroke::new(1.0, theme::MUTED.linear_multiply(0.6)))
        .rounding(theme::SQUARE)
        .min_size(Vec2::new(120.0, 44.0));
    ui.add(btn)
}

fn danger_button_enabled(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).size(16.0).strong().color(Color32::WHITE))
        .fill(theme::DANGER)
        .rounding(theme::SQUARE)
        .min_size(Vec2::new(220.0, 44.0));
    ui.add_enabled(enabled, btn)
}

fn keyboard_hint(ui: &mut egui::Ui, text: &str) {
    ui.add_space(14.0);
    ui.vertical_centered(|ui| {
        ui.label(RichText::new(text).color(theme::MUTED.linear_multiply(0.8)).size(13.0));
    });
}

fn handle_keys(app: &mut InstallerApp, ctx: &egui::Context) {
    if app.step == Step::Installing {
        return;
    }

    let (tab, shift_tab, esc) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::Tab),
            i.consume_key(Modifiers::SHIFT, Key::Tab),
            i.key_pressed(Key::Escape),
        )
    });

    if tab {
        app.focus.step(1);
    }
    if shift_tab {
        app.focus.step(-1);
    }

    if !app.focus.text_focus {
        let (up, down, left, right) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::NONE, Key::ArrowUp),
                i.consume_key(Modifiers::NONE, Key::ArrowDown),
                i.consume_key(Modifiers::NONE, Key::ArrowLeft),
                i.consume_key(Modifiers::NONE, Key::ArrowRight),
            )
        });
        if up {
            app.focus.navigate(Dir::Up);
        }
        if down {
            app.focus.navigate(Dir::Down);
        }
        if left {
            app.focus.navigate(Dir::Left);
        }
        if right {
            app.focus.navigate(Dir::Right);
        }
    }

    if esc {
        match app.step {
            Step::SelectDisk => app.step = Step::Welcome,
            Step::Display => app.step = Step::SelectDisk,
            Step::Keyboard => app.step = Step::Display,
            Step::Network => app.step = Step::Keyboard,
            Step::Confirm => app.step = Step::Network,
            Step::Failed => {
                app.refresh_disks();
                app.confirm_ack = false;
                app.step = Step::SelectDisk;
            }
            _ => {}
        }
    }
}

fn reboot() {
    let _ = std::process::Command::new("sudo")
        .args(["-n", "systemctl", "reboot"])
        .spawn();
}

fn draw_cover(painter: &egui::Painter, texture: &egui::TextureHandle, rect: Rect) {
    let img = texture.size_vec2();
    if img.x <= 0.0 || img.y <= 0.0 {
        return;
    }
    let img_aspect = img.x / img.y;
    let rect_aspect = rect.width() / rect.height();
    let (uv_w, uv_h) = if rect_aspect > img_aspect {
        (1.0, img_aspect / rect_aspect)
    } else {
        (rect_aspect / img_aspect, 1.0)
    };
    let uv = Rect::from_min_max(
        egui::pos2((1.0 - uv_w) * 0.5, (1.0 - uv_h) * 0.5),
        egui::pos2(1.0 - (1.0 - uv_w) * 0.5, 1.0 - (1.0 - uv_h) * 0.5),
    );
    painter.image(texture.id(), rect, uv, Color32::WHITE);
}
