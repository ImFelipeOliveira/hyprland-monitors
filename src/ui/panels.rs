//! Toolbar (apply/reload), per-monitor settings panel, and the status bar.

use super::App;
use crate::application::session::Session;
use crate::domain::monitor::format_scale;
use egui::Color32;
use std::time::Instant;

impl App {
    pub(super) fn draw_top_bar(&mut self, ui: &mut egui::Ui, now: Instant) {
        let mut apply_clicked = false;
        let mut reload_clicked = false;
        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Monitors");
                ui.separator();
                let pending = self.session.as_ref().is_some_and(Session::confirm_pending);
                ui.add_enabled_ui(!pending, |ui| {
                    apply_clicked = ui
                        .button("✅ Apply")
                        .on_hover_text(
                            "Applies live; you have 15s to confirm before the automatic revert",
                        )
                        .clicked();
                    reload_clicked = ui
                        .button("🔄 Reload")
                        .on_hover_text("Discards edits and re-reads the current Hyprland state")
                        .clicked();
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new("drag monitors to position them")
                            .weak()
                            .italics(),
                    );
                });
            });
        });
        if apply_clicked && let Some(s) = self.session.as_mut() {
            match s.apply(now) {
                Ok(()) => self.set_status("Layout applied — confirm to keep it.", false),
                Err(e) => self.set_status(e, true),
            }
        }
        if reload_clicked && let Some(s) = self.session.as_mut() {
            s.resync();
            self.drag = None;
            self.selected = Some(0);
            self.set_status("State re-read from Hyprland.", false);
        }
    }

    pub(super) fn draw_settings_panel(&mut self, ui: &mut egui::Ui) {
        let mut status: Option<(String, bool)> = None;
        egui::Panel::right("settings")
            .min_size(260.0)
            .show(ui, |ui| {
                let Some(s) = self.session.as_mut() else {
                    return;
                };
                let Some(i) = self.selected.filter(|&i| i < s.monitors.len()) else {
                    ui.label("Select a monitor on the canvas.");
                    return;
                };
                let (name, desc, modes, cur_mode, cur_scale, enabled) = {
                    let m = &s.monitors[i];
                    (
                        m.name.clone(),
                        m.description.clone(),
                        m.modes.clone(),
                        m.mode,
                        m.scale,
                        m.enabled,
                    )
                };
                ui.add_space(6.0);
                ui.heading(&name);
                ui.label(egui::RichText::new(&desc).weak());
                ui.separator();

                let mut en = enabled;
                if ui.checkbox(&mut en, "Enabled").changed()
                    && let Err(e) = s.set_enabled(i, en)
                {
                    status = Some((e, true));
                }

                ui.add_space(4.0);
                ui.label("Resolution & refresh rate:");
                let mut sel_mode = cur_mode;
                egui::ComboBox::from_id_salt("mode")
                    .width(220.0)
                    .selected_text(sel_mode.to_string())
                    .show_ui(ui, |ui| {
                        for m in &modes {
                            ui.selectable_value(&mut sel_mode, *m, m.to_string());
                        }
                    });
                if sel_mode != cur_mode {
                    s.set_mode(i, sel_mode);
                }

                ui.add_space(4.0);
                ui.label("Scale:");
                let mut scale = cur_scale;
                egui::ComboBox::from_id_salt("scale")
                    .width(220.0)
                    .selected_text(format_scale(scale))
                    .show_ui(ui, |ui| {
                        for v in [1.0_f32, 1.25, 1.5, 1.6, 1.75, 2.0] {
                            ui.selectable_value(&mut scale, v, format_scale(v));
                        }
                    });
                if (scale - cur_scale).abs() > f32::EPSILON {
                    s.set_scale(i, scale);
                }

                ui.add_space(8.0);
                ui.separator();
                let m = &s.monitors[i];
                ui.label(format!("Position: {}x{}", m.pos.0, m.pos.1));
                let (lw, lh) = m.logical_size();
                ui.label(format!("Logical size: {lw}x{lh}"));
            });
        if let Some((msg, err)) = status {
            self.set_status(msg, err);
        }
    }

    pub(super) fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status").show(ui, |ui| {
            // Compositor errors can be long — always wrap so no message is cut off.
            let text = match &self.status {
                Some((msg, true)) => {
                    egui::RichText::new(format!("⚠ {msg}")).color(Color32::from_rgb(220, 80, 80))
                }
                Some((msg, false)) => egui::RichText::new(msg),
                None => egui::RichText::new("Ready.").weak(),
            };
            ui.add(egui::Label::new(text).wrap());
        });
    }
}
