//! Keyboard shortcuts that can be changed (Options › Keyboard): each
//! command's name, where it applies and its default keys, and the keys the
//! user chose instead, kept in the settings by the command's id as text
//! (`"Ctrl+Shift+R"`, Ctrl being Cmd on macOS).
//!
//! Keys match exactly: Shift+Q turns by 90° and doesn't also turn by Q's
//! step. Text editing, Escape and Enter, the platform's Copy, Cut and Paste,
//! Delete in the area view, and the script editor's numbered bookmarks
//! (Ctrl and a digit) keep their keys. The defaults are Aurora's where it
//! has the command (its forms' shortcuts,
//! `docs/parity/aurora-ui-inventory.md` appendix A), else Moonglow's.

use std::collections::{BTreeMap, HashMap};

use egui::{Event, InputState, Key, KeyboardShortcut, ModifierNames, Modifiers};

/// Where a command's keys work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    /// Anywhere in the window.
    General,
    /// In the area view, with the pointer over it.
    Area,
    /// In the script editor, with its text focused.
    Script,
    /// In the conversation editor, with the pointer over it.
    Conversation,
}

impl Group {
    pub fn name(self) -> &'static str {
        match self {
            Group::General => "General",
            Group::Area => "Area",
            Group::Script => "Script Editor",
            Group::Conversation => "Conversation Editor",
        }
    }

    /// Whether keys in both groups can be pressed at the same moment (the
    /// general keys work everywhere).
    fn overlaps(self, other: Group) -> bool {
        self == other || self == Group::General || other == Group::General
    }
}

/// A command with keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cmd {
    NewModule,
    OpenModule,
    Save,
    Undo,
    Redo,
    ReplaceText,
    AreaWizard,
    Factions,
    Journal,
    CompileAll,
    TestModule,
    TestChoose,
    Manual,
    NewConversation,
    NewScript,
    CreatureWizard,
    ItemWizard,
    FullScreen,
    SelectTiles,
    TurnLeft,
    TurnRight,
    TurnLeft90,
    TurnRight90,
    DropToGround,
    Overview,
    CameraForward,
    CameraBack,
    CameraLeft,
    CameraRight,
    CameraTurnLeft,
    CameraTurnRight,
    CameraTiltUp,
    CameraTiltDown,
    CameraUp,
    CameraDown,
    Find,
    Replace,
    FindNext,
    Bookmark,
    Complete,
    Definition,
    References,
    RenameSymbol,
    AddLine,
    DeleteLine,
}

const CTRL: Modifiers = Modifiers::COMMAND;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NONE: Modifiers = Modifiers::NONE;

impl Cmd {
    pub const ALL: [Cmd; 45] = [
        Cmd::NewModule,
        Cmd::OpenModule,
        Cmd::Save,
        Cmd::Undo,
        Cmd::Redo,
        Cmd::ReplaceText,
        Cmd::AreaWizard,
        Cmd::Factions,
        Cmd::Journal,
        Cmd::CompileAll,
        Cmd::TestModule,
        Cmd::TestChoose,
        Cmd::Manual,
        Cmd::NewConversation,
        Cmd::NewScript,
        Cmd::CreatureWizard,
        Cmd::ItemWizard,
        Cmd::FullScreen,
        Cmd::SelectTiles,
        Cmd::TurnLeft,
        Cmd::TurnRight,
        Cmd::TurnLeft90,
        Cmd::TurnRight90,
        Cmd::DropToGround,
        Cmd::Overview,
        Cmd::CameraForward,
        Cmd::CameraBack,
        Cmd::CameraLeft,
        Cmd::CameraRight,
        Cmd::CameraTurnLeft,
        Cmd::CameraTurnRight,
        Cmd::CameraTiltUp,
        Cmd::CameraTiltDown,
        Cmd::CameraUp,
        Cmd::CameraDown,
        Cmd::Find,
        Cmd::Replace,
        Cmd::FindNext,
        Cmd::Bookmark,
        Cmd::Complete,
        Cmd::Definition,
        Cmd::References,
        Cmd::RenameSymbol,
        Cmd::AddLine,
        Cmd::DeleteLine,
    ];

