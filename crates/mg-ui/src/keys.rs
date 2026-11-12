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
    /// In the NUI designer, outside text fields and menus.
    Nui,
    /// In the module tree or a palette, while it has the keys.
    Panes,
    /// The enabled plugins' commands (anywhere in the window).
    Plugins,
    /// The open project's commands (`[commands]`, anywhere in the
    /// window).
    Project,
}

impl Group {
    pub fn name(self) -> &'static str {
        match self {
            Group::General => "General",
            Group::Area => "Area",
            Group::Script => "Script Editor",
            Group::Conversation => "Conversation Editor",
            Group::Nui => "NUI Creator",
            Group::Panes => "Module Tree and Palettes",
            Group::Plugins => "Plugins",
            Group::Project => "Project Commands",
        }
    }

    /// Whether keys in both groups can be pressed at the same moment (the
    /// general keys work everywhere).
    fn overlaps(self, other: Group) -> bool {
        let anywhere = |g: Group| matches!(g, Group::General | Group::Plugins | Group::Project);
        self == other || anywhere(self) || anywhere(other)
    }
}

/// A command with keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Cmd {
    NewModule,
    OpenModule,
    /// File › Save; in a script editor that has the keyboard, that
    /// script's own Save (`commands::keys_pressed`), as in Aurora.
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
    CommandPalette,
    /// Quick Open: the Top Bar's field (Ctrl+P, where no command the user
    /// gave keys has it: [`Keymap::give_way`]).
    QuickOpen,
    SelectTiles,
    /// The area view's placeable brush, on or off (its toolbar's Paint).
    PaintPlaceables,
    /// The tiles palette's tile tools: chosen as brushes, or with tiles
    /// selected in Select Tiles, at once on them
    /// (`terrain_mode::tool_keys`).
    Eraser,
    RefineTile,
    NextTileAppearance,
    PreviousTileAppearance,
    RotateTile,
    RotateTileClockwise,
    TurnLeft,
    TurnRight,
    TurnLeft90,
    TurnRight90,
    DropToGround,
    DropOntoObject,
    SurfaceAbove,
    SurfaceBelow,
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
    /// The conversation editor's Search pane, its Find What field ready.
    DialogFind,
    /// The conversation editor's search, run again.
    DialogFindNext,
    /// File › Close.
    Close,
    /// File › Exit.
    Exit,
    CloseTab,
    NextTab,
    PreviousTab,
    ReopenTab,
    MaximizeWindow,
    SplitEditor,
    /// The keys to the next pane (the module tree, the middle, the
    /// palettes), or the one before.
    NextPane,
    PreviousPane,
    /// The module tree's and the palettes' Expand All and Collapse All,
    /// while the pane has the keys.
    ExpandAll,
    CollapseAll,
    ToggleTree,
    TogglePalettes,
    ToggleLog,
    HidePanels,
    NuiDelete,
    NuiDuplicate,
    NuiMoveUp,
    NuiMoveDown,
    NuiPrevious,
    NuiNext,
    NuiFold,
    NuiUnfold,
}

const CTRL: Modifiers = Modifiers::COMMAND;
const SHIFT: Modifiers = Modifiers::SHIFT;
const NONE: Modifiers = Modifiers::NONE;

impl Cmd {
    pub const ALL: [Cmd; 83] = [
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
        Cmd::CommandPalette,
        Cmd::QuickOpen,
        Cmd::SelectTiles,
        Cmd::PaintPlaceables,
        Cmd::Eraser,
        Cmd::RefineTile,
        Cmd::NextTileAppearance,
        Cmd::PreviousTileAppearance,
        Cmd::RotateTile,
        Cmd::RotateTileClockwise,
        Cmd::TurnLeft,
        Cmd::TurnRight,
        Cmd::TurnLeft90,
        Cmd::TurnRight90,
        Cmd::DropToGround,
        Cmd::DropOntoObject,
        Cmd::SurfaceAbove,
        Cmd::SurfaceBelow,
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
        Cmd::DialogFind,
        Cmd::DialogFindNext,
        Cmd::Close,
        Cmd::Exit,
        Cmd::CloseTab,
        Cmd::NextTab,
        Cmd::PreviousTab,
        Cmd::ReopenTab,
        Cmd::MaximizeWindow,
        Cmd::SplitEditor,
        Cmd::NextPane,
        Cmd::PreviousPane,
        Cmd::ExpandAll,
        Cmd::CollapseAll,
        Cmd::ToggleTree,
        Cmd::TogglePalettes,
        Cmd::ToggleLog,
        Cmd::HidePanels,
        Cmd::NuiDelete,
        Cmd::NuiDuplicate,
        Cmd::NuiMoveUp,
        Cmd::NuiMoveDown,
        Cmd::NuiPrevious,
        Cmd::NuiNext,
        Cmd::NuiFold,
        Cmd::NuiUnfold,
    ];

