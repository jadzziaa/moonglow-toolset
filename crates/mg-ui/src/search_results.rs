//! Search Results (M4's U13): the Bottom Dock's global panel, shown
//! whatever the editor in front, with its switch in the Status Bar (every
//! board draws it). It holds the last search's hits, what was searched
//! over them; a click on a hit opens it. The script editor's searches feed
//! it (Find In Files, Find In Currently Open Scripts, References); the
//! dialogs that become panels (U28: Find Instance, Find and Replace Text)
//! add their own kinds of [`Hit`]. Floating, a script's window shows it on
//! its bottom pane after the script's own panels (Floating.png).

use egui::Ui;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::script_tools::{self as tools, InfoTab};

/// The accessible name of Search Results' body (its hits or the words
/// saying what fills it): a screen reader's, and the tests' way to it
/// apart from the panel's tab and switch, which share its name.
pub const SEARCH_HITS: &str = "Search hits";

/// One hit of a search, as Search Results lists it and opens it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hit {
    /// A line of a script: the script, its line (from 0) and its text.
    Line { script: ResKey, line: usize, text: String },
}

impl Hit {
    /// Its row's words: `hello(3): int n = 1;` for a script's line.
    pub fn row(&self) -> String {
        match self {
            Hit::Line { script, line, text } => format!("{}({}): {text}", script.resref, line + 1),
        }
    }
}

/// What Search Results shows: the last search, said by `what` (`"Twice":
/// 3 lines found`), and its hits, at most [`tools::MAX_RESULTS`].
#[derive(Debug, Clone, Default)]
pub struct SearchResults {
    pub what: String,
    pub hits: Vec<Hit>,
}

impl Moonglow {
    /// A search's hits into Search Results, in place of the last one's,
    /// said by `what`; the panel brought to the front and the Bottom Dock
    /// opened on it where anything was found (or the window's bottom pane of
    /// the script `from`, where it floats).
    pub(crate) fn show_search_results(
        &mut self,
        what: String,
        hits: Vec<Hit>,
        from: Option<ResKey>,
    ) {
        // (Nothing found opens no closed dock: the log says so.)
        let open = !hits.is_empty();
        self.search_results = SearchResults { what, hits };
        self.script_tools.ask_forward(from, InfoTab::SearchResults, open);
    }
}

/// Search Results' body (`shell::panel_body`, in the Bottom Dock or in a
/// script's window's bottom pane): what was searched, then the hits by
/// the rows in sight (each a line high, not wrapped: Find In Files for a
/// common word keeps thousands); with nothing searched yet, what fills it.
/// A click on a hit opens it (a script at its line).
pub(crate) fn body(app: &mut Moonglow, ui: &mut Ui) {
    let keys = crate::palette_view::Keys::SearchResults;
    crate::script_panels::begin(app, ui, SEARCH_HITS, keys);
    let results = &app.search_results;
    if results.hits.is_empty() && results.what.is_empty() {
        ui.weak(
            "Find In Files, Find In Currently Open Scripts (a script's Find…) and References list \
             what they find here.",
        );
        return;
    }
    let mut open = None;
    ui.add(egui::Label::new(egui::RichText::new(&results.what).weak()).truncate());
    let area = egui::ScrollArea::vertical().id_salt("search-hits").auto_shrink([false, false]);
    mg_uikit::show_rows(area, ui, results.hits.len(), |ui, range| {
        for hit in &results.hits[range] {
            // (A long line cut at the panel's edge, on one row.)
            if ui.add(egui::Button::selectable(false, hit.row()).truncate()).clicked() {
                open = Some(hit.clone());
            }
        }
    });
    match open {
        Some(Hit::Line { script, line, .. }) => {
            crate::script_view::open_line(app, &script.resref.to_string(), line + 1);
        }
        None => {}
    }
}

/// The lines of the scripts `keys` that `find` matches (as the Find
/// window's options say), at most [`tools::MAX_RESULTS`] of them, and how
/// many there were.
pub(crate) fn lines_found(
    app: &Moonglow,
    keys: &[ResKey],
    find: &str,
    options: &tools::SearchOptions,
) -> (Vec<Hit>, usize) {
    let mut hits = Vec::new();
    let mut found = 0;
    let ws = app.ws.as_ref();
    for k in keys {
        let text = crate::script_view::script_source(&app.scripts, ws, None, k).unwrap_or_default();
        for (line, text) in tools::find_lines(&text, find, options) {
            found += 1;
            if hits.len() < tools::MAX_RESULTS {
                hits.push(Hit::Line { script: *k, line, text });
            }
        }
    }
    (hits, found)
}
