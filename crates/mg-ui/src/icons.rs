//! The glyphs that label the object types (in the palette and the area
//! view's toolbar) and the area view's toggles, all in egui's built-in
//! fonts.

use mg_area::ObjectKind;
use mg_module::palette::BlueprintKind;

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
pub(crate) const GRID: &str = "⊞";
pub(crate) const GRASS: &str = "🌿";
/// The game's own shaders (the area and model views' Shaders switch).
pub(crate) const SHADERS: &str = "🎨";
pub(crate) const AMBIENT: &str = "🍃";
pub(crate) const MUSIC: &str = "🎵";
pub(crate) const WALKMESH: &str = "👣";
pub(crate) const SELECT_TILES: &str = "⛶";
pub(crate) const PAINT: &str = "✏";
pub(crate) const PROPERTIES: &str = "ℹ";
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

/// A blueprint type's glyph: its objects'.
pub(crate) fn blueprint(kind: BlueprintKind) -> &'static str {
    ObjectKind::from_restype(kind.restype()).map_or("", object)
}

/// `text` after `glyph`.
pub(crate) fn labelled(glyph: &str, text: &str) -> String {
    format!("{glyph} {text}")
}

/// How a toolbar's buttons are labelled (Options › General › Toolbar
/// buttons): a glyph and its name, or, with Icons only, the glyph alone
/// and its name in its tip, as Aurora's toolbars have them. The main
/// toolbar and the area view's take it from the settings each frame.
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
            PROPERTIES,
            CAMERA,
            GO_TO_START,
        ]);
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