    /// The id the settings keep it by, its name and where it works. One
    /// that works anywhere has no name here: it is a command of the menus
    /// too, of the same id, and named there once
    /// ([`crate::commands::general_name`]; the two lists of names were
    /// kept alike by hand).
    fn about(self) -> (&'static str, Option<&'static str>, Group) {
        use Group::*;
        match self {
            Cmd::NewModule => ("new-module", None, General),
            Cmd::OpenModule => ("open-module", None, General),
            Cmd::Save => ("save", None, General),
            Cmd::Undo => ("undo", None, General),
            Cmd::Redo => ("redo", None, General),
            Cmd::ReplaceText => ("replace-text", None, General),
            Cmd::AreaWizard => ("area-wizard", None, General),
            Cmd::Factions => ("factions", None, General),
            Cmd::Journal => ("journal", None, General),
            Cmd::CompileAll => ("compile-all", None, General),
            Cmd::TestModule => ("test-module", None, General),
            Cmd::TestChoose => ("test-choose", None, General),
            Cmd::Manual => ("manual", None, General),
            Cmd::NewConversation => ("new-conversation", None, General),
            Cmd::NewScript => ("new-script", None, General),
            Cmd::CreatureWizard => ("creature-wizard", None, General),
            Cmd::ItemWizard => ("item-wizard", None, General),
            Cmd::FullScreen => ("full-screen", None, General),
            Cmd::CommandPalette => ("command-palette", None, General),
            Cmd::QuickOpen => ("quick-open", None, General),
            Cmd::SelectTiles => ("select-tiles", Some("Select Tiles or Objects"), Area),
            Cmd::PaintPlaceables => ("paint-placeables", Some("Paint Placeables"), Area),
            Cmd::Eraser => ("tile-eraser", Some("Eraser (Tiles)"), Area),
            Cmd::RefineTile => ("refine-tile", Some("Refine Tile"), Area),
            Cmd::NextTileAppearance => ("next-tile-appearance", Some("Next Tile Appearance"), Area),
            Cmd::PreviousTileAppearance => {
                ("previous-tile-appearance", Some("Previous Tile Appearance"), Area)
            }
            Cmd::RotateTile => ("rotate-tile", Some("Rotate Tile"), Area),
            Cmd::RotateTileClockwise => {
                ("rotate-tile-clockwise", Some("Rotate Tile Clockwise"), Area)
            }
            Cmd::TurnLeft => ("turn-left", Some("Turn Anticlockwise"), Area),
            Cmd::TurnRight => ("turn-right", Some("Turn Clockwise"), Area),
            Cmd::TurnLeft90 => ("turn-left-90", Some("Turn Anticlockwise 90°"), Area),
            Cmd::TurnRight90 => ("turn-right-90", Some("Turn Clockwise 90°"), Area),
            Cmd::DropToGround => ("drop", Some("Drop to the Ground"), Area),
            Cmd::DropOntoObject => ("drop-onto", Some("Drop onto the Object Below"), Area),
            Cmd::SurfaceAbove => ("surface-above", Some("Up to the Next Surface"), Area),
            Cmd::SurfaceBelow => ("surface-below", Some("Down to the Next Surface"), Area),
            Cmd::Overview => ("overview", Some("View the Whole Area"), Area),
            Cmd::CameraForward => ("camera-forward", Some("Move Camera Forward"), Area),
            Cmd::CameraBack => ("camera-back", Some("Move Camera Back"), Area),
            Cmd::CameraLeft => ("camera-left", Some("Move Camera Left"), Area),
            Cmd::CameraRight => ("camera-right", Some("Move Camera Right"), Area),
            Cmd::CameraTurnLeft => ("camera-turn-left", Some("Turn Camera Left"), Area),
            Cmd::CameraTurnRight => ("camera-turn-right", Some("Turn Camera Right"), Area),
            Cmd::CameraTiltUp => ("camera-tilt-up", Some("Tilt Camera Up"), Area),
            Cmd::CameraTiltDown => ("camera-tilt-down", Some("Tilt Camera Down"), Area),
            Cmd::CameraUp => ("camera-up", Some("Move Camera Up"), Area),
            Cmd::CameraDown => ("camera-down", Some("Move Camera Down"), Area),
            Cmd::Find => ("find", Some("Find"), Script),
            Cmd::Replace => ("replace", Some("Replace"), Script),
            Cmd::FindNext => ("find-next", Some("Find Next"), Script),
            Cmd::Bookmark => ("bookmark", Some("Toggle Bookmark"), Script),
            Cmd::Complete => ("complete", Some("Complete"), Script),
            Cmd::Definition => ("definition", Some("Go to Definition"), Script),
            Cmd::References => ("references", Some("Find References"), Script),
            Cmd::RenameSymbol => ("rename-symbol", Some("Rename Symbol"), Script),
            Cmd::AddLine => ("add-line", Some("Add Line"), Conversation),
            Cmd::DeleteLine => ("delete-line", Some("Delete Line"), Conversation),
            Cmd::DialogFind => ("dialog-find", Some("Find and Replace"), Conversation),
            Cmd::DialogFindNext => ("dialog-find-next", Some("Find Next"), Conversation),
            Cmd::Close => ("close", None, General),
            Cmd::Exit => ("exit", None, General),
            Cmd::CloseTab => ("close-tab", None, General),
            Cmd::NextTab => ("next-tab", None, General),
            Cmd::PreviousTab => ("previous-tab", None, General),
            Cmd::ReopenTab => ("reopen-tab", None, General),
            Cmd::MaximizeWindow => ("maximize-window", None, General),
            Cmd::SplitEditor => ("split-editor", None, General),
            Cmd::NextPane => ("next-pane", None, General),
            Cmd::PreviousPane => ("previous-pane", None, General),
            Cmd::ExpandAll => ("expand-all", Some("Expand All"), Panes),
            Cmd::CollapseAll => ("collapse-all", Some("Collapse All"), Panes),
            Cmd::ToggleTree => ("view-tree", None, General),
            Cmd::TogglePalettes => ("view-palettes", None, General),
            Cmd::ToggleLog => ("view-log", None, General),
            Cmd::HidePanels => ("hide-panels", None, General),
            Cmd::NuiDelete => ("nui-delete", Some("Delete Element"), Nui),
            Cmd::NuiDuplicate => ("nui-duplicate", Some("Duplicate Element"), Nui),
            Cmd::NuiMoveUp => ("nui-move-up", Some("Move Element Up"), Nui),
            Cmd::NuiMoveDown => ("nui-move-down", Some("Move Element Down"), Nui),
            Cmd::NuiPrevious => ("nui-previous", Some("Select Previous Element"), Nui),
            Cmd::NuiNext => ("nui-next", Some("Select Next Element"), Nui),
            Cmd::NuiFold => ("nui-fold", Some("Fold Element or Select Its Parent"), Nui),
            Cmd::NuiUnfold => ("nui-unfold", Some("Unfold Element or Select Its First Child"), Nui),
        }
    }

