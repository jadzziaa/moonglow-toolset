//! Help › User Manual: the manual (`docs/manual`), built into the program,
//! a chapter at a time beside the list of chapters; a link to another
//! chapter opens it in place. And Help › About.

use egui::Ui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

use crate::Moonglow;

/// The chapters: file name and text, the contents first.
pub(crate) const CHAPTERS: [(&str, &str); 18] = [
    ("README.md", include_str!("../../../docs/manual/README.md")),
    ("01-getting-started.md", include_str!("../../../docs/manual/01-getting-started.md")),
    ("02-main-window.md", include_str!("../../../docs/manual/02-main-window.md")),
    ("03-modules.md", include_str!("../../../docs/manual/03-modules.md")),
    ("04-areas.md", include_str!("../../../docs/manual/04-areas.md")),
    ("05-blueprints.md", include_str!("../../../docs/manual/05-blueprints.md")),
    ("06-conversations.md", include_str!("../../../docs/manual/06-conversations.md")),
    ("07-scripts.md", include_str!("../../../docs/manual/07-scripts.md")),
    ("08-journal-and-factions.md", include_str!("../../../docs/manual/08-journal-and-factions.md")),
    ("09-build-and-test.md", include_str!("../../../docs/manual/09-build-and-test.md")),
    ("10-options.md", include_str!("../../../docs/manual/10-options.md")),
    ("11-command-line.md", include_str!("../../../docs/manual/11-command-line.md")),
    ("12-differences.md", include_str!("../../../docs/manual/12-differences.md")),
    ("13-troubleshooting.md", include_str!("../../../docs/manual/13-troubleshooting.md")),
    ("14-coming-from-aurora.md", include_str!("../../../docs/manual/14-coming-from-aurora.md")),
    ("15-plugins.md", include_str!("../../../docs/manual/15-plugins.md")),
    ("16-writing-plugins.md", include_str!("../../../docs/manual/16-writing-plugins.md")),
    ("17-plugin-api.md", include_str!("../../../docs/manual/17-plugin-api.md")),
];

/// A chapter's title: its first heading.
pub(crate) fn title(text: &str) -> &str {
    text.lines().find_map(|l| l.strip_prefix("# ")).unwrap_or_default()
}

/// The manual tab's state.
#[derive(Debug, Default)]
pub(crate) struct Manual {
    pub(crate) chapter: usize,
    cache: CommonMarkCache,
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let m = &mut app.manual;
    let mut go = None;
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(190.0);
            for (i, (_, text)) in CHAPTERS.iter().enumerate() {
                let name = if i == 0 { "Contents" } else { title(text) };
                if ui.selectable_label(m.chapter == i, name).clicked() {
                    go = Some(i);
                }
            }
        });
        ui.separator();
        let (_, text) = CHAPTERS[m.chapter.min(CHAPTERS.len() - 1)];
        for (file, _) in CHAPTERS {
            m.cache.add_link_hook(file);
        }
        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt(("manual", m.chapter)).auto_shrink(false).show(
                ui,
                |ui| {
                    ui.set_max_width(ui.available_width().min(760.0));
                    CommonMarkViewer::new().show(ui, &mut m.cache, text);
                },
            );
        });
        go = go.or_else(|| {
            CHAPTERS.iter().position(|(file, _)| m.cache.get_link_hook(file) == Some(true))
        });
    });
    if let Some(i) = go {
        m.chapter = i;
    }
}

/// Help › About Moonglow Toolset, while open.
pub(crate) fn about_window(app: &mut Moonglow, ctx: &egui::Context) {
    if !app.about {
        return;
    }
    let mut open = true;
    egui::Window::new("About Moonglow Toolset")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Moonglow Toolset");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(6.0);
                ui.label("A module toolset for Neverwinter Nights: Enhanced Edition.");
            });
            ui.add_space(10.0);
            ui.label(
                "Moonglow is free software: you can redistribute it and/or modify it under \
                 the terms of the GNU General Public License, version 3. It comes with no \
                 warranty.",
            );
            ui.add_space(6.0);
            ui.label(
                "It includes Beamdog's NWScript compiler and is built with egui, wgpu, \
                 symphonia, rodio and other open-source libraries; their licenses are in \
                 THIRD-PARTY-LICENSES.txt, beside the program.",
            );
            ui.add_space(6.0);
            ui.label(
                "Moonglow is not affiliated with Beamdog or Wizards of the Coast, and \
                 contains no game data: it reads your installation of the game.",
            );
        });
    if !open {
        app.about = false;
    }
}

#[cfg(test)]
mod tests {
    use super::CHAPTERS;

    /// Every chapter of `docs/manual` is built in, and every link between
    /// chapters names one of them.
    #[test]
    fn links_between_chapters_resolve() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/manual");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|f| f.ends_with(".md"))
            .collect();
        files.sort();
        let mut built: Vec<String> = CHAPTERS.iter().map(|(f, _)| f.to_string()).collect();
        built.sort();
        assert_eq!(files, built, "chapters in docs/manual and in CHAPTERS");
        for (file, text) in CHAPTERS {
            for link in text.split("](").skip(1) {
                let target = &link[..link.find(')').unwrap()];
                if target.ends_with(".md") && !target.contains("://") {
                    assert!(
                        CHAPTERS.iter().any(|(f, _)| *f == target),
                        "{file} links to {target}, which isn't a chapter"
                    );
                }
            }
        }
    }
}