    /// The id the settings keep it by, its name and where it works.
    pub fn about(self) -> (&'static str, &'static str, Group) {
        use Group::*;
        match self {
            Cmd::NewModule => ("new-module", "New Module", General),
            Cmd::OpenModule => ("open-module", "Open Module", General),
            Cmd::Save => ("save", "Save", General),
            Cmd::Undo => ("undo", "Undo", General),
            Cmd::Redo => ("redo", "Redo", General),
            Cmd::ReplaceText => ("replace-text", "Find and Replace Text", General),
            Cmd::AreaWizard => ("area-wizard", "Area Wizard", General),
            Cmd::Factions => ("factions", "Faction Editor", General),
            Cmd::Journal => ("journal", "Journal Editor", General),
            Cmd::CompileAll => ("compile-all", "Compile All Scripts", General),
            Cmd::TestModule => ("test-module", "Test Module", General),
            Cmd::TestChoose => ("test-choose", "Test Module, Choose Character", General),
            Cmd::Manual => ("manual", "User Manual", General),
            Cmd::NewConversation => ("new-conversation", "New Conversation", General),
            Cmd::NewScript => ("new-script", "New Script", General),
            Cmd::CreatureWizard => ("creature-wizard", "Creature Wizard", General),
            Cmd::ItemWizard => ("item-wizard", "Item Wizard", General),
            Cmd::FullScreen => ("full-screen", "Full Screen", General),
            Cmd::SelectTiles => ("select-tiles", "Select Tiles or Objects", Area),
            Cmd::TurnLeft => ("turn-left", "Turn Anticlockwise", Area),
            Cmd::TurnRight => ("turn-right", "Turn Clockwise", Area),
            Cmd::TurnLeft90 => ("turn-left-90", "Turn Anticlockwise 90°", Area),
            Cmd::TurnRight90 => ("turn-right-90", "Turn Clockwise 90°", Area),
            Cmd::DropToGround => ("drop", "Drop to the Ground", Area),
            Cmd::Overview => ("overview", "View the Whole Area", Area),
            Cmd::CameraForward => ("camera-forward", "Move Camera Forward", Area),
            Cmd::CameraBack => ("camera-back", "Move Camera Back", Area),
            Cmd::CameraLeft => ("camera-left", "Move Camera Left", Area),
            Cmd::CameraRight => ("camera-right", "Move Camera Right", Area),
            Cmd::CameraTurnLeft => ("camera-turn-left", "Turn Camera Left", Area),
            Cmd::CameraTurnRight => ("camera-turn-right", "Turn Camera Right", Area),
            Cmd::CameraTiltUp => ("camera-tilt-up", "Tilt Camera Up", Area),
            Cmd::CameraTiltDown => ("camera-tilt-down", "Tilt Camera Down", Area),
            Cmd::CameraUp => ("camera-up", "Move Camera Up", Area),
            Cmd::CameraDown => ("camera-down", "Move Camera Down", Area),
            Cmd::Find => ("find", "Find", Script),
            Cmd::Replace => ("replace", "Replace", Script),
            Cmd::FindNext => ("find-next", "Find Next", Script),
            Cmd::Bookmark => ("bookmark", "Toggle Bookmark", Script),
            Cmd::Complete => ("complete", "Complete", Script),
            Cmd::Definition => ("definition", "Go to Definition", Script),
            Cmd::References => ("references", "Find References", Script),
            Cmd::RenameSymbol => ("rename-symbol", "Rename Symbol", Script),
            Cmd::AddLine => ("add-line", "Add Line", Conversation),
            Cmd::DeleteLine => ("delete-line", "Delete Line", Conversation),
        }
    }