    pub fn id(self) -> &'static str {
        self.about().0
    }

    pub fn name(self) -> &'static str {
        let (id, name, _) = self.about();
        name.unwrap_or_else(|| crate::commands::general_name(id))
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
            Cmd::CommandPalette => vec![k(CTRL | SHIFT, Key::P)],
            // (Given its key after builders had given keys of their own:
            // it gives way to one given another command, `Keymap::give_way`.)
            Cmd::QuickOpen => vec![k(CTRL, Key::P)],
            Cmd::SelectTiles => vec![k(NONE, Key::F10)],
            Cmd::PaintPlaceables => vec![k(NONE, Key::B)],
            // (Moonglow's tools: Aurora has none of them but the Eraser,
            // which has no key there.)
            Cmd::Eraser => vec![k(NONE, Key::X)],
            Cmd::RefineTile => vec![k(NONE, Key::T)],
            Cmd::NextTileAppearance => vec![k(NONE, Key::N)],
            Cmd::PreviousTileAppearance => vec![k(SHIFT, Key::N)],
            Cmd::RotateTile => vec![k(NONE, Key::R)],
            Cmd::RotateTileClockwise => vec![k(SHIFT, Key::R)],
            Cmd::TurnLeft => vec![k(NONE, Key::Q)],
            Cmd::TurnRight => vec![k(NONE, Key::E)],
            Cmd::TurnLeft90 => vec![k(SHIFT, Key::Q)],
            Cmd::TurnRight90 => vec![k(SHIFT, Key::E)],
            Cmd::DropToGround => vec![k(NONE, Key::G)],
            Cmd::DropOntoObject => vec![k(SHIFT, Key::G)],
            Cmd::SurfaceAbove => vec![k(NONE, Key::PageUp)],
            Cmd::SurfaceBelow => vec![k(NONE, Key::PageDown)],
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
            // (And a Mac's delete key, which is Backspace: a laptop's has
            // no Delete key.)
            Cmd::DeleteLine if cfg!(target_os = "macos") => {
                vec![k(NONE, Key::Delete), k(NONE, Key::Backspace)]
            }
            Cmd::DeleteLine => vec![k(NONE, Key::Delete)],
            // Aurora's: Ctrl+F finds, Ctrl+R replaces (one pane here), F3
            // finds again (Ctrl+F4, its key for closing the module, closes a tab here).
            Cmd::DialogFind => vec![k(CTRL, Key::F), k(CTRL, Key::R)],
            Cmd::DialogFindNext => vec![k(NONE, Key::F3)],
            // (No key at first: Ctrl+F4, Aurora's for it, closes a tab.)
            Cmd::Close => Vec::new(),
            // Aurora's.
            Cmd::Exit => vec![k(Modifiers::ALT, Key::X)],
            // A browser's, both.
            Cmd::CloseTab => vec![k(CTRL, Key::W), k(CTRL, Key::F4)],
            Cmd::NextTab => vec![k(CTRL, Key::Tab), k(CTRL, Key::PageDown)],
            Cmd::PreviousTab => vec![k(CTRL | SHIFT, Key::Tab), k(CTRL, Key::PageUp)],
            Cmd::ReopenTab => vec![k(CTRL | SHIFT, Key::T)],
            Cmd::MaximizeWindow => vec![k(CTRL | SHIFT, Key::M)],
            Cmd::SplitEditor => vec![k(CTRL, Key::Backslash)],
            Cmd::NextPane => vec![k(NONE, Key::F6)],
            Cmd::PreviousPane => vec![k(SHIFT, Key::F6)],
            // (Right and Left open and close a row; with Ctrl+Shift,
            // every one. Ctrl and + or -, egui's zoom, are the window's.)
            Cmd::ExpandAll => vec![k(CTRL | SHIFT, Key::ArrowRight)],
            Cmd::CollapseAll => vec![k(CTRL | SHIFT, Key::ArrowLeft)],
            Cmd::ToggleTree => vec![k(CTRL | Modifiers::ALT, Key::Num1)],
            Cmd::TogglePalettes => vec![k(CTRL | Modifiers::ALT, Key::Num2)],
            Cmd::ToggleLog => vec![k(CTRL | Modifiers::ALT, Key::Num3)],
            Cmd::HidePanels => vec![k(CTRL | Modifiers::ALT, Key::Num0)],
            Cmd::NuiDelete => vec![k(NONE, Key::Delete)],
            Cmd::NuiDuplicate => vec![k(CTRL, Key::D)],
            Cmd::NuiMoveUp => vec![k(Modifiers::ALT, Key::ArrowUp)],
            Cmd::NuiMoveDown => vec![k(Modifiers::ALT, Key::ArrowDown)],
            Cmd::NuiPrevious => vec![k(NONE, Key::ArrowUp)],
            Cmd::NuiNext => vec![k(NONE, Key::ArrowDown)],
            Cmd::NuiFold => vec![k(NONE, Key::ArrowLeft)],
            Cmd::NuiUnfold => vec![k(NONE, Key::ArrowRight)],
        }
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

