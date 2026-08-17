//! Application service: owns the candidate layout and the apply → confirm →
//! keep/revert → persist state machine. Depends only on the `Compositor` and
//! `ConfigStore` ports, so the whole flow is unit-tested with fakes.

use crate::application::ports::{Compositor, ConfigStore, ProfileStore};
use crate::application::profiles::{Profile, ProfileMonitor};
use crate::domain::geometry::{Rect, normalize_offset, resolve_overlap};
use crate::domain::monitor::{Mode, MonitorState};
use std::time::{Duration, Instant};

pub const CONFIRM_SECS: u64 = 15;

struct PendingConfirm {
    deadline: Instant,
    /// The last known-good layout, replayed to restore the previous state.
    revert_layout: Vec<MonitorState>,
}

pub struct Session {
    pub monitors: Vec<MonitorState>,
    applied_snapshot: Vec<MonitorState>,
    confirm: Option<PendingConfirm>,
    comp: Box<dyn Compositor>,
    store: Box<dyn ConfigStore>,
    profiles: Box<dyn ProfileStore>,
}

impl Session {
    pub fn new(
        comp: Box<dyn Compositor>,
        store: Box<dyn ConfigStore>,
        profiles: Box<dyn ProfileStore>,
    ) -> Result<Session, String> {
        let raws = comp.query()?;
        if raws.is_empty() {
            return Err("Hyprland reported no monitors".into());
        }
        let monitors: Vec<MonitorState> = raws.iter().map(MonitorState::from_raw).collect();
        let applied_snapshot = monitors.clone();
        Ok(Session {
            monitors,
            applied_snapshot,
            confirm: None,
            comp,
            store,
            profiles,
        })
    }

    // ----- candidate editing -----

    pub fn logical_rect(&self, i: usize) -> Rect {
        let m = &self.monitors[i];
        let (w, h) = m.logical_size();
        Rect::new(m.pos.0, m.pos.1, w, h)
    }

    /// Rects of every *other* enabled monitor — the snap/overlap obstacles for `i`.
    pub fn obstacle_rects(&self, i: usize) -> Vec<Rect> {
        (0..self.monitors.len())
            .filter(|&j| j != i && self.monitors[j].enabled)
            .map(|j| self.logical_rect(j))
            .collect()
    }

    /// Commit a drop at `pos`: resolve overlaps, then shift everything so the
    /// enabled bounding box starts at 0x0.
    pub fn finalize_drop(&mut self, i: usize, pos: (i32, i32)) {
        let (w, h) = self.monitors[i].logical_size();
        let others = self.obstacle_rects(i);
        let dropped = Rect::new(pos.0, pos.1, w, h);
        self.monitors[i].pos = resolve_overlap(dropped, &others);
        self.normalize();
    }

    fn normalize(&mut self) {
        let rects: Vec<Rect> = (0..self.monitors.len())
            .map(|i| self.logical_rect(i))
            .collect();
        let anchors: Vec<usize> = (0..self.monitors.len())
            .filter(|&i| self.monitors[i].enabled)
            .collect();
        let (dx, dy) = normalize_offset(&rects, &anchors);
        for m in &mut self.monitors {
            m.pos = (m.pos.0 + dx, m.pos.1 + dy);
        }
    }

    pub fn set_mode(&mut self, i: usize, mode: Mode) {
        let m = &mut self.monitors[i];
        m.mode = mode;
        m.mode_touched = true;
        self.normalize();
    }

    pub fn set_scale(&mut self, i: usize, scale: f32) {
        self.monitors[i].scale = scale.clamp(0.25, 4.0);
        self.normalize();
    }

    pub fn set_transform(&mut self, i: usize, transform: u8) {
        self.monitors[i].transform = transform.min(7);
        self.normalize();
    }

    pub fn set_vrr(&mut self, i: usize, vrr: u8) {
        self.monitors[i].vrr = vrr.min(2);
    }

    /// Rejects a monitor mirroring itself.
    pub fn set_mirror(&mut self, i: usize, source: Option<String>) -> Result<(), String> {
        if source.as_deref() == Some(self.monitors[i].name.as_str()) {
            return Err("A monitor cannot mirror itself.".into());
        }
        self.monitors[i].mirror_of = source;
        Ok(())
    }