    pub fn id(self) -> &'static str {
        self.about().0
    }

    pub fn name(self) -> &'static str {
        self.about().1
    }

    pub fn group(self) -> Group {
        self.about().2
    }

    /// Moonglow's keys for it.
    pub fn defaults(self) -> Vec<KeyboardShortcut> {
        let k = KeyboardShortcut::new;
        match self {
            Cmd::NewModule => vec![k(CTRL, Key::N)],
            Cmd::OpenModule => vec![k(CTRL, Key::O)],
            Cmd::Save => vec![k(CTRL, Key::S)],
            Cmd::Undo => vec![k(CTRL, Key::Z)],
            Cmd::Redo => vec![k(CTRL | SHIFT, Key::Z), k(CTRL, Key::Y)],
            Cmd::ReplaceText => vec![k(CTRL, Key::H)],
            Cmd::AreaWizard => vec![k(CTRL | Modifiers::ALT, Key::A)],
            Cmd::Factions => vec![k(CTRL | Modifiers::ALT, Key::F)],
            Cmd::Journal => vec![k(CTRL | Modifiers::ALT, Key::J)],
            Cmd::CompileAll => vec![k(NONE, Key::F7)],
            Cmd::TestModule => vec![k(NONE, Key::F9)],
            Cmd::TestChoose => vec![k(SHIFT, Key::F9)],
            Cmd::Manual => vec![k(NONE, Key::F1)],
            // Aurora's keys.
            Cmd::NewConversation => vec![k(CTRL | Modifiers::ALT, Key::V)],
            Cmd::NewScript => vec![k(CTRL | Modifiers::ALT, Key::S)],
            Cmd::CreatureWizard => vec![k(CTRL | Modifiers::ALT, Key::C)],
            Cmd::ItemWizard => vec![k(CTRL | Modifiers::ALT, Key::I)],
            Cmd::FullScreen => vec![k(NONE, Key::F11)],
            Cmd::SelectTiles => vec![k(NONE, Key::F10)],
            Cmd::TurnLeft => vec![k(NONE, Key::Q)],
            Cmd::TurnRight => vec![k(NONE, Key::E)],
            Cmd::TurnLeft90 => vec![k(SHIFT, Key::Q)],
            Cmd::TurnRight90 => vec![k(SHIFT, Key::E)],
            Cmd::DropToGround => vec![k(NONE, Key::G)],
            Cmd::Overview => vec![k(NONE, Key::Num5)],
            Cmd::CameraForward => {
                vec![k(NONE, Key::W), k(NONE, Key::ArrowUp), k(NONE, Key::Num8)]
            }
            Cmd::CameraBack => vec![k(NONE, Key::S), k(NONE, Key::ArrowDown), k(NONE, Key::Num2)],
            Cmd::CameraLeft => vec![k(NONE, Key::A), k(NONE, Key::ArrowLeft), k(NONE, Key::Num4)],
            Cmd::CameraRight => {
                vec![k(NONE, Key::D), k(NONE, Key::ArrowRight), k(NONE, Key::Num6)]
            }
            Cmd::CameraTurnLeft => vec![k(NONE, Key::Num7)],
            Cmd::CameraTurnRight => vec![k(NONE, Key::Num9)],
            Cmd::CameraTiltUp => vec![k(NONE, Key::Num1)],
            Cmd::CameraTiltDown => vec![k(NONE, Key::Num3)],
            Cmd::CameraUp => vec![k(NONE, Key::Z)],
            Cmd::CameraDown => vec![k(NONE, Key::C)],
            Cmd::Find => vec![k(CTRL, Key::F)],
            Cmd::Replace => vec![k(CTRL, Key::R)],
            Cmd::FindNext => vec![k(NONE, Key::F3)],
            Cmd::Bookmark => vec![k(NONE, Key::F5)],
            Cmd::Complete => vec![k(NONE, Key::F2), k(CTRL, Key::Space)],
            Cmd::Definition => vec![k(NONE, Key::F12)],
            Cmd::References => vec![k(SHIFT, Key::F12)],
            Cmd::RenameSymbol => vec![k(CTRL | SHIFT, Key::R)],
            Cmd::AddLine => vec![k(CTRL, Key::A)],
            Cmd::DeleteLine => vec![k(NONE, Key::Delete)],
        }
    }

    /// Held down rather than pressed (the camera's moves and turns).
    pub fn is_held(self) -> bool {
        matches!(
            self,
            Cmd::CameraForward
                | Cmd::CameraBack
                | Cmd::CameraLeft
                | Cmd::CameraRight
                | Cmd::CameraTurnLeft
                | Cmd::CameraTurnRight
                | Cmd::CameraTiltUp
                | Cmd::CameraTiltDown
                | Cmd::CameraUp
                | Cmd::CameraDown
        )
    }

    pub fn from_id(id: &str) -> Option<Cmd> {
        Cmd::ALL.into_iter().find(|c| c.id() == id)
    }
}

