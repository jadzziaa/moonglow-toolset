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

/// A chapter without its frontmatter. The manual is part of an Open
/// Knowledge Format bundle (`docs/index.md`): each chapter opens with a
/// YAML block between `---` lines, which is for tools and not for the
/// reader.
pub(crate) fn body(text: &str) -> &str {
    let mut end = 0;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        end += line.len();
        match (i, line.trim_end() == "---") {
            (0, false) => return text,
            (0, true) => {}
            (_, true) => return text[end..].trim_start(),
            _ => {}
        }
    }
    text
}

/// A chapter's title: its first heading.
pub(crate) fn title(text: &str) -> &str {
    body(text).lines().find_map(|l| l.strip_prefix("# ")).unwrap_or_default()
}

/// The manual tab's state.
#[derive(Debug, Default)]
pub(crate) struct Manual {
    pub(crate) chapter: usize,
    cache: CommonMarkCache,
    /// The chapter as shown ([`fitted`]), and the chapter and width it
    /// was made for.
    shown: Option<(usize, u32, String)>,
}

/// A table row's cells: split at `|`, but not at one escaped (`\|`) or
/// inside a code span.
fn cells(row: &str) -> Vec<String> {
    let row = row.trim();
    let row = row.strip_prefix('|').unwrap_or(row);
    let mut out = vec![String::new()];
    let (mut code, mut chars) = (false, row.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '`' => code = !code,
            '\\' if chars.peek() == Some(&'|') => {
                chars.next();
                out.last_mut().expect("one cell").push('|');
                continue;
            }
            '|' if !code => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        out.last_mut().expect("one cell").push(c);
    }
    // (What follows the last `|` is no cell.)
    if out.len() > 1 && out.last().is_some_and(|c| c.trim().is_empty()) {
        out.pop();
    }
    out.into_iter().map(|c| c.trim().to_string()).collect()
}

/// About how wide a cell's text is drawn, in points: code is wider than
/// prose, and of a link only its text shows.
fn drawn_width(cell: &str) -> f32 {
    let (mut width, mut code, mut chars) = (0.0, false, cell.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '`' => code = !code,
            '*' if !code => {}
            ']' if !code && chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            _ => width += if code { 8.0 } else { 6.0 },
        }
    }
    width
}