/// Where Find's keys are kept, for the search fields of the panes and the
/// windows ([`find_pressed`]).
fn find_keys_id() -> egui::Id {
    egui::Id::new("keymap-find-keys")
}

/// Whether one of Find's keys (Ctrl+F, or what Options › Keyboard gave
/// it) was pressed this frame; the press is used up. The search fields
/// read it, which have no key map at hand: [`Keymap::share_find`] keeps
/// its keys where they look.
pub fn find_pressed(ctx: &egui::Context) -> bool {
    let keys = ctx.data(|d| d.get_temp::<Vec<KeyboardShortcut>>(find_keys_id()));
    let keys = keys.unwrap_or_else(|| Cmd::Find.defaults());
    ctx.input_mut(|i| {
        let mut hit = false;
        i.events.retain(|e| {
            let this = !hit && keys.iter().any(|s| is(e, s));
            hit |= this;
            !this
        });
        hit
    })
}

/// A command's own keys, by the id its keys are kept under: a key
/// command's defaults, none for any other command.
pub fn defaults_of(id: &str) -> Vec<KeyboardShortcut> {
    Cmd::from_id(id).map(Cmd::defaults).unwrap_or_default()
}

/// The keys of every command: Moonglow's, but where the settings choose
/// others. Kept by command id: the key commands here ([`Cmd`]), and any
/// other command (the window's, which have no keys until given some).
#[derive(Debug, Clone)]
pub struct Keymap {
    keys: HashMap<String, Vec<KeyboardShortcut>>,
    /// The own keys of commands that are no key commands (a plugin's), and
    /// those a command that gives way was left ([`Keymap::give_way`]).
    defaults: HashMap<String, Vec<KeyboardShortcut>>,
    /// The commands whose keys the user chose (the settings', or Options ›
    /// Keyboard's since): theirs, whatever gives way.
    chosen_ids: std::collections::HashSet<String>,
}

/// Two key maps are one where every command has the same keys (and the
/// same own keys): which of them the user chose is no part of it.
impl PartialEq for Keymap {
    fn eq(&self, other: &Keymap) -> bool {
        self.keys == other.keys && self.defaults == other.defaults
    }
}

impl Eq for Keymap {}

/// The key commands whose own keys came after builders could give those
/// keys to other commands: where a command that works at the same moment
/// was given one of them (by the user, a plugin's manifest or a project's
/// file), the key stays that command's and this one goes without it
/// ([`Keymap::give_way`]). Quick Open's Ctrl+P.
const GIVES_WAY: [Cmd; 1] = [Cmd::QuickOpen];

impl Default for Keymap {
    fn default() -> Keymap {
        Keymap::new(&BTreeMap::new())
    }
}

/// The keys a list has taken from the commands that read keys held down
/// (the area camera's): [`Keymap::held_but`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Taken {
    /// The arrow keys.
    pub arrows: bool,
    /// The keys that type a character (letters, digits), without
    /// modifiers.
    pub letters: bool,
}

/// Whether a text field with the keyboard uses the key itself: any key
/// alone or with Shift but a function key and Escape (a letter, a digit
/// or a sign types; Space, Enter, Backspace, Delete, the arrows, Home and
/// End edit: a command given one of them fired in the field, which never
/// saw the key), and on a Mac (`mac`) Option with a key that types, which
/// there is a letter of its own (Option + X is `≈`, on a Polish layout
/// `ź`: it typed, and ran Alt + X's command).
pub(crate) fn field_s_key(s: &KeyboardShortcut, mac: bool) -> bool {
    let m = s.modifiers;
    let name = s.logical_key.name();
    let function =
        name.len() > 1 && name.starts_with('F') && name[1..].bytes().all(|b| b.is_ascii_digit());
    let command = m.command || m.ctrl || m.mac_cmd;
    if m.alt {
        return mac && !command && name.chars().count() == 1;
    }
    !command && !function && s.logical_key != Key::Escape
}