/// A shortcut as the settings keep it: `Ctrl+Shift+R` (Ctrl is Cmd on
/// macOS), `F12`, `Alt+5`.
pub fn to_text(s: &KeyboardShortcut) -> String {
    let mut parts = Vec::new();
    if s.modifiers.command || s.modifiers.ctrl || s.modifiers.mac_cmd {
        parts.push("Ctrl");
    }
    if s.modifiers.alt {
        parts.push("Alt");
    }
    if s.modifiers.shift {
        parts.push("Shift");
    }
    parts.push(s.logical_key.name());
    parts.join("+")
}

/// Reads [`to_text`]'s form (`Cmd` is taken as `Ctrl`).
pub fn from_text(text: &str) -> Option<KeyboardShortcut> {
    let parts: Vec<&str> = text.split('+').map(str::trim).collect();
    let (key, mods) = parts.split_last()?;
    let mut m = Modifiers::NONE;
    for p in mods {
        match p.to_ascii_lowercase().as_str() {
            "ctrl" | "cmd" | "command" => m |= Modifiers::COMMAND,
            "alt" | "option" => m |= Modifiers::ALT,
            "shift" => m |= Modifiers::SHIFT,
            _ => return None,
        }
    }
    // An empty key with a trailing "+": the plus key.
    let key = if key.is_empty() && text.ends_with('+') { "Plus" } else { key };
    Some(KeyboardShortcut::new(m, Key::from_name(key)?))
}

/// A shortcut as shown on this system (`Cmd` on macOS).
pub fn shown(s: &KeyboardShortcut, mac: bool) -> String {
    s.format(&ModifierNames::NAMES, mac)
}

/// Whether a key press in `input` is the shortcut, modifiers exactly.
fn is(e: &Event, s: &KeyboardShortcut) -> bool {
    matches!(e, Event::Key { key, pressed: true, modifiers, .. }
        if *key == s.logical_key && modifiers.matches_exact(s.modifiers))
}

/// The keys of every command: Moonglow's, but where the settings choose
/// others.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap {
    keys: HashMap<Cmd, Vec<KeyboardShortcut>>,
}

impl Default for Keymap {
    fn default() -> Keymap {
        Keymap::new(&BTreeMap::new())
    }
}

impl Keymap {
    /// The keys with the settings' choices (by command id; an empty list
    /// takes a command's keys away). Keys that don't read are left out.
    pub fn new(chosen: &BTreeMap<String, Vec<String>>) -> Keymap {
        let keys = Cmd::ALL
            .into_iter()
            .map(|c| {
                let keys = match chosen.get(c.id()) {
                    Some(list) => list.iter().filter_map(|t| from_text(t)).collect(),
                    None => c.defaults(),
                };
                (c, keys)
            })
            .collect();
        Keymap { keys }
    }

    /// The settings' form: each command whose keys aren't Moonglow's.
    pub fn chosen(&self) -> BTreeMap<String, Vec<String>> {
        Cmd::ALL
            .into_iter()
            .filter(|c| self.keys(*c) != c.defaults())
            .map(|c| (c.id().to_string(), self.keys(c).iter().map(to_text).collect()))
            .collect()
    }

    pub fn keys(&self, cmd: Cmd) -> Vec<KeyboardShortcut> {
        self.keys.get(&cmd).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, cmd: Cmd, keys: Vec<KeyboardShortcut>) {
        self.keys.insert(cmd, keys);
    }

    /// The first key, as shown on this system, for menus and tooltips;
    /// empty for none.
    pub fn label(&self, cmd: Cmd, ctx: &egui::Context) -> String {
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        self.keys.get(&cmd).and_then(|k| k.first()).map(|s| shown(s, mac)).unwrap_or_default()
    }

    /// A name with its key in brackets: `Test Module (F9)`.
    pub fn titled(&self, name: &str, cmd: Cmd, ctx: &egui::Context) -> String {
        match self.label(cmd, ctx) {
            k if k.is_empty() => name.to_string(),
            k => format!("{name} ({k})"),
        }
    }

