//! The glyphs that label the object types (in the palette and the area
//! view's toolbar) and the area view's toggles, all in egui's built-in
//! fonts.

use mg_area::ObjectKind;

pub(crate) const TILES: &str = "🗻";
pub(crate) const ERASER: &str = "🗑";
pub(crate) const RAISE_LOWER: &str = "↕";
pub(crate) const REFINE: &str = "🔁";
/// The tiles palette's Next and Previous Tile Appearance and Rotate
/// Tile (a quarter turn counter-clockwise, as the variants' "↺90°").
pub(crate) const NEXT_APPEARANCE: &str = "⏭";
pub(crate) const PREVIOUS_APPEARANCE: &str = "⏮";
pub(crate) const ROTATE: &str = "↺";
pub(crate) const START: &str = "🏁";
pub(crate) const NIGHT: &str = "🌙";
pub(crate) const FOG: &str = "☁";
/// The area view's Shadows switch.
pub(crate) const SHADOWS: &str = "◑";
pub(crate) const GRID: &str = "⊞";
pub(crate) const GRASS: &str = "🌿";
/// The game's own shaders (the area and model views' Shaders switch).
pub(crate) const SHADERS: &str = "🎨";
pub(crate) const AMBIENT: &str = "🍃";
pub(crate) const MUSIC: &str = "🎵";
pub(crate) const WALKMESH: &str = "👣";
pub(crate) const SELECT_TILES: &str = "⛶";
pub(crate) const PAINT: &str = "✏";
/// The area view's Area Properties (the main toolbar's Module Properties
/// is ℹ).
pub(crate) const AREA_PROPERTIES: &str = "🔧";
/// A palette's Gallery (its pictures; the area view's Grid is [`GRID`]).
pub(crate) const GALLERY: &str = "🖼";
/// The area view's Show All and Show None, Together (turning several as
/// one), To Scratch and Save as Prefab…, and its dropdowns' glyphs with
/// Icons only (Fade Geometry, Snap, Turn).
pub(crate) const SHOW_ALL: &str = "☑";
pub(crate) const SHOW_NONE: &str = "☐";
pub(crate) const TOGETHER: &str = "🔗";
pub(crate) const TO_SCRATCH: &str = "📤";
pub(crate) const FADE: &str = "🏠";
pub(crate) const SNAP: &str = "📐";
pub(crate) const TURN: &str = "↻";
pub(crate) const CAMERA: &str = "🎥";
pub(crate) const GO_TO_START: &str = "🏃";
pub(crate) const LIGHTING: &str = "💡";
pub(crate) const ANIMATIONS: &str = "▶";
pub(crate) const SOUNDS: &str = "🔊";
pub(crate) const SOUND_RANGES: &str = "◎";
/// An area another builder has claimed, one this builder has, and one
/// merged (its Tag and ResRef kept): the module tree's marks.
pub(crate) const CLAIMED: &str = "🔒";
pub(crate) const CLAIMED_MINE: &str = "✔";
pub(crate) const MERGED: &str = "📌";
/// Prefabs: groups of placed objects kept to place again.
pub(crate) const PREFABS: &str = "🗐";

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
    pub(crate) const SAVE: &str = "💾";
    pub(crate) const COMPILE: &str = "⚙";
    pub(crate) const SAVE_AS: &str = "🖹";
    pub(crate) const PRINT: &str = "🖶";
    pub(crate) const OPEN: &str = "🗁";
    pub(crate) const EXTERNAL: &str = "🖊";
    pub(crate) const FIND: &str = "🔍";
    pub(crate) const REPLACE: &str = "🔄";
    pub(crate) const BOOKMARK: &str = "🔖";
    pub(crate) const DEFINITION: &str = "🎯";
    pub(crate) const REFERENCES: &str = "🔗";
    pub(crate) const RENAME: &str = "🏷";
    pub(crate) const USED_BY: &str = "⤴";
    pub(crate) const ADD: &str = "➕";
    pub(crate) const COPY: &str = "📑";
    pub(crate) const CUT: &str = "✂";
    pub(crate) const PASTE: &str = "📋";
    pub(crate) const PASTE_LINK: &str = "📎";
    pub(crate) const DELETE: &str = "🗑";
    pub(crate) const EXPAND_ALL: &str = "⏬";
    pub(crate) const COLLAPSE_ALL: &str = "⏫";
    pub(crate) const COMMENTS: &str = "💬";
    pub(crate) const ACTIONS: &str = "⚡";
    pub(crate) const QUESTS: &str = "📜";
    pub(crate) const ANIMATIONS: &str = "🎭";
    pub(crate) const SOUNDS: &str = "🔊";
    pub(crate) const EXPORT: &str = "📤";
    pub(crate) const IMPORT: &str = "📥";
    pub(crate) const SCRIPTS: &str = "📃";
    pub(crate) const PLAY: &str = "▶";
    pub(crate) const PAUSE: &str = "⏸";
    pub(crate) const FRAME: &str = "🔲";
    pub(crate) const LIGHTS: &str = "💡";
    pub(crate) const POP_OUT: &str = "🗗";
    pub(crate) const BUILD: &str = "🔨";
    pub(crate) const HOW_TO_OPEN: &str = "❓";

    /// Every one (the fonts must have them all).
    #[cfg(test)]
    pub(crate) const ALL: [&str; 36] = [
        SAVE,
        COMPILE,
        SAVE_AS,
        PRINT,
        OPEN,
        EXTERNAL,
        FIND,
        REPLACE,
        BOOKMARK,
        DEFINITION,
        REFERENCES,
        RENAME,
        USED_BY,
        ADD,
        COPY,
        CUT,
        PASTE,
        PASTE_LINK,
        DELETE,
        EXPAND_ALL,
        COLLAPSE_ALL,
        COMMENTS,
        ACTIONS,
        QUESTS,
        ANIMATIONS,
        SOUNDS,
        EXPORT,
        IMPORT,
        SCRIPTS,
        PLAY,
        PAUSE,
        FRAME,
        LIGHTS,
        POP_OUT,
        BUILD,
        HOW_TO_OPEN,
    ];
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
}

impl Tools {
    pub(crate) fn of(settings: &crate::Settings) -> Tools {
        Tools { icons_only: settings.toolbar_icons_only }
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

    /// Every glyph the toolbars show (the fonts must have them all).
    #[cfg(test)]
    pub(crate) fn glyphs() -> Vec<&'static str> {
        let mut all: Vec<&str> = crate::commands::TOOLBAR.iter().flatten().map(|t| t.1).collect();
        all.extend(ObjectKind::ALL.map(object));
        all.extend([
            START,
            NIGHT,
            LIGHTING,
            SHADOWS,
            FOG,
            ANIMATIONS,
            GRASS,
            SHADERS,
            GRID,
            SOUNDS,
            SOUND_RANGES,
            AMBIENT,
            MUSIC,
            WALKMESH,
            SELECT_TILES,
            PAINT,
            CAMERA,
            GO_TO_START,
            AREA_PROPERTIES,
            GALLERY,
            SHOW_ALL,
            SHOW_NONE,
            TOGETHER,
            TO_SCRATCH,
            FADE,
            SNAP,
            TURN,
            PREFABS,
        ]);
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
        let (both, icons) = (Tools { icons_only: false }, Tools { icons_only: true });
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
            ANIMATIONS,
            GRASS,
            SHADERS,
            GRID,
            SOUNDS,
            SOUND_RANGES,
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
            TO_SCRATCH,
            PREFABS,
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
