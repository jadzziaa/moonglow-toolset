//! What Moonglow remembers between runs (saved by the desktop app).

use std::path::{Path, PathBuf};

use mg_resman::GameInstall;
use serde::{Deserialize, Serialize};

/// How many recent modules File > Recent Modules keeps.
pub const RECENT_MAX: usize = 10;

/// The script editor's syntax elements, in the order of
/// [`ScriptStyle::colors`] (Aurora's Options > Script Editor list).
pub const SCRIPT_ELEMENTS: [&str; 8] =
    ["Text", "Comment", "Directive", "Keyword", "Number", "String", "Constant", "Error"];

/// The script editor's look (Options > Script Editor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScriptStyle {
    /// Font size in points.
    pub font_size: u8,
    /// A colour (sRGB) for each of [`SCRIPT_ELEMENTS`]; `None` follows the
    /// light or dark theme.
    pub colors: [Option<[u8; 3]>; 8],
}

impl Default for ScriptStyle {
    fn default() -> ScriptStyle {
        ScriptStyle { font_size: 13, colors: [None; 8] }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Recently opened or saved modules, most recent first.
    pub recent: Vec<PathBuf>,
    /// The game install, when not the detected one.
    pub game_root: Option<PathBuf>,
    /// The NWN user folder (haks, override, modules), when not the detected
    /// one.
    pub user_dir: Option<PathBuf>,
    pub script_style: ScriptStyle,
}

impl Settings {
    /// Puts a module at the top of the recent list.
    pub fn remember(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(RECENT_MAX);
    }

    /// The game install these settings choose: the configured folders, or
    /// what is detected.
    pub fn install(&self) -> Option<GameInstall> {
        let detected = GameInstall::detect();
        let root = match &self.game_root {
            Some(r) => r.clone(),
            None => detected.as_ref()?.root.clone(),
        };
        let user_dir = self.user_dir.clone().or_else(|| detected.and_then(|d| d.user_dir));
        Some(GameInstall::new(root, user_dir, "en"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_saved_before_script_styles_still_load() {
        let s: Settings = serde_json::from_str(r#"{"recent":["a.mod"]}"#).unwrap();
        assert_eq!(s.script_style, ScriptStyle::default());
        let mut t = s.clone();
        t.script_style.colors[3] = Some([1, 2, 3]);
        let back: Settings = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn recent_modules_are_most_recent_first_without_duplicates() {
        let mut s = Settings::default();
        for p in ["a.mod", "b.mod", "a.mod"] {
            s.remember(Path::new(p));
        }
        assert_eq!(s.recent, [PathBuf::from("a.mod"), PathBuf::from("b.mod")]);
        for i in 0..20 {
            s.remember(Path::new(&format!("{i}.mod")));
        }
        assert_eq!(s.recent.len(), RECENT_MAX);
        assert_eq!(s.recent[0], PathBuf::from("19.mod"));
    }
}