impl Keymap {
    /// The keys with the settings' choices (by command id; an empty list
    /// takes a command's keys away). Keys that don't read are left out; an
    /// id that is no command's now keeps its keys for when it is one.
    pub fn new(chosen: &BTreeMap<String, Vec<String>>) -> Keymap {
        let mut keys: HashMap<String, Vec<KeyboardShortcut>> =
            Cmd::ALL.into_iter().map(|c| (c.id().to_string(), c.defaults())).collect();
        for (id, list) in chosen {
            keys.insert(id.clone(), list.iter().filter_map(|t| from_text(t)).collect());
        }
        let chosen_ids = chosen.keys().cloned().collect();
        let mut map = Keymap { keys, defaults: HashMap::new(), chosen_ids };
        map.give_way();
        map
    }

    /// The key `cmd` gave way on ([`Keymap::give_way`]), and the id of the
    /// command that has it: what Options › Keyboard says on its row.
    pub fn gave_way_to(&self, cmd: Cmd) -> Option<(KeyboardShortcut, String)> {
        if !GIVES_WAY.contains(&cmd) || self.chosen_ids.contains(cmd.id()) {
            return None;
        }
        let group = |id: &str| Cmd::from_id(id).map_or(Group::General, Cmd::group);
        let mine = self.keys(cmd);
        cmd.defaults().into_iter().filter(|k| !mine.contains(k)).find_map(|k| {
            let mut ids: Vec<&String> = (self.keys.iter())
                .filter(|(id, keys)| {
                    id.as_str() != cmd.id() && group(id).overlaps(cmd.group()) && keys.contains(&k)
                })
                .map(|(id, _)| id)
                .collect();
            ids.sort();
            ids.first().map(|id| (k, (*id).clone()))
        })
    }

    /// The commands that give way ([`GIVES_WAY`]) without each of their
    /// own keys another command that works at the same moment has: unless
    /// the user chose their keys, they are left the rest (none, where
    /// that was all), as their own keys for now, so that the settings
    /// don't keep it (the key comes back once the other command lets it
    /// go). To be called again after a plugin's or a project's keys are
    /// taken in ([`Keymap::set_default`]).
    pub fn give_way(&mut self) {
        let group = |id: &str| Cmd::from_id(id).map_or(Group::General, Cmd::group);
        for cmd in GIVES_WAY {
            let id = cmd.id();
            if self.chosen_ids.contains(id) {
                continue;
            }
            let own = cmd.defaults();
            let taken = |k: &KeyboardShortcut| {
                self.keys.iter().any(|(other, keys)| {
                    other != id && group(other).overlaps(cmd.group()) && keys.contains(k)
                })
            };
            let left: Vec<KeyboardShortcut> = own.iter().filter(|k| !taken(k)).copied().collect();
            if left.len() == own.len() {
                self.defaults.remove(id);
            } else {
                self.defaults.insert(id.to_string(), left.clone());
            }
            self.keys.insert(id.to_string(), left);
        }
    }

    /// The own keys of a command that is no key command (a plugin's, from
    /// its manifest): they are its keys unless the settings chose others.
    pub fn set_default(&mut self, id: &str, keys: Vec<KeyboardShortcut>) {
        // (The settings' choice for it, if any, is in already.)
        self.keys.entry(id.to_string()).or_insert_with(|| keys.clone());
        self.defaults.insert(id.to_string(), keys);
    }

    /// A command's own keys: Moonglow's for a key command, its manifest's
    /// for a plugin's, none for any other.
    pub fn defaults_of(&self, id: &str) -> Vec<KeyboardShortcut> {
        self.defaults.get(id).cloned().unwrap_or_else(|| defaults_of(id))
    }

    /// Every command back to its own keys.
    pub fn reset_all(&mut self) {
        let ids: Vec<String> = self.keys.keys().cloned().collect();
        for id in ids {
            let own = self.defaults_of(&id);
            self.keys.insert(id, own);
        }
        self.chosen_ids.clear();
    }

    /// The keys as a file to hand on: the commands whose keys aren't
    /// their own, as the settings keep them.
    pub fn to_file(&self) -> String {
        let file = serde_json::json!({ "moonglow-keys": 1, "keys": self.chosen() });
        serde_json::to_string_pretty(&file).unwrap_or_default()
    }

