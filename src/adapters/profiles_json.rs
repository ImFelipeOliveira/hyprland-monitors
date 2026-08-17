//! Profile-store adapter: all named profiles live in a single JSON map at
//! `~/.config/hyprland-monitors/profiles.json`, written atomically.

use crate::application::ports::ProfileStore;
use crate::application::profiles::Profile;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub struct JsonProfileStore {
    pub path: PathBuf,
}

impl JsonProfileStore {
    pub fn default_path() -> Result<PathBuf, String> {
        let home = std::env::var_os("HOME").ok_or("HOME environment variable is not set")?;
        Ok(Path::new(&home).join(".config/hyprland-monitors/profiles.json"))
    }

    fn read_all(&self) -> Result<BTreeMap<String, Profile>, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| format!("corrupt profiles file {}: {e}", self.path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(format!("failed to read {}: {e}", self.path.display())),
        }
    }

    fn write_all(&self, map: &BTreeMap<String, Profile>) -> Result<(), String> {
        let dir = self
            .path
            .parent()
            .ok_or("profiles path has no parent directory")?;
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("failed to create {}: {e}", dir.display()))?;
        let content = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
        let tmp = dir.join(".profiles.json.tmp");
        std::fs::write(&tmp, content)
            .map_err(|e| format!("failed to write {}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, &self.path).map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("failed to replace {}: {e}", self.path.display())
        })
    }
}

impl ProfileStore for JsonProfileStore {
    fn list(&self) -> Result<Vec<String>, String> {
        Ok(self.read_all()?.keys().cloned().collect())
    }
    fn load(&self, name: &str) -> Result<Option<Profile>, String> {
        Ok(self.read_all()?.remove(name))
    }
    fn save(&self, name: &str, profile: &Profile) -> Result<(), String> {
        let mut map = self.read_all()?;
        map.insert(name.to_string(), profile.clone());
        self.write_all(&map)
    }
    fn delete(&self, name: &str) -> Result<(), String> {
        let mut map = self.read_all()?;
        map.remove(name);
        self.write_all(&map)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::profiles::ProfileMonitor;

    #[test]
    fn json_store_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "hyprland-monitors-test-profiles-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = JsonProfileStore {
            path: dir.join("profiles.json"),
        };

        let profile = vec![ProfileMonitor {
            name: "eDP-1".into(),
            mode: "1920x1080@144".into(),
            pos: (0, 0),
            scale: 1.0,
            enabled: true,
            transform: 1,
            vrr: 2,
            mirror_of: None,
        }];
        store.save("docked", &profile).unwrap();
        assert_eq!(store.list().unwrap(), vec!["docked".to_string()]);

        let loaded = store.load("docked").unwrap().unwrap();
        assert_eq!(loaded[0].transform, 1);
        assert_eq!(loaded[0].vrr, 2);
        assert!(store.load("missing").unwrap().is_none());

        store.delete("docked").unwrap();
        assert!(store.list().unwrap().is_empty());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
