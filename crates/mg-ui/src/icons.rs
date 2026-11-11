//! The drawn icons (`mg_uikit::Icon`) that label the object types (in the
//! palette and the area view's toolbar), the area view's toggles and the
//! toolbars' buttons, and [`Tools`], which labels a toolbar's buttons as
//! Options › General › Toolbar buttons says.

use mg_area::ObjectKind;
use mg_uikit::Controls as _;
pub(crate) use mg_uikit::Icon;
use mg_uikit::{IconButton, Label};

/// Declares icons, each a constant, and `$all`, the list of them: an icon
/// is in the list by being declared, so the tests over every icon
/// (`every_icon_used_is_drawn`) can't miss one.
macro_rules! icons {
    ($all:ident: $($(#[$doc:meta])* $name:ident = $icon:ident;)*) => {
        $($(#[$doc])* pub(crate) const $name: Icon = Icon::$icon;)*
        /// Every one.
        #[cfg(test)]
        pub(crate) const $all: &[Icon] = &[$($name),*];
    };
}

icons! {
    ALL:
    /// The palette's Tiles.
    TILES = Tile;
    ERASER = Eraser;
    RAISE_LOWER = Terrain;
    REFINE = Sparkles;
    /// The tiles palette's Next and Previous Tile Appearance (the design's
    /// "‹ Appearance ›") and Rotate Tile (a quarter turn counter-clockwise).
    NEXT_APPEARANCE = ChevronRight;
    PREVIOUS_APPEARANCE = ChevronLeft;
    ROTATE = Restart;
    /// The start location (the area view's Start, the palette's, and Go
    /// to Start Location, as the design has them).
    START = Start;
    NIGHT = Moon;
    FOG = Fog;
    /// The area view's Tile Border (what the game draws past the edge).
    TILE_BORDER = TileBorder;
    /// The area view's Shadows switch (and the model view's Shadow).
    SHADOWS = Shadow;
    GRID = Grid;
    GRASS = Grass;
    /// The game's own shaders (the area and model views' Shaders switch).
    SHADERS = Shaders;
    AMBIENT = Ambient;
    MUSIC = Music;
    WALKMESH = Walkmesh;
    SELECT_TILES = SelectTiles;
    PAINT = Paint;
    /// The area view's Area Properties (the main toolbar's Module Properties
    /// is `module`: `area-properties` and `properties` are one drawing).
    AREA_PROPERTIES = AreaProperties;
    /// A palette's Gallery and List (its pictures or its names).
    GALLERY = Gallery;
    LIST = List;
    /// The area view's Show All and Show None.
    SHOW_ALL = Eye;
    SHOW_NONE = EyeOff;
    /// Together (turning several as one), To Scratch, Onto Objects (what is
    /// placed goes onto the placeable or door under the pointer), and the
    /// dropdowns' icons with Icons only (Fade Geometry, Snap, Turn).
    TOGETHER = Together;
    TO_SCRATCH = ToScratch;
    ONTO_OBJECTS = OntoObjects;
    FADE = Fade;
    SNAP = Magnet;
    TURN = Rotate;
    CAMERA = Camera;
    LIGHTING = Lightbulb;
    ANIMATIONS = Animations;
    /// The area view's play-sounds switch (the sound objects' filter is
    /// `sound`).
    SOUNDS = Speaker;
    SOUND_RANGES = SoundRanges;
    /// The area view's Minimap (the area's map in its corner).
    MINIMAP = Minimap;
    /// An area another builder has claimed, one this builder has, and one
    /// merged (its Tag and ResRef kept): the module tree's marks.
    CLAIMED = Lock;
    CLAIMED_MINE = Check;
    MERGED = Pin;
    /// Prefabs: groups of placed objects kept to place again.
    PREFABS = Prefab;
    /// The palette's lights (the game's invisible light placeables), a sun
    /// as the area view marks them.
    LIGHTS = Sun;
    /// A favorite in a palette's list (the design's bookmark mark).
    FAVORITE = Bookmark;
    /// The palette's Exact Tile (the tile put down as chosen).
    EXACT_TILE = Pin;
    /// A window's bar: Maximize, and Restore while it is maximized.
    MAXIMIZE = Maximize;
    RESTORE = Restore;
}

/// An object type's icon (the set's kind icons, the same everywhere a kind
/// is shown).
pub(crate) fn object(kind: ObjectKind) -> Icon {
    match kind {
        ObjectKind::Creature => Icon::Creature,
        ObjectKind::Door => Icon::Door,
        ObjectKind::Encounter => Icon::Encounter,
        ObjectKind::Item => Icon::Item,
        ObjectKind::Placeable => Icon::Placeable,
        ObjectKind::Sound => Icon::Sound,
        ObjectKind::Store => Icon::Store,
        ObjectKind::Trigger => Icon::Trigger,
        ObjectKind::Waypoint => Icon::Waypoint,
    }
}

/// The editors' toolbars' icons, with Icons only (beside the others,
/// their words alone): the script editor's, the conversation editor's,
/// the model view's and the NUI Creator's.
pub(crate) mod editor {
    use super::Icon;
    icons! {
        ALL:
        SAVE = Save;
        COMPILE = Compile;
        SAVE_AS = SaveAs;
        PRINT = Print;
        OPEN = Open;
        EXTERNAL = ExternalEditor;
        FIND = Find;
        REPLACE = Replace;
        BOOKMARK = Bookmark;
        DEFINITION = Definition;
        REFERENCES = References;
        RENAME = Rename;
        USED_BY = UsedBy;
        ADD = Plus;
        COPY = Copy;
        CUT = Cut;
        PASTE = Paste;
        PASTE_LINK = Link;
        DELETE = Delete;
        EXPAND_ALL = ExpandAll;
        COLLAPSE_ALL = CollapseAll;
        COMMENTS = Comment;
        ACTIONS = Bolt;
        QUESTS = Journal;
        ANIMATIONS = Animations;
        SOUNDS = Speaker;
        EXPORT = Export;
        IMPORT = Import;
        SCRIPTS = Script;
        PLAY = Play;
        PAUSE = Pause;
        FRAME = Frame;
        LIGHTS = Light;
        POP_OUT = OpenInTab;
        BUILD = Hammer;
        HOW_TO_OPEN = Help;
    }
}

/// How a toolbar's buttons are labelled (Options › General › Toolbar
/// buttons): a glyph and its name, or, with Icons only, the glyph alone
/// and its name in its tip (and as its name for a screen reader), as
/// Aurora's toolbars have them. The main toolbar and the area view's take
/// it from the settings each frame; the editors' toolbars show their
/// words alone beside them ([`Tools::word`]).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Tools {
    pub(crate) icons_only: bool,
    /// The area toolbar's buttons left out ([`AREA_TOOLS`], a bit each by
    /// its place there; none on other toolbars).
    hidden: u64,
}

/// The area view's toolbar's buttons that can be hidden (Options › Area ›
/// Toolbar, the toolbar's right-click menu), by the names they give
/// [`Tools`]; the objects' filters are [`area_tool_kind`]'s.
pub(crate) const AREA_TOOLS: [&str; 29] = [
    "Start",
    "All",
    "None",
    "Night",
    "Lighting",
    "Shadows",
    "Fog",
    "Fade Geometry",
    "Animations",
    "Grass",
    "Tile Border",
    "Shaders",
    "Grid",
    "Minimap",
    "Sounds",
    "Sound Ranges",
    "Ambient",
    "Music",
    "Snap",
    "Turn",
    "Together",
    "Onto Objects",
    "Walkmesh…",
    "Select Tiles",
    "Paint",
    "Area Properties",
    "Reorient Camera",
    "Go to Start Location",
    "To Scratch",
];

/// The name an objects' filter of the area toolbar is hidden by: "Show
/// Creatures" (the play-sounds button is "Sounds").
pub(crate) fn area_tool_kind(label: &str) -> String {
    format!("Show {label}")
}

impl Tools {
    pub(crate) fn of(settings: &crate::Settings) -> Tools {
        Tools { icons_only: settings.toolbar_icons_only, hidden: 0 }
    }

    /// The same, with the icons alone where `buttons` with their words
    /// would not fit in one row of `ui`'s width (the design's rule: a
    /// header's words yield to icons before it wraps; what still has no
    /// room wraps to the next row).
    pub(crate) fn fitted(self, ui: &egui::Ui, buttons: &[IconButton]) -> Tools {
        self.fitted_after(ui, 0.0, buttons)
    }

    /// [`Tools::fitted`] for a row that begins with `before` points of
    /// something else (the hak editor's count of what it holds, and the
    /// room after it).
    pub(crate) fn fitted_after(self, ui: &egui::Ui, before: f32, buttons: &[IconButton]) -> Tools {
        let gaps = ui.spacing().item_spacing.x * buttons.len().saturating_sub(1) as f32;
        let words: f32 = buttons.iter().map(|b| b.width(ui)).sum::<f32>() + gaps;
        Tools { icons_only: self.icons_only || before + words > ui.available_width(), ..self }
    }

    /// The area view's toolbar's: those its settings hide left out.
    pub(crate) fn of_area(settings: &crate::Settings) -> Tools {
        let hidden = (AREA_TOOLS.iter().enumerate())
            .filter(|(_, n)| settings.hidden_area_tools.iter().any(|h| h == *n))
            .fold(0, |m, (i, _)| m | 1 << i);
        Tools { icons_only: settings.toolbar_icons_only, hidden }
    }

    /// Whether the button named `name` is left out.
    pub(crate) fn hides(self, name: &str) -> bool {
        AREA_TOOLS.iter().position(|n| *n == name).is_some_and(|i| self.hidden & 1 << i != 0)
    }

    /// The same, hiding nothing (the objects' filters, named as buttons
    /// that may be hidden).
    pub(crate) fn showing_all(self) -> Tools {
        Tools { hidden: 0, ..self }
    }

    /// How a button of the main or the area toolbar is labelled: its icon
    /// and its name, or its icon alone.
    pub(crate) fn look(self) -> Label {
        if self.icons_only { Label::IconOnly } else { Label::Both }
    }

    /// How an editor's toolbar button is labelled: its words alone, or with
    /// Icons only its icon alone.
    pub(crate) fn word_look(self) -> Label {
        if self.icons_only { Label::IconOnly } else { Label::WordsOnly }
    }

    /// A button of the main or the area toolbar: `icon` and `name`, or the
    /// icon alone ([`Tools::look`]), named `name` either way.
    pub(crate) fn widget(self, icon: Icon, name: &str) -> IconButton {
        // (4 px between the icon and the words, not the mock's 6: the area
        // toolbar's forty buttons took a row more with 6, in today's
        // layout, until M3 moves its switches into popovers.)
        IconButton::new(icon, name).label(self.look()).gap(mg_uikit::metrics::GAP_S)
    }

    /// An editor's toolbar button: its words, or its icon alone
    /// ([`Tools::word_look`]), named `name` either way.
    pub(crate) fn word_widget(self, icon: Icon, name: &str) -> IconButton {
        IconButton::new(icon, name).label(self.word_look())
    }

    /// A button's tip from `tip` (with its key, where it has one; empty:
    /// none): as it is beside the name; with Icons only, the name first
    /// (unless the tip begins with it), or the name alone.
    pub(crate) fn tip(self, name: &str, tip: &str) -> Option<String> {
        let begins = tip.get(..name.len()).is_some_and(|t| t.eq_ignore_ascii_case(name));
        match (self.icons_only, tip.is_empty()) {
            (false, true) => None,
            (false, false) => Some(tip.to_string()),
            (true, true) => Some(name.to_string()),
            (true, false) if begins => Some(tip.to_string()),
            (true, false) => Some(format!("{name}\n{tip}")),
        }
    }

    /// A button: `icon`, named `name`, with `tip`.
    pub(crate) fn button(
        self,
        ui: &mut egui::Ui,
        icon: Icon,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        if self.hides(name) {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
        }
        self.add(ui, self.widget(icon, name), name, tip)
    }

    /// A button that stays down while `on`, and turns it on and off.
    pub(crate) fn toggle(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        icon: Icon,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        self.toggle_as(ui, on, icon, name, name, tip)
    }

    /// [`Tools::toggle`] showing `words` (by which it is hidden) and named
    /// `name` for a screen reader and in its tip: the area toolbar's
    /// "Sounds" filter named "Show Sounds", its "Sounds" switch "Play
    /// Sounds".
    pub(crate) fn toggle_as(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        icon: Icon,
        words: &str,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        if self.hides(words) {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
        }
        let r = toggled(ui.add(self.widget(icon, words).name(name).selectable(*on)), on);
        self.with_tip(r, name, tip)
    }

    /// An editor's toolbar toggle: its words, or its icon ([`Tools::word_look`]).
    pub(crate) fn toggle_word(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        icon: Icon,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = toggled(ui.add(self.word_widget(icon, name).selectable(*on)), on);
        self.with_tip(r, name, tip)
    }

    /// An editor's toolbar tick: a check box and its words, or with Icons
    /// only its icon as a toggle (on in the accent), named `name` either
    /// way.
    pub(crate) fn checkbox_word(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        icon: Icon,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = match self.icons_only {
            true => toggled(ui.add(self.word_widget(icon, name).selectable(*on)), on),
            false => ui.check_box(on, name),
        };
        self.with_tip(r, name, tip)
    }

    /// `button` (an [`IconButton`], named by its words) added, with its tip.
    pub(crate) fn add(
        self,
        ui: &mut egui::Ui,
        button: impl egui::Widget,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = ui.add(button);
        self.with_tip(r, name, tip)
    }

    /// `r`, a button, with its tip.
    pub(crate) fn named(self, r: egui::Response, name: &str, tip: &str) -> egui::Response {
        self.with_tip(r, name, tip)
    }

    /// `r` with its tip, shown while it is disabled too (an icon alone,
    /// grayed, says least).
    fn with_tip(self, r: egui::Response, name: &str, tip: &str) -> egui::Response {
        match self.tip(name, tip) {
            Some(tip) => r.on_hover_text(&tip).on_disabled_hover_text(tip),
            None => r,
        }
    }

    /// Every icon the toolbars, the palettes and the module tree show: the
    /// main toolbar's, the object types', and every one declared here
    /// ([`ALL`], [`editor::ALL`]).
    #[cfg(test)]
    pub(crate) fn icons() -> Vec<Icon> {
        let mut all: Vec<Icon> = crate::commands::TOOLBAR.iter().flatten().map(|t| t.1).collect();
        all.extend(ObjectKind::ALL.map(object));
        all.extend(ALL);
        all.extend(editor::ALL);
        all
    }
}

/// `r`, a button that toggles `on`: turned when clicked, and changed.
pub(crate) fn toggled(mut r: egui::Response, on: &mut bool) -> egui::Response {
    if r.clicked() {
        *on = !*on;
        r.mark_changed();
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With Icons only, the icon alone and the name in the tip: first,
    /// unless the tip begins with it, or alone; beside the name, the tip
    /// as it was.
    #[test]
    fn icons_only_moves_the_name_into_the_tip() {
        let both = Tools { icons_only: false, hidden: 0 };
        let icons = Tools { icons_only: true, hidden: 0 };
        assert_eq!(both.look(), Label::Both);
        assert_eq!(icons.look(), Label::IconOnly);
        assert_eq!(both.word_look(), Label::WordsOnly);
        assert_eq!(icons.word_look(), Label::IconOnly);
        assert_eq!(both.tip("Reorient Camera", ""), None);
        assert_eq!(icons.tip("Reorient Camera", "").as_deref(), Some("Reorient Camera"));
        assert_eq!(both.tip("Grid", "Display Grid").as_deref(), Some("Display Grid"));
        assert_eq!(icons.tip("Grid", "Display Grid").as_deref(), Some("Grid\nDisplay Grid"));
        assert_eq!(icons.tip("Undo", "Undo (Ctrl+Z)").as_deref(), Some("Undo (Ctrl+Z)"));
        assert_eq!(icons.tip("Save", "Save module").as_deref(), Some("Save module"));
        // (A tip shorter than the name, or cut inside a character.)
        assert_eq!(icons.tip("Properties", "é").as_deref(), Some("Properties\né"));
        assert_eq!(icons.tip("ab", "é").as_deref(), Some("ab\né"));
    }

    /// An icon is one command's across the main toolbar and the area
    /// view's (Module Properties `module`, Area Properties
    /// `area-properties`; the area's Grid `grid`, a palette's Gallery
    /// `gallery`): the one they share is the start location's, which the
    /// area view's Start shows and Go to Start Location goes to, as the
    /// design has them. Compared by drawing: the set's aliases (`lightbulb`
    /// and `light`, `hammer` and `compile`) are one picture.
    #[test]
    fn an_icon_is_one_command_s() {
        let drawing = |i: Icon| i.polylines().map(|(p, c)| (p.to_vec(), c)).collect::<Vec<_>>();
        let drawn = |icons: &[Icon]| {
            let mut all: Vec<_> = icons.iter().map(|i| format!("{:?}", drawing(*i))).collect();
            all.sort_unstable();
            all
        };
        let main: Vec<Icon> = crate::commands::TOOLBAR.iter().flatten().map(|t| t.1).collect();
        let area = [
            START,
            NIGHT,
            LIGHTING,
            SHADOWS,
            FOG,
            FADE,
            TILE_BORDER,
            ANIMATIONS,
            GRASS,
            SHADERS,
            GRID,
            SOUNDS,
            SOUND_RANGES,
            MINIMAP,
            AMBIENT,
            MUSIC,
            SNAP,
            TURN,
            WALKMESH,
            SELECT_TILES,
            PAINT,
            CAMERA,
            AREA_PROPERTIES,
            SHOW_ALL,
            SHOW_NONE,
            TOGETHER,
            ONTO_OBJECTS,
            TO_SCRATCH,
            PREFABS,
            LIGHTS,
        ];
        let main_drawn = drawn(&main);
        let shared: Vec<Icon> = (area.into_iter())
            .filter(|i| main_drawn.contains(&format!("{:?}", drawing(*i))))
            .collect();
        assert_eq!(shared, [START]);
        let mut seen = area.to_vec();
        seen.extend(ObjectKind::ALL.map(object));
        seen.push(GALLERY);
        seen.push(LIST);
        // (Two icons drawn alike, by name.)
        let alike = |icons: &[Icon]| {
            let mut pairs = Vec::new();
            for (i, a) in icons.iter().enumerate() {
                for b in &icons[i + 1..] {
                    if drawing(*a) == drawing(*b) {
                        pairs.push((a.name(), b.name()));
                    }
                }
            }
            pairs
        };
        assert_eq!(alike(&seen), [], "one icon for two of the area's buttons");
        use editor as e;
        let editors: [&[Icon]; 4] = [
            &[
                e::SAVE,
                e::COMPILE,
                e::SAVE_AS,
                e::PRINT,
                e::OPEN,
                e::EXTERNAL,
                TO_SCRATCH,
                e::FIND,
                e::REPLACE,
                e::BOOKMARK,
                e::DEFINITION,
                e::REFERENCES,
                e::RENAME,
                e::USED_BY,
            ],
            &[
                e::ADD,
                e::COPY,
                e::CUT,
                e::PASTE,
                e::PASTE_LINK,
                e::DELETE,
                e::EXPAND_ALL,
                e::COLLAPSE_ALL,
                e::COMMENTS,
                e::ACTIONS,
                e::QUESTS,
                e::ANIMATIONS,
                e::SOUNDS,
                e::EXPORT,
                e::IMPORT,
                e::SCRIPTS,
            ],
            &[e::PLAY, e::PAUSE, e::FRAME, e::LIGHTS, SHADOWS, SHADERS, e::POP_OUT, e::EXPORT],
            &[e::IMPORT, e::EXPORT, e::BUILD, e::HOW_TO_OPEN],
        ];
        for icons in editors {
            assert_eq!(alike(icons), [], "one icon for two of an editor's buttons");
        }
    }

    /// Every icon the application names is one of the set's
    /// (`Icon::ALL`), and each paints strokes at 16 px: those declared
    /// here, the main toolbar's, and every icon named (`Icon::` and its name) in the
    /// application's source.
    #[test]
    fn every_icon_used_is_drawn() {
        let mut used = Tools::icons();
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut files = vec![std::path::PathBuf::from(src)];
        let mut named = 0;
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for (at, _) in text.match_indices("Icon::") {
                // (Not egui's `CursorIcon::`.)
                if text[..at].ends_with(|c: char| c.is_ascii_alphanumeric()) {
                    continue;
                }
                let name: String = text[at + 6..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    continue;
                }
                if name == "ALL" {
                    continue;
                }
                let icon = Icon::ALL.into_iter().find(|i| format!("{i:?}") == name);
                let icon = icon.unwrap_or_else(|| panic!("{}: Icon::{name}", path.display()));
                used.push(icon);
                named += 1;
            }
        }
        assert!(named > 50, "{named} icons named in the source");
        let ctx = egui::Context::default();
        let rect = egui::Rect::from_min_size(egui::pos2(4.0, 4.0), egui::vec2(16.0, 16.0));
        let shapes = |icon: Option<Icon>| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                if let Some(icon) = icon {
                    icon.paint(ui.painter(), rect, egui::Color32::WHITE);
                }
            });
            out.textures_delta.clear();
            out.shapes.len()
        };
        let none = shapes(None);
        used.sort_unstable();
        used.dedup();
        for icon in used {
            assert!(Icon::ALL.contains(&icon));
            assert!(shapes(Some(icon)) > none, "{} paints nothing", icon.name());
        }
    }
}
