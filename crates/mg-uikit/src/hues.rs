//! Colored icons: each object kind's icon in a hue of its own, where the
//! builder asks for it (Options › General › Colored icons, off by
//! default; 5.2.0, a builder's ask, the owner's ruling).
//!
//! The redesign draws every icon in one color (rule 7, "chrome recedes":
//! `text-2` at rest), and so it stays unless the option is on. On, a
//! kind's icon (a creature's, a door's, an area's, a script's) takes its
//! hue from [`kind_hue`] where a view asks for it: the Module panel's
//! groups and an area's kinds, the Outliner's kinds and resource types,
//! and the Palette's row of types ([`crate::TreeRow::kind_icon`],
//! [`crate::TreeGroup::kind_icon`], [`crate::IconButton::kind_hue`],
//! [`kind_icon`]); and the rows that paint their icon themselves, the
//! Resources panel's, Quick Open's and the hak editor's ([`tint`]).
//! Everything else keeps `text-2`.
//!
//! The hues follow the area view where it has a color for a kind
//! (encounters orange, triggers green, doors and waypoints blue, sounds
//! yellow, the markers' red), with the kinds that share a color there
//! moved apart around the wheel: waypoints cyan beside the doors' blue,
//! stores lime beside the triggers' green. Each is at least 3:1 (as a
//! graphic) on every surface a row or a button rests on in its theme
//! (`panel`, `hover-row`, `inset`, `bar`, `raised`, `raised-hover`).
//! A selected row or a chosen type keeps `on-fill` on `accent-fill`, as
//! its words do: no hue is 3:1 on both the light theme's panel and its
//! selected row (on both, only a near-black is), and the selection is
//! what that row says first. A dimmed row
//! (a kind hidden in the area, an empty kind) keeps `text-3`, so it still
//! reads as dimmed.

use egui::Color32;

use crate::icons::Icon;
use crate::theme::Theme;

/// An opaque color from its hex value.
const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// The kinds with a hue: their icon, and the hue in the dark and the light
/// theme.
pub const KIND_HUES: [(Icon, Color32, Color32); 12] = [
    (Icon::Creature, hex(0xf2807a), hex(0xb0302a)),
    (Icon::Encounter, hex(0xf0964a), hex(0x9a4800)),
    (Icon::Area, hex(0xd4b27c), hex(0x755218)),
    (Icon::Sound, hex(0xe2ca52), hex(0x6e5a00)),
    (Icon::Store, hex(0xa8d45a), hex(0x476800)),
    (Icon::Trigger, hex(0x5ccc84), hex(0x17703c)),
    (Icon::Waypoint, hex(0x4fc8de), hex(0x006b7e)),
    (Icon::Door, hex(0x72a8ff), hex(0x1f58b4)),
    (Icon::Script, hex(0xa0a2ff), hex(0x4044b8)),
    (Icon::Placeable, hex(0xc290f2), hex(0x6e36aa)),
    (Icon::Item, hex(0xea82d2), hex(0x962c7e)),
    (Icon::Conversation, hex(0xf590b0), hex(0xa42a58)),
];

/// The hue of the kind `icon` stands for in `theme`; `None` for an icon
/// that names no kind with a hue (a folder, a file, a journal, a tool),
/// which stays `text-2`.
pub const fn kind_hue(icon: Icon, theme: Theme) -> Option<Color32> {
    let mut i = 0;
    while i < KIND_HUES.len() {
        let (kind, dark, light) = KIND_HUES[i];
        if kind as usize == icon as usize {
            return Some(match theme {
                Theme::Dark => dark,
                Theme::Light => light,
            });
        }
        i += 1;
    }
    None
}

fn colored_id() -> egui::Id {
    egui::Id::new("mg-uikit-colored-icons")
}

/// Turns colored icons on or off for every view of `ctx`, from this
/// frame on. Cheap to call every frame: it writes only when it changes.
pub fn colored_icons(ctx: &egui::Context, on: bool) {
    if are_colored(ctx) != on {
        ctx.data_mut(|d| d.insert_temp(colored_id(), on));
        ctx.request_repaint();
    }
}

/// Whether `ctx`'s kind icons are colored ([`colored_icons`]; off until
/// turned on).
pub fn are_colored(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(colored_id())).unwrap_or(false)
}

/// The color a kind's `icon` is drawn in where it would be `rest` (its
/// `text-2`): its hue in the applied theme while icons are colored, else
/// `rest`. `rest` too where no theme was applied.
pub fn tint(ctx: &egui::Context, icon: Icon, rest: Color32) -> Color32 {
    hue(ctx, icon).unwrap_or(rest)
}

