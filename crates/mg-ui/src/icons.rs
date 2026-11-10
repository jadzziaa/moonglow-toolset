//! The glyphs that label the object types (in the palette and the area
//! view's toolbar) and the area view's toggles, all in egui's built-in
//! fonts.

use mg_area::ObjectKind;

/// Declares glyphs, each a constant, and `$all`, the list of them: a
/// glyph is in the list by being declared, so the test that the fonts
/// have every glyph (`the_fonts_have_every_toolbar_glyph`) can't miss one.
/// (The list was kept by hand beside the constants, and ten were not in
/// it.)
macro_rules! glyphs {
    ($all:ident: $($(#[$doc:meta])* $name:ident = $glyph:literal;)*) => {
        $($(#[$doc])* pub(crate) const $name: &str = $glyph;)*
        /// Every one (the fonts must have them all).
        #[cfg(test)]
        pub(crate) const $all: &[&str] = &[$($name),*];
    };
}

glyphs! {
    ALL:
    TILES = "🗻";
    ERASER = "🗑";
    RAISE_LOWER = "↕";
    REFINE = "🔁";
    /// The tiles palette's Next and Previous Tile Appearance and Rotate
    /// Tile (a quarter turn counter-clockwise, as the variants' "↺90°").
    NEXT_APPEARANCE = "⏭";
    PREVIOUS_APPEARANCE = "⏮";
    ROTATE = "↺";
    START = "🏁";
    NIGHT = "🌙";
    FOG = "☁";
    /// The area view's Tile Border (what the game draws past the edge).
    TILE_BORDER = "🌄";
    /// The area view's Shadows switch.
    SHADOWS = "◑";
    GRID = "⊞";
    GRASS = "🌿";
    /// The game's own shaders (the area and model views' Shaders switch).
    SHADERS = "🎨";
    AMBIENT = "🍃";
    MUSIC = "🎵";
    WALKMESH = "👣";
    SELECT_TILES = "⛶";
    PAINT = "✏";
    /// The area view's Area Properties (the main toolbar's Module Properties
    /// is ℹ).
    AREA_PROPERTIES = "🔧";
    /// A palette's Gallery (its pictures; the area view's Grid is [`GRID`]).
    GALLERY = "🖼";
    /// The area view's Show All and Show None, Together (turning several as
    /// one), To Scratch and Save as Prefab…, and its dropdowns' glyphs with
    /// Icons only (Fade Geometry, Snap, Turn).
    SHOW_ALL = "☑";
    SHOW_NONE = "☐";
    TOGETHER = "🔗";
    TO_SCRATCH = "📤";
    /// The area view's Onto Objects (what is placed goes onto the placeable
    /// or door under the pointer).
    ONTO_OBJECTS = "📥";
    FADE = "🏠";
    SNAP = "📐";
    TURN = "↻";
    CAMERA = "🎥";
    GO_TO_START = "🏃";
    LIGHTING = "💡";
    ANIMATIONS = "▶";
    SOUNDS = "🔊";
    SOUND_RANGES = "◎";
    /// The area view's Minimap (the area's map in its corner).
    MINIMAP = "🗾";
    /// An area another builder has claimed, one this builder has, and one
    /// merged (its Tag and ResRef kept): the module tree's marks.
    CLAIMED = "🔒";
    CLAIMED_MINE = "✔";
    MERGED = "📌";
    /// Prefabs: groups of placed objects kept to place again.
    PREFABS = "🗐";
    /// The palette's lights (the game's invisible light placeables), a sun
    /// as the area view marks them.
    LIGHTS = "☀";
    /// A window's bar: Maximize, and Restore while it is maximized.
    MAXIMIZE = "🗖";
    RESTORE = "🗗";
}

/// An object type's glyph.
pub(crate) fn object(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Creature => "👤",
        ObjectKind::Door => "🚪",
        ObjectKind::Encounter => "⚔",
        ObjectKind::Item => "🗡",
        ObjectKind::Placeable => "⛲",
        ObjectKind::Sound => "🔉",
        ObjectKind::Store => "💰",
        ObjectKind::Trigger => "⚡",
        ObjectKind::Waypoint => "📍",
    }
}

/// `text` after `glyph`.
pub(crate) fn labelled(glyph: &str, text: &str) -> String {
    format!("{glyph} {text}")
}

/// The editors' toolbars' glyphs, with Icons only (beside the others,
/// their words alone): the script editor's, the conversation editor's,
/// the model view's and the NUI Creator's.
pub(crate) mod editor {
    glyphs! {
        ALL:
        SAVE = "💾";
        COMPILE = "⚙";
        SAVE_AS = "🖹";
        PRINT = "🖶";
        OPEN = "🗁";
        EXTERNAL = "🖊";
        FIND = "🔍";
        REPLACE = "🔄";
        BOOKMARK = "🔖";
        DEFINITION = "🎯";
        REFERENCES = "🔗";
        RENAME = "🏷";
        USED_BY = "⤴";
        ADD = "➕";
        COPY = "📑";
        CUT = "✂";
        PASTE = "📋";
        PASTE_LINK = "📎";
        DELETE = "🗑";
        EXPAND_ALL = "⏬";
        COLLAPSE_ALL = "⏫";
        COMMENTS = "💬";
        ACTIONS = "⚡";
        QUESTS = "📜";
        ANIMATIONS = "🎭";
        SOUNDS = "🔊";
        EXPORT = "📤";
        IMPORT = "📥";
        SCRIPTS = "📃";
        PLAY = "▶";
        PAUSE = "⏸";
        FRAME = "🔲";
        LIGHTS = "💡";
        POP_OUT = "🗗";
        BUILD = "🔨";
        HOW_TO_OPEN = "❓";
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

    /// A button's text: the glyph and the name, or the glyph alone.
    pub(crate) fn label(self, glyph: &str, name: &str) -> String {
        if self.icons_only { glyph.to_string() } else { labelled(glyph, name) }
    }

    /// An editor's toolbar button's text: its words (`name`), or with
    /// Icons only its glyph alone.
    pub(crate) fn word(self, glyph: &str, name: &str) -> String {
        if self.icons_only { glyph.to_string() } else { name.to_string() }
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

    /// A button: `glyph`, named `name`, with `tip`.
    pub(crate) fn button(
        self,
        ui: &mut egui::Ui,
        glyph: &str,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        if self.hides(name) {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
        }
        self.add(ui, egui::Button::new(self.label(glyph, name)), name, tip)
    }

    /// A button that stays down while `on`, and turns it on and off.
    pub(crate) fn toggle(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        glyph: &str,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        if self.hides(name) {
            return ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover());
        }
        let r = ui.toggle_value(on, self.label(glyph, name));
        self.named(r, name, tip, Some(*on))
    }

    /// An editor's toolbar toggle: its words, or its glyph ([`Tools::word`]).
    pub(crate) fn toggle_word(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        glyph: &str,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = ui.toggle_value(on, self.word(glyph, name));
        self.named(r, name, tip, Some(*on))
    }

    /// An editor's toolbar tick: its words, or its glyph.
    pub(crate) fn checkbox_word(
        self,
        ui: &mut egui::Ui,
        on: &mut bool,
        glyph: &str,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = ui.checkbox(on, self.word(glyph, name));
        if self.icons_only {
            let (enabled, ticked) = (r.enabled(), *on);
            r.widget_info(|| {
                egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, ticked, name)
            });
        }
        self.with_tip(r, name, tip)
    }

    /// `button` (labelled with [`Tools::label`]) added, with its tip.
    pub(crate) fn add(
        self,
        ui: &mut egui::Ui,
        button: impl egui::Widget,
        name: &str,
        tip: &str,
    ) -> egui::Response {
        let r = ui.add(button);
        self.named(r, name, tip, None)
    }

    /// `r`, a button (`selected`: one that stays down while on), with its
    /// tip, and with Icons only named `name` for a screen reader, not by
    /// its glyph.
    pub(crate) fn named(
        self,
        r: egui::Response,
        name: &str,
        tip: &str,
        selected: Option<bool>,
    ) -> egui::Response {
        if self.icons_only {
            let enabled = r.enabled();
            r.widget_info(|| match selected {
                Some(on) => egui::WidgetInfo::selected(egui::WidgetType::Button, enabled, on, name),
                None => egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, name),
            });
        }
        self.with_tip(r, name, tip)
    }

    /// `r` with its tip, shown while it is disabled too (a glyph alone,
    /// grayed, says least).
    fn with_tip(self, r: egui::Response, name: &str, tip: &str) -> egui::Response {
        match self.tip(name, tip) {
            Some(tip) => r.on_hover_text(&tip).on_disabled_hover_text(tip),
            None => r,
        }
    }

    /// Every glyph the toolbars, the palettes and the module tree show (the
    /// fonts must have them all): the main toolbar's, the object types',
    /// and every one declared here ([`ALL`], [`editor::ALL`]).
    #[cfg(test)]
    pub(crate) fn glyphs() -> Vec<&'static str> {
        let mut all: Vec<&str> = crate::commands::TOOLBAR.iter().flatten().map(|t| t.1).collect();
        all.extend(ObjectKind::ALL.map(object));
        all.extend(ALL);
        all.extend(editor::ALL);
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With Icons only, the glyph alone and the name in the tip: first,
    /// unless the tip begins with it, or alone; beside the name, the tip
    /// as it was.
    #[test]
    fn icons_only_moves_the_name_into_the_tip() {
        let both = Tools { icons_only: false, hidden: 0 };
        let icons = Tools { icons_only: true, hidden: 0 };
        assert_eq!(both.label("🎥", "Reorient Camera"), "🎥 Reorient Camera");
        assert_eq!(icons.label("🎥", "Reorient Camera"), "🎥");
        assert_eq!(both.tip("Reorient Camera", ""), None);
        assert_eq!(icons.tip("Reorient Camera", "").as_deref(), Some("Reorient Camera"));
        assert_eq!(both.tip("Grid", "Display Grid").as_deref(), Some("Display Grid"));
        assert_eq!(icons.tip("Grid", "Display Grid").as_deref(), Some("Grid\nDisplay Grid"));
        assert_eq!(icons.tip("Undo", "Undo (Ctrl+Z)").as_deref(), Some("Undo (Ctrl+Z)"));
        assert_eq!(icons.tip("Save", "Save module").as_deref(), Some("Save module"));
        // (A tip shorter than the name, or cut inside a character.)
        assert_eq!(icons.tip("Properties", "ℹ").as_deref(), Some("Properties\nℹ"));
        assert_eq!(icons.tip("ab", "ℹ").as_deref(), Some("ab\nℹ"));
    }

    /// A glyph is one command's across the main toolbar and the area
    /// view's (Module Properties ℹ, Area Properties 🔧; the area's Grid
    /// ⊞, a palette's Gallery 🖼): the one they share is Go to Start
    /// Location, the same command on both.
    #[test]
    fn a_glyph_is_one_command_s() {
        let main: Vec<&str> = crate::commands::TOOLBAR.iter().flatten().map(|t| t.1).collect();
        let area = [
            START,
            NIGHT,
            LIGHTING,
            SHADOWS,
            FOG,
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
            WALKMESH,
            SELECT_TILES,
            PAINT,
            CAMERA,
            GO_TO_START,
            AREA_PROPERTIES,
            SHOW_ALL,
            SHOW_NONE,
            TOGETHER,
            ONTO_OBJECTS,
            TO_SCRATCH,
            PREFABS,
            LIGHTS,
        ];
        let shared: Vec<&str> = area.into_iter().filter(|g| main.contains(g)).collect();
        assert_eq!(shared, [GO_TO_START]);
        let mut seen = area.to_vec();
        seen.extend(ObjectKind::ALL.map(object));
        seen.push(GALLERY);
        let n = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), n, "a glyph for two of the area's buttons");
        let mut editor = editor::ALL.to_vec();
        editor.sort_unstable();
        editor.dedup();
        assert_eq!(editor.len(), editor::ALL.len(), "a glyph for two of an editor's buttons");
    }

    /// The toolbars' glyphs are all in the app's fonts (none drawn as a
    /// square).
    #[test]
    fn the_fonts_have_every_toolbar_glyph() {
        let ctx = egui::Context::default();
        crate::widgets::install_fonts(&ctx);
        // (The fonts are there from the next frame.)
        for _ in 0..2 {
            ctx.run_ui(egui::RawInput::default(), |_| {}).textures_delta.clear();
        }
        let font = egui::TextStyle::Button.resolve(&ctx.global_style());
        // What each character of `text` is drawn with, in the font texture.
        // (Not `Fonts::has_glyphs`: it says no for every glyph of the font
        // that has the replacement square, egui's emoji font.)
        let drawn = |text: &str| {
            let galley = ctx.fonts_mut(|f| {
                f.layout_no_wrap(text.to_string(), font.clone(), egui::Color32::WHITE)
            });
            let rows = galley.rows.iter().flat_map(|r| r.glyphs.iter());
            rows.map(|g| g.uv_rect).collect::<Vec<_>>()
        };
        let square = drawn("◻")[0];
        let lacks = |text: &str| drawn(text).contains(&square);
        let glyphs = Tools::glyphs();
        assert!(glyphs.len() > 30);
        let missing: Vec<_> = glyphs.into_iter().filter(|g| lacks(g)).collect();
        assert!(missing.is_empty(), "no glyph for {missing:?}");
        assert!(lacks("\u{10ffff}") && !lacks("A"), "the check can fail");
    }
}
