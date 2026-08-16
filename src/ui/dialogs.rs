//! Modal-ish dialogs: the apply-confirmation countdown and the fatal-error screen.

use super::App;
use crate::application::session::Session;
use egui::Align2;
use std::time::Instant;

impl App {
    pub(super) fn draw_confirm_dialog(&mut self, ctx: &egui::Context, now: Instant) {
        let pending = self.session.as_ref().is_some_and(Session::confirm_pending);
        if !pending {
            return;
        }
        let remaining = self
            .session
            .as_ref()
            .map(|s| s.confirm_remaining(now).as_secs())
            .unwrap_or(0);
        let mut keep = false;
        let mut revert = false;
        egui::Window::new("Keep these settings?")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!(
                    "The layout was applied. Without confirmation it reverts in {remaining} s."
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    keep = ui.button("✅ Keep & save").clicked();
                    revert = ui.button("↩ Revert now").clicked();
                });
            });
        if keep && let Some(s) = self.session.as_mut() {
            match s.keep() {
                Ok(msg) => self.set_status(msg, false),
                Err(e) => self.set_status(
                    format!("Layout kept in the session, but saving failed: {e}"),
                    true,
                ),
            }
        }
        if revert && let Some(s) = self.session.as_mut() {
            let msg = s.revert_now();
            self.set_status(msg, false);
        }
    }
}

pub(super) fn draw_fatal(ui: &mut egui::Ui, msg: &str) {
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.heading("Could not connect to Hyprland");
            ui.add_space(12.0);
            ui.label(msg);
            ui.add_space(20.0);
            if ui.button("Close").clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    });
}