/// The hue a kind's `icon` is drawn in now: its kind's in the applied
/// theme while icons are colored; `None` while they are not, where no
/// theme was applied, or for an icon without a hue.
pub fn hue(ctx: &egui::Context, icon: Icon) -> Option<Color32> {
    if !are_colored(ctx) {
        return None;
    }
    crate::theme::applied(ctx).and_then(|t| kind_hue(icon, t))
}

/// A kind's icon (16 px, not interactive) in its hue while icons are
/// colored, else as [`crate::icon`] draws it (the text color).
pub fn kind_icon(ui: &mut egui::Ui, icon: Icon) -> egui::Response {
    let size = egui::Vec2::splat(crate::metrics::ICON);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        let color = tint(ui.ctx(), icon, ui.visuals().text_color());
        icon.paint(ui.painter(), rect, color);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{Tokens, contrast};

    /// The surfaces a kind's icon rests on in `t`: a row's (the panel, a
    /// well, a bar, hovered) and an icon button's (hovered and pressed).
    fn surfaces(t: &Tokens) -> [(&'static str, Color32); 6] {
        [
            ("panel", t.panel),
            ("hover-row", t.hover_row),
            ("inset", t.inset),
            ("bar", t.bar),
            ("raised", t.raised),
            ("raised-hover", t.raised_hover),
        ]
    }

    /// Every hue is 3:1 on every surface it rests on in its theme; the
    /// kinds' hues differ; in the light theme none is 3:1 on the selected
    /// row (why a selected row keeps `on-fill`, 4.5:1 there in both
    /// themes).
    #[test]
    fn hues_are_3_to_1_on_their_surfaces_and_apart() {
        for theme in Theme::ALL {
            let t = theme.tokens();
            for (icon, ..) in KIND_HUES {
                let hue = kind_hue(icon, theme).expect("a hue");
                for (name, under) in surfaces(&t) {
                    let r = contrast(hue, under);
                    assert!(r >= 3.0, "{theme:?}: {icon:?} on {name} is {r:.2}:1");
                }
                assert_ne!(hue, t.text_2, "{icon:?}");
            }
            for (i, a) in KIND_HUES.iter().enumerate() {
                for b in &KIND_HUES[i + 1..] {
                    assert_ne!(kind_hue(a.0, theme), kind_hue(b.0, theme), "{a:?} {b:?}");
                }
            }
            assert!(contrast(t.on_fill, t.accent_fill) >= 4.5, "{theme:?}");
        }
        // (In the light theme no hue is 3:1 on its selected row: only a
        // near-black would be, on both it and the panel.)
        for (icon, _, light) in KIND_HUES {
            let r = contrast(light, Tokens::LIGHT.accent_fill);
            assert!(r < 3.0, "{icon:?} on the light selected row is {r:.2}:1");
        }
    }

    /// Kinds without an identity keep `text-2`.
    #[test]
    fn other_icons_have_no_hue() {
        for icon in [Icon::File, Icon::Folder, Icon::Journal, Icon::Module, Icon::Filter] {
            assert_eq!(kind_hue(icon, Theme::Dark), None, "{icon:?}");
        }
    }

    /// The colors of the strokes and dots a frame of `add` painted, in
    /// `theme`, with colored icons `on`; and the frame's shapes as text.
    fn painted(theme: Theme, on: bool, add: impl Fn(&mut egui::Ui)) -> (Vec<Color32>, String) {
        let ctx = egui::Context::default();
        let mut out = (Vec::new(), String::new());
        // (The theme lands in the frame after it is applied.)
        for _ in 0..2 {
            let full = ctx.run_ui(egui::RawInput::default(), |ui| {
                crate::theme::apply(ui.ctx(), theme);
                colored_icons(ui.ctx(), on);
                add(ui);
            });
            let mut full = full;
            full.textures_delta.clear();
            let mut colors = Vec::new();
            fn walk(s: &egui::Shape, colors: &mut Vec<Color32>) {
                match s {
                    egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, colors)),
                    egui::Shape::Path(p) => {
                        if let egui::epaint::ColorMode::Solid(c) = p.stroke.color {
                            colors.push(c);
                        }
                    }
                    egui::Shape::Circle(c) => colors.push(c.fill),
                    _ => {}
                }
            }
            full.shapes.iter().for_each(|c| walk(&c.shape, &mut colors));
            out = (colors, format!("{:?}", full.shapes));
        }
        out
    }

    /// The kinds' rows of a tree and the Palette's type buttons: in the
    /// kind's hue while icons are colored, in both themes; `text-3` dimmed
    /// and `on-fill` marked or chosen, as before; and off, painted exactly
    /// as an icon that is no kind's.
    #[test]
    fn kind_icons_take_their_hue_only_while_colored() {
        use crate::{IconButton, TreeGroup, TreeRow};
        for theme in Theme::ALL {
            let t = theme.tokens();
            for (icon, ..) in KIND_HUES {
                let hue = kind_hue(icon, theme).unwrap();
                let row = |ui: &mut egui::Ui| _ = TreeRow::new("Kind").kind_icon(icon).show(ui);
                let (on, _) = painted(theme, true, row);
                assert!(on.contains(&hue), "{theme:?} {icon:?}: {on:?}");
                assert!(!on.contains(&t.text_2), "{theme:?} {icon:?}: the icon is the hue");
                let (off, kind) = painted(theme, false, row);
                assert!(off.contains(&t.text_2) && !off.contains(&hue), "{icon:?}: {off:?}");
                let plain = |ui: &mut egui::Ui| _ = TreeRow::new("Kind").icon(icon).show(ui);
                assert_eq!(kind, painted(theme, false, plain).1, "off as today");
                let group = |ui: &mut egui::Ui| {
                    _ = TreeGroup::new("g", "Kind").kind_icon(icon).show(ui, |_| ());
                };
                assert!(painted(theme, true, group).0.contains(&hue), "{icon:?}'s group");
                let button = |ui: &mut egui::Ui| {
                    ui.add(IconButton::new(icon, "Kind").icon_only(true).kind_hue());
                };
                let (on, _) = painted(theme, true, button);
                assert!(on.contains(&hue), "{theme:?} {icon:?}'s button: {on:?}");
                let (off, kind) = painted(theme, false, button);
                assert!(off.contains(&t.text_2) && !off.contains(&hue), "{off:?}");
                let plain = |ui: &mut egui::Ui| {
                    ui.add(IconButton::new(icon, "Kind").icon_only(true));
                };
                assert_eq!(kind, painted(theme, false, plain).1, "the button off as today");
                let chosen = |ui: &mut egui::Ui| {
                    ui.add(
                        IconButton::new(icon, "Kind").icon_only(true).kind_hue().selectable(true),
                    );
                };
                let (on, _) = painted(theme, true, chosen);
                assert!(on.contains(&t.on_fill) && !on.contains(&hue), "chosen: {on:?}");
            }
            let hue = kind_hue(Icon::Door, theme).unwrap();
            let dim =
                |ui: &mut egui::Ui| _ = TreeRow::new("Doors").kind_icon(Icon::Door).dim().show(ui);
            let (on, _) = painted(theme, true, dim);
            assert!(on.contains(&t.text_3) && !on.contains(&hue), "dimmed: {on:?}");
            let marked = |ui: &mut egui::Ui| {
                _ = TreeRow::new("Doors").kind_icon(Icon::Door).selected(true).show(ui);
            };
            let (on, _) = painted(theme, true, marked);
            assert!(on.contains(&t.on_fill) && !on.contains(&hue), "marked: {on:?}");
            let file = |ui: &mut egui::Ui| _ = TreeRow::new("File").kind_icon(Icon::File).show(ui);
            let (on, _) = painted(theme, true, file);
            assert!(on.contains(&t.text_2), "no kind: {on:?}");
            // The narrow Palette's icon beside its dropdown: the hue on;
            // off, as `crate::icon` draws it.
            let alone = |ui: &mut egui::Ui| _ = kind_icon(ui, Icon::Door);
            let (on, _) = painted(theme, true, alone);
            assert!(!on.is_empty() && on.iter().all(|c| *c == hue), "alone: {on:?}");
            let plain = |ui: &mut egui::Ui| _ = crate::icon(ui, Icon::Door);
            assert_eq!(painted(theme, false, alone).1, painted(theme, false, plain).1, "alone off");
        }
    }

    /// Off unless turned on; on, the applied theme's hue; off again, the
    /// color at rest; no theme applied, the color at rest.
    #[test]
    fn tint_follows_the_switch_and_the_theme() {
        let ctx = egui::Context::default();
        let rest = Color32::from_rgb(1, 2, 3);
        assert!(!are_colored(&ctx));
        colored_icons(&ctx, true);
        assert_eq!(tint(&ctx, Icon::Door, rest), rest, "no theme applied");
        for theme in Theme::ALL {
            crate::theme::apply(&ctx, theme);
            colored_icons(&ctx, true);
            assert!(are_colored(&ctx));
            assert_eq!(tint(&ctx, Icon::Door, rest), kind_hue(Icon::Door, theme).unwrap());
            assert_eq!(tint(&ctx, Icon::File, rest), rest);
            colored_icons(&ctx, false);
            assert_eq!(tint(&ctx, Icon::Door, rest), rest);
        }
    }
}