    pub fn enabled_count(&self) -> usize {
        self.monitors.iter().filter(|m| m.enabled).count()
    }

    /// Rejects disabling the last enabled monitor.
    pub fn set_enabled(&mut self, i: usize, enabled: bool) -> Result<(), String> {
        if !enabled && self.enabled_count() <= 1 && self.monitors[i].enabled {
            return Err("At least one monitor must stay enabled.".into());
        }
        self.monitors[i].enabled = enabled;
        self.normalize();
        Ok(())
    }

    // ----- apply / confirm / revert -----

    pub fn confirm_pending(&self) -> bool {
        self.confirm.is_some()
    }

    pub fn confirm_remaining(&self, now: Instant) -> Duration {
        self.confirm
            .as_ref()
            .map(|c| c.deadline.saturating_duration_since(now))
            .unwrap_or_default()
    }

    /// Apply the candidate to the running compositor as one atomic batch. On
    /// failure the previous state is restored and the error names the rejected
    /// monitor when the compositor's message identifies it. On success a confirm
    /// countdown starts.
    pub fn apply(&mut self, now: Instant) -> Result<(), String> {
        self.normalize();
        let revert_layout = self.applied_snapshot.clone();
        if let Err(detail) = self.comp.apply_layout(&self.monitors) {
            let culprit = self
                .monitors
                .iter()
                .map(|m| m.name.as_str())
                .find(|name| detail.contains(name));
            let msg = match culprit {
                Some(name) => format!(
                    "The compositor rejected the configuration of {name}: {detail}. Previous layout restored."
                ),
                None => format!(
                    "The compositor rejected the layout: {detail}. Previous layout restored."
                ),
            };
            self.revert(&revert_layout);
            return Err(msg);
        }
        self.confirm = Some(PendingConfirm {
            deadline: now + Duration::from_secs(CONFIRM_SECS),
            revert_layout,
        });
        Ok(())
    }

    /// Drive the countdown; returns a user-facing message when an auto-revert fired.
    pub fn tick(&mut self, now: Instant) -> Option<String> {
        if self.confirm.as_ref().is_some_and(|c| now >= c.deadline) {
            let c = self.confirm.take().unwrap();
            self.revert(&c.revert_layout);
            return Some("Not confirmed in time — previous layout restored.".into());
        }
        None
    }

    /// User confirmed: candidate becomes the known-good state and is persisted.
    pub fn keep(&mut self) -> Result<String, String> {
        self.confirm = None;
        self.applied_snapshot = self.monitors.clone();
        self.persist()
    }

    pub fn revert_now(&mut self) -> String {
        if let Some(c) = self.confirm.take() {
            self.revert(&c.revert_layout);
        }
        "Previous layout restored.".into()
    }

    fn revert(&mut self, layout: &[MonitorState]) {
        // Best-effort: a monitor unplugged mid-countdown makes its entry a no-op.
        let _ = self.comp.apply_layout(layout);
        self.resync();
    }

    /// Re-read compositor state so the UI matches reality after apply/revert/hotplug.
    pub fn resync(&mut self) {
        if let Ok(raws) = self.comp.query() {
            self.monitors = raws.iter().map(MonitorState::from_raw).collect();
            self.applied_snapshot = self.monitors.clone();
        }
    }

    /// Hotplug refresh that keeps uncommitted edits of still-present monitors.
    pub fn refresh_preserving_edits(&mut self) -> Result<(), String> {
        let raws = self.comp.query()?;
        self.applied_snapshot = raws.iter().map(MonitorState::from_raw).collect();
        let old = std::mem::take(&mut self.monitors);
        self.monitors = raws
            .iter()
            .map(|r| {
                old.iter()
                    .find(|m| m.name == r.name)
                    .cloned()
                    .map(|mut kept| {
                        let fresh = MonitorState::from_raw(r);
                        kept.modes = fresh.modes;
                        kept.description = fresh.description;
                        kept
                    })
                    .unwrap_or_else(|| MonitorState::from_raw(r))
            })
            .collect();
        Ok(())
    }

    // ----- persistence -----

    pub fn persist(&mut self) -> Result<String, String> {
        self.store.persist(&self.monitors)?;
        Ok("Settings kept and saved to your Hyprland config.".into())
    }

