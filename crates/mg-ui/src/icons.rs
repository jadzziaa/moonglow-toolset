//! The glyphs that label the object types (in the palette and the area
//! view's toolbar) and the area view's toggles, all in egui's built-in
//! fonts.

use mg_area::ObjectKind;
use mg_module::palette::BlueprintKind;

pub(crate) const TILES: &str = "🗻";
pub(crate) const ERASER: &str = "🗑";
pub(crate) const RAISE_LOWER: &str = "↕";
pub(crate) const START: &str = "🏁";
pub(crate) const NIGHT: &str = "🌙";
pub(crate) const FOG: &str = "☁";
pub(crate) const GRID: &str = "⊞";
pub(crate) const AMBIENT: &str = "🍃";
pub(crate) const MUSIC: &str = "🎵";
pub(crate) const WALKMESH: &str = "👣";
pub(crate) const SELECT_TILES: &str = "⛶";
pub(crate) const PROPERTIES: &str = "ℹ";
pub(crate) const CAMERA: &str = "🎥";
pub(crate) const GO_TO_START: &str = "🏃";

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
