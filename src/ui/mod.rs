//! Thin egui view over `application::session::Session`. No business logic here:
//! `canvas` renders and drags, `panels` hosts the toolbar/settings/status bar,
//! `dialogs` the confirm-countdown and fatal-error screens.

mod canvas;
mod dialogs;
mod panels;

use crate::adapters::hyprctl::{self, ConfigProvider, EvalCompositor, KeywordCompositor};
use crate::adapters::hyprland_conf::ConfConfigStore;
use crate::adapters::omarchy_lua::FileConfigStore;
use crate::adapters::profiles_json::JsonProfileStore;
use crate::application::ports::{Compositor, ConfigStore};
use crate::application::session::Session;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

type Adapters = (Box<dyn Compositor>, Box<dyn ConfigStore>);

pub struct App {
    session: Option<Session>,
    fatal: Option<String>,
    selected: Option<usize>,
    drag: Option<canvas::DragState>,
    status: Option<(String, bool)>, // (message, is_error)
    hotplug_dirty: Arc<AtomicBool>,
    profile_name_input: String,
    selected_profile: Option<String>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let dirty = Arc::new(AtomicBool::new(false));
        let (comp, store) = match Self::adapters_for_provider() {
            Ok(pair) => pair,
            Err(e) => return App::fatal_app(e, dirty),
        };
        let profiles = match JsonProfileStore::default_path() {
            Ok(path) => JsonProfileStore { path },
            Err(e) => return App::fatal_app(e, dirty),
        };
        match Session::new(comp, store, Box::new(profiles)) {
            Ok(session) => {
                let ctx = cc.egui_ctx.clone();
                let flag = dirty.clone();
                hyprctl::spawn_event_listener(move || {
                    flag.store(true, Ordering::SeqCst);
                    ctx.request_repaint();
                });
                App {
                    session: Some(session),
                    fatal: None,
                    selected: Some(0),
                    drag: None,
                    status: None,
                    hotplug_dirty: dirty,
                    profile_name_input: String::new(),
                    selected_profile: None,
                }
            }
            Err(e) => App::fatal_app(e, dirty),
        }
    }

    /// Pick the adapters matching the running compositor's config provider.
    fn adapters_for_provider() -> Result<Adapters, String> {
        match hyprctl::detect_provider() {
            ConfigProvider::Lua => {
                let path = FileConfigStore::default_path()?;
                Ok((Box::new(EvalCompositor), Box::new(FileConfigStore { path })))
            }
            ConfigProvider::Classic => {
                let path = ConfConfigStore::default_path()?;
                Ok((
                    Box::new(KeywordCompositor),
                    Box::new(ConfConfigStore { path }),
                ))
            }
        }
    }

    fn fatal_app(message: String, dirty: Arc<AtomicBool>) -> App {
        App {
            session: None,
            fatal: Some(message),
            selected: None,
            drag: None,
            status: None,
            hotplug_dirty: dirty,
            profile_name_input: String::new(),
            selected_profile: None,
        }
    }

    fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if let Some(msg) = self.fatal.clone() {
            dialogs::draw_fatal(ui, &msg);
            return;
        }
        let now = Instant::now();

        // Hotplug: re-query unless a confirm countdown is in flight.
        if self.hotplug_dirty.swap(false, Ordering::SeqCst)
            && let Some(s) = self.session.as_mut()
            && !s.confirm_pending()
        {
            match s.refresh_preserving_edits() {
                Ok(()) => self.set_status("Monitor list refreshed (hotplug detected).", false),
                Err(e) => self.set_status(e, true),
            }
            self.selected = Some(0);
            self.drag = None;
        }

        // Countdown auto-revert.
        if let Some(msg) = self.session.as_mut().and_then(|s| s.tick(now)) {
            self.set_status(msg, true);
        }

        self.draw_top_bar(ui, now);
        self.draw_settings_panel(ui);
        self.draw_status_bar(ui);
        self.draw_canvas(ui);
        self.draw_confirm_dialog(&ui.ctx().clone(), now);

        if self.session.as_ref().is_some_and(Session::confirm_pending) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}