    // ----- profiles -----

    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.list().unwrap_or_default()
    }

    /// Save the current candidate layout under `name` (overwrites an existing profile).
    pub fn save_profile(&mut self, name: &str) -> Result<String, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("Profile name cannot be empty.".into());
        }
        let profile: Profile = self
            .monitors
            .iter()
            .map(ProfileMonitor::from_state)
            .collect();
        self.profiles.save(name, &profile)?;
        Ok(format!("Profile \"{name}\" saved."))
    }

    /// Load a profile into the candidate layout. Connected monitors absent from
    /// the profile are left untouched; profile entries for disconnected monitors
    /// are ignored. Nothing is applied to the compositor.
    pub fn load_profile(&mut self, name: &str) -> Result<String, String> {
        let Some(profile) = self.profiles.load(name)? else {
            return Err(format!("Profile \"{name}\" not found."));
        };
        // Reject a load that would leave zero enabled monitors.
        let would_enable = self.monitors.iter().any(|m| {
            profile
                .iter()
                .find(|p| p.name == m.name)
                .map(|p| p.enabled)
                .unwrap_or(m.enabled)
        });
        if !would_enable {
            return Err(format!(
                "Profile \"{name}\" would disable every connected monitor — not loaded."
            ));
        }
        let mut applied = 0;
        for entry in &profile {
            if let Some(m) = self.monitors.iter_mut().find(|m| m.name == entry.name) {
                entry.apply_to(m);
                applied += 1;
            }
        }
        self.normalize();
        Ok(format!(
            "Profile \"{name}\" loaded onto {applied} monitor(s) — click Apply to take effect."
        ))
    }

    pub fn delete_profile(&mut self, name: &str) -> Result<String, String> {
        self.profiles.delete(name)?;
        Ok(format!("Profile \"{name}\" deleted."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::monitor::{RawMonitor, parse_monitors_json};
    use std::cell::RefCell;
    use std::rc::Rc;

    const FIXTURE: &str = include_str!("../../tests/fixtures/monitors_all.json");

    #[derive(Default)]
    struct FakeState {
        applied: Vec<String>,
        batch_calls: usize,
        /// Substring that poisons a batch (e.g. a mode the compositor rejects).
        /// The previous (valid) config never contains it, so reverts succeed —
        /// matching real compositor behavior.
        reject_containing: Option<String>,
    }

    struct FakeCompositor {
        state: Rc<RefCell<FakeState>>,
        monitors: Vec<RawMonitor>,
    }

    impl Compositor for FakeCompositor {
        fn query(&self) -> Result<Vec<RawMonitor>, String> {
            Ok(self.monitors.clone())
        }
        fn apply_layout(&self, monitors: &[MonitorState]) -> Result<(), String> {
            self.state.borrow_mut().batch_calls += 1;
            let entries: Vec<String> = monitors.iter().map(|m| m.to_lua_entry()).collect();
            // Batch semantics: a failing entry rejects the whole chunk; nothing lands.
            if let Some(bad) = self.state.borrow().reject_containing.clone()
                && let Some(i) = entries.iter().position(|e| e.contains(bad.as_str()))
            {
                return Err(format!("invalid mode on output {}", monitors[i].name));
            }
            self.state.borrow_mut().applied.extend(entries);
            Ok(())
        }
    }

    struct MemStore {
        persisted: Rc<RefCell<Vec<MonitorState>>>,
    }

    impl ConfigStore for MemStore {
        fn persist(&self, monitors: &[MonitorState]) -> Result<(), String> {
            *self.persisted.borrow_mut() = monitors.to_vec();
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemProfiles {
        map: Rc<RefCell<std::collections::BTreeMap<String, Profile>>>,
    }

    impl ProfileStore for MemProfiles {
        fn list(&self) -> Result<Vec<String>, String> {
            Ok(self.map.borrow().keys().cloned().collect())
        }
        fn load(&self, name: &str) -> Result<Option<Profile>, String> {
            Ok(self.map.borrow().get(name).cloned())
        }
        fn save(&self, name: &str, profile: &Profile) -> Result<(), String> {
            self.map
                .borrow_mut()
                .insert(name.to_string(), profile.clone());
            Ok(())
        }
        fn delete(&self, name: &str) -> Result<(), String> {
            self.map.borrow_mut().remove(name);
            Ok(())
        }
    }

    type Persisted = Rc<RefCell<Vec<MonitorState>>>;

    fn setup(reject_containing: Option<&str>) -> (Session, Rc<RefCell<FakeState>>, Persisted) {
        let state = Rc::new(RefCell::new(FakeState {
            reject_containing: reject_containing.map(String::from),
            ..Default::default()
        }));
        let persisted = Rc::new(RefCell::new(Vec::new()));
        let comp = FakeCompositor {
            state: state.clone(),
            monitors: parse_monitors_json(FIXTURE).unwrap(),
        };
        let store = MemStore {
            persisted: persisted.clone(),
        };
        let session = Session::new(
            Box::new(comp),
            Box::new(store),
            Box::new(MemProfiles::default()),
        )
        .unwrap();
        (session, state, persisted)
    }

    #[test]
    fn apply_sends_lua_entries_and_starts_countdown() {
        let (mut s, state, _) = setup(None);
        let t0 = Instant::now();
        s.apply(t0).unwrap();
        assert!(s.confirm_pending());
        // The whole layout goes out as ONE atomic batch (no transient overlaps).
        assert_eq!(state.borrow().batch_calls, 1);
        let applied = state.borrow().applied.clone();
        assert!(applied
            .iter()
            .any(|a| a.contains("output = \"eDP-1\"") && a.contains("mode = \"1920x1080@144\"")));
        assert!(
            applied
                .iter()
                .any(|a| a.contains("output = \"DP-1\"") && a.contains("disabled = true"))
        );
    }

    #[test]
    fn countdown_timeout_reverts_to_previous_layout() {
        let (mut s, state, _) = setup(None);
        let t0 = Instant::now();
        // Move HDMI to the right of eDP and apply.
        let i = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        s.finalize_drop(i, (1920, 1080));
        s.apply(t0).unwrap();
        state.borrow_mut().applied.clear();

        let msg = s.tick(t0 + Duration::from_secs(CONFIRM_SECS + 1));
        assert!(msg.is_some());
        assert!(!s.confirm_pending());
        // Revert replayed the original positions (HDMI back at 0x0).
        let applied = state.borrow().applied.clone();
        assert!(
            applied
                .iter()
                .any(|a| a.contains("output = \"HDMI-A-1\"") && a.contains("position = \"0x0\""))
        );
    }

    #[test]
    fn tick_before_deadline_does_nothing() {
        let (mut s, _, _) = setup(None);
        let t0 = Instant::now();
        s.apply(t0).unwrap();
        assert!(s.tick(t0 + Duration::from_secs(1)).is_none());
        assert!(s.confirm_pending());
    }

    #[test]
    fn apply_failure_names_monitor_and_reverts() {
        // The compositor rejects the 4K mode; the previous config stays valid.
        let (mut s, state, _) = setup(Some("3840x2160"));
        let hdmi = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        let four_k = s.monitors[hdmi].modes[0];
        s.set_mode(hdmi, four_k);
        let err = s.apply(Instant::now()).unwrap_err();
        assert!(err.contains("HDMI-A-1"));
        assert!(err.contains("invalid mode"));
        assert!(!s.confirm_pending());
        // Batch was rejected atomically, so only the revert replay was recorded.
        let applied = state.borrow().applied.clone();
        let edp_count = applied
            .iter()
            .filter(|a| a.contains("output = \"eDP-1\""))
            .count();
        assert_eq!(edp_count, 1);
        assert!(applied.iter().any(|a| a.contains("output = \"HDMI-A-1\"")));
    }

    #[test]
    fn keep_persists_layout_to_store() {
        let (mut s, _, persisted) = setup(None);
        let t0 = Instant::now();
        s.apply(t0).unwrap();
        let msg = s.keep().unwrap();
        assert!(msg.contains("saved"));
        let saved = persisted.borrow();
        assert!(saved.iter().any(|m| m.name == "eDP-1" && m.enabled));
        assert!(saved.iter().any(|m| m.name == "DP-1" && !m.enabled));
        assert!(!s.confirm_pending());
    }

    #[test]
    fn cannot_disable_last_enabled_monitor() {
        let (mut s, _, _) = setup(None);
        let hdmi = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        s.set_enabled(hdmi, false).unwrap();
        let edp = s.monitors.iter().position(|m| m.name == "eDP-1").unwrap();
        let err = s.set_enabled(edp, false).unwrap_err();
        assert!(err.contains("At least one monitor"));
        assert!(s.monitors[edp].enabled);
    }

    #[test]
    fn finalize_drop_resolves_overlap_and_normalizes() {
        let (mut s, _, _) = setup(None);
        let hdmi = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        // Drop HDMI right on top of eDP (which sits at 0x1080).
        s.finalize_drop(hdmi, (100, 1000));
        let a = s.logical_rect(hdmi);
        let edp = s.monitors.iter().position(|m| m.name == "eDP-1").unwrap();
        let b = s.logical_rect(edp);
        assert!(!a.overlaps(&b));
        // Normalized: enabled bounding box starts at 0x0.
        let min_x = a.x.min(b.x);
        let min_y = a.y.min(b.y);
        assert_eq!((min_x, min_y), (0, 0));
    }

    #[test]
    fn profile_save_load_delete_roundtrip() {
        let (mut s, _, _) = setup(None);
        let hdmi = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        s.set_transform(hdmi, 1);
        s.set_vrr(hdmi, 1);
        s.save_profile("docked").unwrap();
        assert_eq!(s.profile_names(), vec!["docked".to_string()]);

        // Change state, then load restores it.
        s.set_transform(hdmi, 0);
        s.set_vrr(hdmi, 0);
        let msg = s.load_profile("docked").unwrap();
        assert!(msg.contains("Apply"));
        assert_eq!(s.monitors[hdmi].transform, 1);
        assert_eq!(s.monitors[hdmi].vrr, 1);
        assert!(s.monitors[hdmi].mode_touched);

        s.delete_profile("docked").unwrap();
        assert!(s.profile_names().is_empty());
    }

    #[test]
    fn profile_load_ignores_disconnected_monitors() {
        let (mut s, _, _) = setup(None);
        s.save_profile("full").unwrap();
        // Simulate the profile referencing a monitor that is no longer present.
        let ghost = ProfileMonitor {
            name: "DP-9".into(),
            mode: "1920x1080@60".into(),
            pos: (0, 0),
            scale: 1.0,
            enabled: true,
            transform: 0,
            vrr: 0,
            mirror_of: None,
        };
        let mut profile: Profile = s.monitors.iter().map(ProfileMonitor::from_state).collect();
        profile.push(ghost);
        s.profiles.save("full", &profile).unwrap();
        let msg = s.load_profile("full").unwrap();
        assert!(msg.contains("3 monitor(s)"));
    }

    #[test]
    fn profile_that_disables_everything_is_rejected() {
        let (mut s, _, _) = setup(None);
        let mut profile: Profile = s.monitors.iter().map(ProfileMonitor::from_state).collect();
        for p in &mut profile {
            p.enabled = false;
        }
        s.profiles.save("dark", &profile).unwrap();
        let err = s.load_profile("dark").unwrap_err();
        assert!(err.contains("disable every"));
        assert!(s.monitors.iter().any(|m| m.enabled)); // untouched
    }

    #[test]
    fn cannot_mirror_itself() {
        let (mut s, _, _) = setup(None);
        let edp = s.monitors.iter().position(|m| m.name == "eDP-1").unwrap();
        assert!(s.set_mirror(edp, Some("eDP-1".into())).is_err());
        assert!(s.set_mirror(edp, Some("HDMI-A-1".into())).is_ok());
        assert_eq!(s.monitors[edp].mirror_of.as_deref(), Some("HDMI-A-1"));
    }

    #[test]
    fn mode_change_marks_touched() {
        let (mut s, _, _) = setup(None);
        let hdmi = s
            .monitors
            .iter()
            .position(|m| m.name == "HDMI-A-1")
            .unwrap();
        let four_k = s.monitors[hdmi].modes[0];
        s.set_mode(hdmi, four_k);
        assert!(s.monitors[hdmi].mode_touched);
        assert_eq!(s.monitors[hdmi].mode.width, 3840);
    }
}