    /// Whether one of the command's keys was pressed this frame; the press
    /// is used up, so nothing else takes it.
    pub fn consume(&self, i: &mut InputState, cmd: Cmd) -> bool {
        let Some(keys) = self.keys.get(&cmd) else { return false };
        let mut hit = false;
        i.events.retain(|e| {
            let this = !hit && keys.iter().any(|s| is(e, s));
            hit |= this;
            !this
        });
        hit
    }

    /// Whether one of the command's keys was pressed this frame (left for
    /// others).
    pub fn pressed(&self, i: &InputState, cmd: Cmd) -> bool {
        let Some(keys) = self.keys.get(&cmd) else { return false };
        i.events.iter().any(|e| keys.iter().any(|s| is(e, s)))
    }

    /// Whether one of the command's keys is held down. A key without
    /// modifiers counts with none held; letters and digits only when no
    /// text field has the keyboard (`typing`), so Ctrl+S doesn't move the
    /// camera.
    pub fn held(&self, i: &InputState, cmd: Cmd, typing: bool) -> bool {
        let Some(keys) = self.keys.get(&cmd) else { return false };
        keys.iter().any(|s| {
            let plain = s.modifiers.is_none();
            let text = s.logical_key.name().chars().count() == 1;
            i.key_down(s.logical_key)
                && if plain {
                    !(text && typing)
                        && !(text && (i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt))
                } else {
                    i.modifiers.matches_exact(s.modifiers)
                }
        })
    }

    /// Keys that two commands working at the same moment share.
    pub fn conflicts(&self) -> Vec<(KeyboardShortcut, Cmd, Cmd)> {
        let mut out = Vec::new();
        for (i, a) in Cmd::ALL.iter().enumerate() {
            for b in &Cmd::ALL[i + 1..] {
                if !a.group().overlaps(b.group()) {
                    continue;
                }
                for k in self.keys(*a) {
                    if self.keys(*b).contains(&k) {
                        out.push((k, *a, *b));
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcuts_as_text() {
        for c in Cmd::ALL {
            for k in c.defaults() {
                assert_eq!(from_text(&to_text(&k)), Some(k), "{}", to_text(&k));
            }
        }
        assert_eq!(to_text(&KeyboardShortcut::new(CTRL | SHIFT, Key::R)), "Ctrl+Shift+R");
        assert_eq!(
            from_text("cmd+alt+F12"),
            Some(KeyboardShortcut::new(CTRL | Modifiers::ALT, Key::F12))
        );
        assert_eq!(from_text("Hyper+R"), None);
        assert_eq!(from_text("Ctrl+"), Some(KeyboardShortcut::new(CTRL, Key::Plus)));
        // Every command has an id of its own.
        let ids: std::collections::HashSet<_> = Cmd::ALL.iter().map(|c| c.id()).collect();
        assert_eq!(ids.len(), Cmd::ALL.len());
        assert!(Keymap::default().conflicts().is_empty(), "{:?}", Keymap::default().conflicts());
    }

    #[test]
    fn chosen_keys_override_and_match_exactly() {
        let mut chosen = BTreeMap::new();
        chosen.insert("test-module".to_string(), vec!["Ctrl+T".to_string()]);
        chosen.insert("manual".to_string(), Vec::new());
        let map = Keymap::new(&chosen);
        assert_eq!(map.keys(Cmd::TestModule), [KeyboardShortcut::new(CTRL, Key::T)]);
        assert!(map.keys(Cmd::Manual).is_empty());
        assert_eq!(map.chosen(), chosen);
        let press = |m: Modifiers, key: Key| {
            let mut i = InputState::default();
            i.events.push(Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: m,
            });
            i
        };
        let i = press(SHIFT, Key::Q);
        assert!(map.pressed(&i, Cmd::TurnLeft90) && !map.pressed(&i, Cmd::TurnLeft));
        let mut i = press(CTRL, Key::T);
        assert!(map.consume(&mut i, Cmd::TestModule));
        assert!(!map.consume(&mut i, Cmd::TestModule), "used up");
        // A conflict: the same key for two general commands.
        let mut map = map;
        map.set(Cmd::Manual, vec![KeyboardShortcut::new(CTRL, Key::T)]);
        assert_eq!(map.conflicts().len(), 1);
    }
}
