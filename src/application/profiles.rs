//! Named layout profiles: serializable snapshots of the per-monitor settings,
//! matched back onto connected monitors by output name.

use crate::domain::monitor::{Mode, MonitorState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMonitor {
    pub name: String,
    /// Mode as a config string ("1920x1080@144").
    pub mode: String,
    pub pos: (i32, i32),
    pub scale: f32,
    pub enabled: bool,
    #[serde(default)]
    pub transform: u8,
    #[serde(default)]
    pub vrr: u8,
    #[serde(default)]
    pub mirror_of: Option<String>,
}

pub type Profile = Vec<ProfileMonitor>;

impl ProfileMonitor {
    pub fn from_state(m: &MonitorState) -> ProfileMonitor {
        ProfileMonitor {
            name: m.name.clone(),
            mode: m.mode.to_config_string(),
            pos: m.pos,
            scale: m.scale,
            enabled: m.enabled,
            transform: m.transform,
            vrr: m.vrr,
            mirror_of: m.mirror_of.clone(),
        }
    }

    /// Apply this profile entry onto a matching connected monitor. The mode is
    /// snapped to the closest advertised mode and marked as touched — a profile
    /// pins its modes deliberately.
    pub fn apply_to(&self, m: &mut MonitorState) {
        if let Some(wanted) = Mode::parse(&self.mode) {
            m.mode = m
                .modes
                .iter()
                .copied()
                .find(|c| {
                    c.width == wanted.width
                        && c.height == wanted.height
                        && (c.refresh - wanted.refresh).abs() < 0.5
                })
                .unwrap_or(wanted);
            m.mode_touched = true;
        }
        m.pos = self.pos;
        m.scale = self.scale.clamp(0.25, 4.0);
        m.enabled = self.enabled;
        m.transform = self.transform.min(7);
        m.vrr = self.vrr.min(2);
        m.mirror_of = self.mirror_of.clone();
    }
}
