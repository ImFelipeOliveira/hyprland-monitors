//! Ports: the two IO boundaries the application layer depends on.
//! Real adapters live in `adapters::hyprctl` (compositor, Lua or classic) and
//! `adapters::{omarchy_lua, hyprland_conf}` (config file); tests substitute
//! in-memory fakes.

use crate::domain::monitor::{MonitorState, RawMonitor};

pub trait Compositor {
    fn query(&self) -> Result<Vec<RawMonitor>, String>;
    /// Apply a whole layout as ONE atomic request. Applying monitors one at a
    /// time creates transient overlapping states that Hyprland rejects
    /// ("Your monitor layout is set up incorrectly"). Each adapter renders its
    /// own wire format from the domain state.
    fn apply_layout(&self, monitors: &[MonitorState]) -> Result<(), String>;
}

pub trait ConfigStore {
    /// Persist the given layout so it survives compositor restarts.
    fn persist(&self, monitors: &[MonitorState]) -> Result<(), String>;
}

pub trait ProfileStore {
    fn list(&self) -> Result<Vec<String>, String>;
    fn load(&self, name: &str) -> Result<Option<crate::application::profiles::Profile>, String>;
    fn save(
        &self,
        name: &str,
        profile: &crate::application::profiles::Profile,
    ) -> Result<(), String>;
    fn delete(&self, name: &str) -> Result<(), String>;
}