    /// Takes the keys of a file [`Keymap::to_file`] wrote: every command
    /// back to its own keys, then the file's choices. How many commands
    /// it sets, or why it is no such file.
    pub fn take_file(&mut self, text: &str) -> Result<usize, String> {
        let file: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("it is not a key file: {e}"))?;
        if file.get("moonglow-keys").and_then(serde_json::Value::as_u64) != Some(1) {
            return Err("it is not a key file of Moonglow's".into());
        }
        let chosen: BTreeMap<String, Vec<String>> = file
            .get("keys")
            .cloned()
            .and_then(|k| serde_json::from_value(k).ok())
            .ok_or("it has no keys")?;
        self.reset_all();
        for (id, list) in &chosen {
            self.keys.insert(id.clone(), list.iter().filter_map(|t| from_text(t)).collect());
        }
        self.chosen_ids = chosen.keys().cloned().collect();
        self.give_way();
        Ok(chosen.len())
    }

    /// The settings' form: each command whose keys aren't Moonglow's.
    pub fn chosen(&self) -> BTreeMap<String, Vec<String>> {
        self.keys
            .iter()
            .filter(|(id, keys)| **keys != self.defaults_of(id))
            .map(|(id, keys)| (id.clone(), keys.iter().map(to_text).collect()))
            .collect()
    }

    /// The keys of the command kept under `id`.
    pub fn keys_of(&self, id: &str) -> Vec<KeyboardShortcut> {
        self.keys.get(id).cloned().unwrap_or_default()
    }

    pub fn keys(&self, cmd: Cmd) -> Vec<KeyboardShortcut> {
        self.keys_of(cmd.id())
    }

    pub fn set_of(&mut self, id: &str, keys: Vec<KeyboardShortcut>) {
        self.chosen_ids.insert(id.to_string());
        self.keys.insert(id.to_string(), keys);
    }

    pub fn set(&mut self, cmd: Cmd, keys: Vec<KeyboardShortcut>) {
        self.set_of(cmd.id(), keys);
    }

    /// The first key, as shown on this system, for menus and tooltips;
    /// empty for none.
    pub fn label_of(&self, id: &str, ctx: &egui::Context) -> String {
        let mac = ctx.os() == egui::os::OperatingSystem::Mac;
        self.keys.get(id).and_then(|k| k.first()).map(|s| shown(s, mac)).unwrap_or_default()
    }

    pub fn label(&self, cmd: Cmd, ctx: &egui::Context) -> String {
        self.label_of(cmd.id(), ctx)
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
    pub fn consume_of(&self, i: &mut InputState, id: &str) -> bool {
        let Some(keys) = self.keys.get(id) else { return false };
        let mut hit = false;
        i.events.retain(|e| {
            let this = !hit && keys.iter().any(|s| is(e, s));
            hit |= this;
            !this
        });
        hit
    }

    pub fn consume(&self, i: &mut InputState, cmd: Cmd) -> bool {
        self.consume_of(i, cmd.id())
    }

    /// Keeps Find's keys where the search fields read them
    /// ([`find_pressed`]), for this frame and the next.
    pub fn share_find(&self, ctx: &egui::Context) {
        let keys = self.keys(Cmd::Find);
        ctx.data_mut(|d| d.insert_temp(find_keys_id(), keys));
    }

    /// [`Keymap::consume_of`] for a command of the whole window: while a
    /// text field has the keyboard (`typing`), a key the field uses is
    /// the field's ([`field_s_key`]).
    pub fn consume_outside_text(&self, i: &mut InputState, id: &str, typing: bool) -> bool {
        let Some(keys) = self.keys.get(id) else { return false };
        let types = |s: &KeyboardShortcut| field_s_key(s, cfg!(target_os = "macos"));
        let mut hit = false;
        i.events.retain(|e| {
            let this = !hit && keys.iter().any(|s| is(e, s) && !(typing && types(s)));
            hit |= this;
            !this
        });
        hit
    }

    /// Whether `e` presses a key some command was given (a plugin's or a
    /// project's too): Alt and a letter is a menu's only where none was.
    pub fn bound(&self, e: &Event) -> bool {
        self.keys.values().chain(self.defaults.values()).flatten().any(|s| is(e, s))
    }

    /// Whether `e` presses a key of a command of `group`: a list's own key
    /// that a command there was given is the command's.
    pub fn bound_in(&self, group: Group, e: &Event) -> bool {
        let of = |c: &Cmd| c.group() == group;
        Cmd::ALL.iter().filter(|c| of(c)).any(|c| self.keys(*c).iter().any(|s| is(e, s)))
    }

    /// Whether one of the command's keys was pressed this frame (left for
    /// others).
    pub fn pressed(&self, i: &InputState, cmd: Cmd) -> bool {
        let Some(keys) = self.keys.get(cmd.id()) else { return false };
        i.events.iter().any(|e| keys.iter().any(|s| is(e, s)))
    }

    /// Whether one of the command's keys is held down. A key without
    /// modifiers counts with none held, so Ctrl+S doesn't move the
    /// camera; and no key counts while a text field has the keyboard
    /// (`typing`): the arrows move its cursor, with Ctrl or Shift too.
    pub fn held(&self, i: &InputState, cmd: Cmd, typing: bool) -> bool {
        self.held_but(i, cmd, typing, Taken::default())
    }

    /// [`held`](Self::held), but for the keys a list takes (the palette's
    /// tree moves by the arrows after a click in it; a letter typed in the
    /// conversation tree or the module tree goes to a line or a row): the
    /// command's other keys go on working.
    pub fn held_but(&self, i: &InputState, cmd: Cmd, typing: bool, taken: Taken) -> bool {
        use egui::Key::{ArrowDown, ArrowLeft, ArrowRight, ArrowUp};
        let Some(keys) = self.keys.get(cmd.id()) else { return false };
        !typing
            && keys.iter().any(|s| {
                let plain = s.modifiers.is_none();
                let text = s.logical_key.name().chars().count() == 1;
                let arrow = matches!(s.logical_key, ArrowDown | ArrowLeft | ArrowRight | ArrowUp);
                !(taken.arrows && arrow)
                    && !(taken.letters && text && plain)
                    && i.key_down(s.logical_key)
                    && if plain {
                        !(text && (i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt))
                    } else {
                        i.modifiers.matches_exact(s.modifiers)
                    }
            })
    }

    /// Keys that two commands working at the same moment share: the key
    /// and the two commands' ids. A command that is no key command works
    /// anywhere in the window.
    pub fn conflicts(&self) -> Vec<(KeyboardShortcut, String, String)> {
        let group = |id: &str| Cmd::from_id(id).map_or(Group::General, Cmd::group);
        // The key commands in their order, then the others by id.
        let mut ids: Vec<&String> = self.keys.keys().collect();
        ids.sort_by_key(|id| {
            (Cmd::ALL.iter().position(|c| c.id() == id.as_str()).unwrap_or(usize::MAX), *id)
        });
        let mut out = Vec::new();
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                if !group(a).overlaps(group(b)) {
                    continue;
                }
                for k in &self.keys[*a] {
                    if self.keys[*b].contains(k) {
                        out.push((*k, (*a).clone(), (*b).clone()));
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
    fn keys_go_to_a_file_and_come_back() {
        let mut keys = Keymap::default();
        let cmd = Cmd::ALL[0];
        let own = keys.keys(cmd);
        let chosen = from_text("Ctrl+Alt+F9").unwrap();
        keys.set(cmd, vec![chosen]);
        let file = keys.to_file();
        // Into a key map with another choice of its own: the file's
        // replace it, and what the file doesn't name is its own again.
        let mut other = Keymap::default();
        let second = Cmd::ALL[1];
        other.set(second, Vec::new());
        assert_eq!(other.take_file(&file), Ok(1));
        assert_eq!(other.keys(cmd), [chosen]);
        assert_eq!(other.keys(second), second.defaults());
        assert_ne!(own, [chosen]);
        assert!(other.take_file("{}").is_err());
        assert!(other.take_file("not json").is_err());
    }

    /// A command that is no key command has no keys until given some, keeps
    /// them through the settings, and counts in what conflicts.
    #[test]
    fn any_command_takes_keys_by_its_id() {
        let mut map = Keymap::default();
        assert!(map.keys_of("build-module").is_empty());
        let f8 = KeyboardShortcut::new(NONE, Key::F8);
        map.set_of("build-module", vec![f8]);
        let chosen = map.chosen();
        assert_eq!(chosen.get("build-module"), Some(&vec!["F8".to_string()]));
        assert_eq!(chosen.len(), 1, "{chosen:?}");
        let back = Keymap::new(&chosen);
        assert_eq!(back, map);
        assert_eq!(back.keys_of("build-module"), [f8]);
        // The same key on a key command that works anywhere: named.
        map.set(Cmd::Manual, vec![f8]);
        assert_eq!(map.conflicts(), [(f8, "manual".to_string(), "build-module".to_string())]);
        // Taken away again: nothing kept for it.
        map.set_of("build-module", Vec::new());
        assert!(!map.chosen().contains_key("build-module"));
    }

    /// Quick Open's Ctrl+P gives way to a command the user, a plugin or a
    /// project gave it: Quick Open is then without a key, and the settings
    /// don't keep that (it has Ctrl+P back once the other lets it go).
    /// Given by the user to Quick Open too, both keep it: a conflict.
    #[test]
    fn quick_open_gives_ctrl_p_to_a_command_given_it() {
        let ctrl_p = KeyboardShortcut::new(Modifiers::COMMAND, Key::P);
        assert_eq!(Keymap::default().keys(Cmd::QuickOpen), [ctrl_p]);
        let mut chosen = BTreeMap::new();
        chosen.insert("build-module".to_string(), vec!["Ctrl+P".to_string()]);
        let map = Keymap::new(&chosen);
        assert_eq!(map.keys_of("build-module"), [ctrl_p]);
        assert!(map.keys(Cmd::QuickOpen).is_empty(), "the user's key wins");
        assert!(map.conflicts().is_empty(), "{:?}", map.conflicts());
        assert_eq!(map.chosen(), chosen, "nothing kept for Quick Open");
        // Let go of, Quick Open has it again.
        assert_eq!(Keymap::new(&BTreeMap::new()).keys(Cmd::QuickOpen), [ctrl_p]);
        let mut reset = map.clone();
        reset.reset_all();
        reset.give_way();
        assert_eq!(reset.keys(Cmd::QuickOpen), [ctrl_p]);
        // (An area's command too: the window's keys work over the area.)
        let mut area = BTreeMap::new();
        area.insert("camera-up".to_string(), vec!["Ctrl+P".to_string()]);
        assert!(Keymap::new(&area).keys(Cmd::QuickOpen).is_empty());
        // A plugin's own key, and a project's, win too.
        let mut map = Keymap::default();
        map.set_default("some-plugin.go", vec![ctrl_p]);
        map.give_way();
        assert!(map.keys(Cmd::QuickOpen).is_empty());
        assert!(map.chosen().is_empty(), "{:?}", map.chosen());
        // Chosen for Quick Open by the user as well: both have it, said.
        chosen.insert("quick-open".to_string(), vec!["Ctrl+P".to_string()]);
        let map = Keymap::new(&chosen);
        assert_eq!(map.keys(Cmd::QuickOpen), [ctrl_p]);
        assert_eq!(map.conflicts().len(), 1);
        // Another key the user gives it is kept.
        let mut map = Keymap::default();
        let f1 = KeyboardShortcut::new(Modifiers::COMMAND, Key::F1);
        map.set(Cmd::QuickOpen, vec![f1]);
        map.set_of("build-module", vec![ctrl_p]);
        map.give_way();
        assert_eq!(map.keys(Cmd::QuickOpen), [f1]);
    }

    /// A letter given to a command of the window stays a letter while a
    /// text field has the keyboard; a key with Ctrl, or a function key,
    /// works there too.
    #[test]
    fn letters_are_a_text_field_s_while_it_has_the_keyboard() {
        let press = |key: Key, modifiers: Modifiers| {
            let mut i = InputState::default();
            i.events.push(Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            });
            i
        };
        let mut map = Keymap::default();
        map.set_of("a", vec![KeyboardShortcut::new(NONE, Key::B)]);
        map.set_of("b", vec![KeyboardShortcut::new(SHIFT, Key::B)]);
        map.set_of("c", vec![KeyboardShortcut::new(CTRL, Key::B)]);
        map.set_of("d", vec![KeyboardShortcut::new(NONE, Key::F8)]);
        map.set_of("e", vec![KeyboardShortcut::new(NONE, Key::Delete)]);
        map.set_of("f", vec![KeyboardShortcut::new(NONE, Key::Space)]);
        map.set_of("g", vec![KeyboardShortcut::new(SHIFT, Key::ArrowDown)]);
        map.set_of("h", vec![KeyboardShortcut::new(CTRL, Key::Delete)]);
        // (Option and a letter types on a Mac; Alt and a letter is a
        // command's elsewhere, Escape and the function keys everywhere.)
        let alt_x = KeyboardShortcut::new(Modifiers::ALT, Key::X);
        assert!(field_s_key(&alt_x, true) && !field_s_key(&alt_x, false));
        assert!(!field_s_key(&KeyboardShortcut::new(NONE, Key::Escape), true));
        assert!(!field_s_key(&KeyboardShortcut::new(NONE, Key::F12), false));
        for (id, key, modifiers, when_typing) in [
            ("a", Key::B, NONE, false),
            ("b", Key::B, SHIFT, false),
            ("c", Key::B, CTRL, true),
            ("d", Key::F8, NONE, true),
            ("e", Key::Delete, NONE, false),
            ("f", Key::Space, NONE, false),
            ("g", Key::ArrowDown, SHIFT, false),
            ("h", Key::Delete, CTRL, true),
        ] {
            assert!(map.consume_outside_text(&mut press(key, modifiers), id, false), "{id}");
            let mut typing = press(key, modifiers);
            assert_eq!(map.consume_outside_text(&mut typing, id, true), when_typing, "{id}");
            assert_eq!(typing.events.is_empty(), when_typing, "{id}: the field keeps its key");
        }
    }

    /// A conversation command's keys, as the conversation tree asks before
    /// it takes one of its own: Delete Line's Delete, and on macOS its
    /// delete key (Backspace) too.
    #[test]
    fn keys_bound_in_a_group() {
        let key = |key: Key, modifiers: Modifiers| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        };
        let map = Keymap::default();
        let conversation = |e: &Event| map.bound_in(Group::Conversation, e);
        assert!(conversation(&key(Key::Delete, NONE)));
        assert!(!conversation(&key(Key::Delete, SHIFT)));
        assert!(!conversation(&key(Key::Q, NONE)), "an area's");
        assert_eq!(conversation(&key(Key::Backspace, NONE)), cfg!(target_os = "macos"));
    }

    /// The arrows move the camera, but a text field's cursor while one has
    /// the keyboard (a window over the area view).
    #[test]
    fn no_key_moves_the_camera_while_a_text_field_has_the_keyboard() {
        let map = Keymap::default();
        let mut i = InputState::default();
        i.keys_down.insert(Key::ArrowLeft);
        assert!(map.held(&i, Cmd::CameraLeft, false));
        assert!(!map.held(&i, Cmd::CameraLeft, true));
        // The arrows taken (the palette's): the command's other keys work.
        let arrows = Taken { arrows: true, letters: false };
        assert!(!map.held_but(&i, Cmd::CameraLeft, false, arrows));
        i.keys_down.insert(Key::A);
        assert!(map.held_but(&i, Cmd::CameraLeft, false, arrows));
        // The letters taken (a tree's): the arrow works, not A, nor the
        // keypad's 4.
        let letters = Taken { arrows: false, letters: true };
        assert!(map.held_but(&i, Cmd::CameraLeft, false, letters));
        i.keys_down.remove(&Key::ArrowLeft);
        assert!(!map.held_but(&i, Cmd::CameraLeft, false, letters));
        i.keys_down.remove(&Key::A);
        i.keys_down.insert(Key::Num4);
        assert!(!map.held_but(&i, Cmd::CameraLeft, false, letters));
        assert!(map.held_but(&i, Cmd::CameraLeft, false, Taken::default()));
    }

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
