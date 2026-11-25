//! Openers for the dialogs of `wizards.rs`, `creature_wizard.rs` and the
//! pickers (`blueprint/replacements.rs`, `blueprint/situated.rs`).

use super::Opener;
use crate::common::*;

/// An empty window with no module open.
fn plain() -> Option<Harness<'static, Moonglow>> {
    Some(harness(app_with(Vec::new()), 1280.0, 720.0))
}

/// A module of the game's open.
fn game() -> Option<Harness<'static, Moonglow>> {
    game_harness(&super::scratch("u27-pattern"))
}

/// A module with an area open.
fn area() -> Option<Harness<'static, Moonglow>> {
    area_ui_harness(&super::scratch("u27-pattern-area")).map(|(h, _)| h)
}

fn creature() -> Option<Harness<'static, Moonglow>> {
    blueprint_harness("nw_bandit001", "u27_portrait", ResType::UTC).map(|(h, _)| h)
}

fn chest() -> Option<Harness<'static, Moonglow>> {
    let (mut h, key) = blueprint_harness("plc_chest1", "u27_texture", ResType::UTP)?;
    h.state_mut().blueprint_pages.insert((key, mg_edit::GffPath::root()), "Visuals");
    Some(h)
}

/// Empties the `nth` one-line field of the dialog `title`.
pub(crate) fn clear_field(h: &mut Harness<'static, Moonglow>, title: &str, nth: usize) {
    let fields = super::fields_in(h, title);
    fields.get(nth).unwrap_or_else(|| panic!("{title}: its field {nth}")).focus();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.key_press(egui::Key::Backspace);
    h.run();
}

/// Clicks the first control named `name`, scrolled into sight first (a
/// window at the least size shows a part of its page).
pub(crate) fn click_reached(h: &mut Harness<'static, Moonglow>, name: &str) {
    h.get_all_by_label(name).next().unwrap_or_else(|| panic!("{name}")).scroll_to_me();
    settle(h);
    h.get_all_by_label(name).next().unwrap_or_else(|| panic!("{name}")).click();
}

/// The open Area Wizard on its page `page` (counted from 0).
fn area_wizard_page(h: &mut Harness<'static, Moonglow>, page: usize) {
    if let Some(mg_ui::Wizard::NewArea(w)) = &mut h.state_mut().wizard {
        w.page = page;
    }
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new("wizards.rs:ui#1", "New Module", plain, |h| {
            h.state_mut().actions.push(mg_ui::Action::NewModuleDialog)
        })
        .field(0)
        .off(|h| clear_field(h, "New Module", 0)),
        Opener::new("wizards.rs:ui#2", "Area Wizard", area, |h| {
            h.state_mut().actions.push(mg_ui::Action::AreaWizard)
        })
        .field(0)
        .off(|h| clear_field(h, "Area Wizard", 0))
        // (Its Size and Finish pages.)
        .states(vec![
            ("on step 2", |h| area_wizard_page(h, 1)),
            ("on step 3", |h| area_wizard_page(h, 2)),
        ])
        .enter_does("goes to its next page", |h| {
            super::is_shown(h, "Area Wizard") && h.query_by_label_contains("Step 2 of").is_some()
        }),
        Opener::new("wizards.rs:ui#3", "Close Moonglow?", plain, |h| h.state_mut().closing = true),
        Opener::new("wizards.rs:ui#4", "Unsaved Changes", plain, |h| {
            h.state_mut().confirm_discard = Some(mg_ui::Action::NewModuleDialog)
        }),
        Opener::new("creature_wizard.rs:window#1", "Creature Wizard", game, |h| {
            h.state_mut().creature_wizard = Some(Default::default())
        })
        // (Steps 2 and 4, the review's finds.)
        .states(vec![
            ("on step 2", |h| h.state_mut().creature_wizard.as_mut().unwrap().page = 1),
            ("on step 4", |h| h.state_mut().creature_wizard.as_mut().unwrap().page = 3),
        ])
        .enter_does("goes to its next page", |h| {
            super::is_shown(h, "Creature Wizard")
                && h.query_by_label_contains("Step 2 of").is_some()
        }),
        Opener::new("blueprint/situated.rs:portrait#1", "Select Portrait", creature, |h| {
            click_reached(h, "Portraits…")
        }),
        Opener::new("blueprint/replacements.rs:gallery#1", "Select Texture", chest, |h| {
            click_reached(h, "Select Texture…")
        }),
    ]
}