/// A chapter as the viewer can show it in `width` points. The viewer
/// draws a table's cell on one line, so a table wider than the page is
/// cut off at its edge, and the text after it too: such a table is shown
/// as a list (a row an item; with more than two columns, the further
/// cells under it, each after its column's heading).
fn fitted(text: &str, width: f32) -> String {
    let is_row = |l: &str| l.trim_start().starts_with('|');
    let is_rule = |l: &str| {
        is_row(l) && l.contains('-') && l.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
    };
    let lines: Vec<&str> = text.lines().collect();
    let (mut out, mut i, mut fenced) = (String::with_capacity(text.len()), 0, false);
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        }
        let table = !fenced && is_row(line) && lines.get(i + 1).is_some_and(|l| is_rule(l));
        if !table {
            out.push_str(line);
            out.push('\n');
            i += 1;
            continue;
        }
        let end = (i..lines.len()).find(|&j| !is_row(lines[j])).unwrap_or(lines.len());
        let head = cells(line);
        let rows: Vec<Vec<String>> = lines[i + 2..end].iter().map(|l| cells(l)).collect();
        let columns = head.len().max(rows.iter().map(Vec::len).max().unwrap_or(0));
        let widest = |c: usize| {
            rows.iter()
                .chain([&head])
                .filter_map(|r| r.get(c))
                .map(|cell| drawn_width(cell))
                .fold(0.0, f32::max)
        };
        // (Each column has its padding, and the table its frame.)
        let needs = (0..columns).map(widest).sum::<f32>() + 10.0 * columns as f32 + 16.0;
        if needs <= width {
            for l in &lines[i..end] {
                out.push_str(l);
                out.push('\n');
            }
        } else {
            if columns == 2 && head.iter().all(|h| !h.is_empty()) {
                out.push_str(&format!("**{}** — **{}**\n\n", head[0], head[1]));
            }
            for row in &rows {
                let Some(first) = row.first() else { continue };
                match (columns, row.get(1).filter(|c| !c.is_empty())) {
                    (2, Some(second)) => out.push_str(&format!("- {first} — {second}\n")),
                    _ => out.push_str(&format!("- {first}\n")),
                }
                if columns > 2 {
                    for (c, cell) in row.iter().enumerate().skip(1).filter(|(_, c)| !c.is_empty()) {
                        match head.get(c).filter(|h| !h.is_empty()) {
                            Some(h) => out.push_str(&format!("  - {h}: {cell}\n")),
                            None => out.push_str(&format!("  - {cell}\n")),
                        }
                    }
                }
            }
            out.push('\n');
        }
        i = end;
    }
    out
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
        let chapter = m.chapter.min(CHAPTERS.len() - 1);
        for (file, _) in CHAPTERS {
            m.cache.add_link_hook(file);
        }
        ui.vertical(|ui| {
            egui::ScrollArea::vertical().id_salt(("manual", m.chapter)).auto_shrink(false).show(
                ui,
                |ui| {
                    let width = ui.available_width().min(760.0);
                    ui.set_max_width(width);
                    // (Made again when the page is wider or narrower by a step.)
                    let step = (width / 20.0) as u32;
                    if !matches!(&m.shown, Some((c, s, _)) if (*c, *s) == (chapter, step)) {
                        let text = fitted(body(CHAPTERS[chapter].1), step as f32 * 20.0);
                        m.shown = Some((chapter, step, text));
                    }
                    let text = m.shown.as_ref().map_or("", |(_, _, t)| t.as_str());
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
                "It includes Beamdog's NWScript compiler and (for plugins) Luau, and is built \
                 with egui, wgpu, symphonia, rodio and other open-source libraries; their \
                 licenses are in THIRD-PARTY-LICENSES.txt, beside the program.",
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
    use super::{CHAPTERS, body, cells, fitted, title};

    /// A chapter is shown from its heading, not from its frontmatter.
    #[test]
    fn a_chapter_is_shown_without_its_frontmatter() {
        assert_eq!(body("---\ntype: Manual Page\n---\n\n# Keys\n"), "# Keys\n");
        assert_eq!(body("---\r\ntitle: a\r\n---\r\n# Keys\r\n"), "# Keys\r\n");
        assert_eq!(body("# Keys\n\n---\n\nMore.\n"), "# Keys\n\n---\n\nMore.\n");
        assert_eq!(body("---\nnever closed\n"), "---\nnever closed\n");
        for (file, text) in CHAPTERS {
            assert!(text.starts_with("---\n"), "{file} has frontmatter");
            assert!(body(text).starts_with("# "), "{file} is shown from its heading");
            assert!(!title(text).is_empty(), "{file}");
        }
    }

    /// A table the page has room for stays one; a wider one is a list,
    /// and what is in a code block is left alone.
    #[test]
    fn a_table_too_wide_for_the_page_is_shown_as_a_list() {
        let narrow = "| Keys | Command |\n| --- | --- |\n| Ctrl+N | New module |\n";
        assert_eq!(fitted(narrow, 600.0), narrow);
        assert_eq!(fitted(narrow, 100.0), "**Keys** — **Command**\n\n- Ctrl+N — New module\n\n");
        let wide =
            "Before.\n\n| | |\n| --- | --- |\n| `a\\|b` | one `x | y` two |\n| c | |\n\nAfter.\n";
        assert_eq!(fitted(wide, 60.0), "Before.\n\n- `a|b` — one `x | y` two\n- c\n\n\nAfter.\n");
        let three = "| What | Linux | macOS |\n| --- | --- | --- |\n| Settings | `~/a` | `~/b` |\n";
        assert_eq!(fitted(three, 60.0), "- Settings\n  - Linux: `~/a`\n  - macOS: `~/b`\n\n");
        let code = "```text\n| not | a table |\n| --- | --- |\n| x | y |\n```\n";
        assert_eq!(fitted(code, 10.0), code);
        assert_eq!(cells("| a | b |"), ["a", "b"]);
        assert_eq!(cells("| a | |"), ["a", ""]);
    }

    /// No chapter is left with a table wider than the page.
    #[test]
    fn every_chapter_fits_the_page() {
        for (file, text) in CHAPTERS {
            let shown = fitted(body(text), 680.0);
            for line in shown.lines().filter(|l| l.starts_with('|')) {
                let needs: f32 = cells(line).iter().map(|c| super::drawn_width(c) + 10.0).sum();
                assert!(needs <= 680.0, "{file}: {line}");
            }
        }
    }

    /// Every chapter of `docs/manual` is built in, and every link between
    /// chapters names one of them.
    #[test]
    fn links_between_chapters_resolve() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/manual");
        let mut files: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            // (`index.md` and `log.md` are the bundle's listing and history, not chapters.)
            .filter(|f| f.ends_with(".md") && f != "index.md" && f != "log.md")
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
