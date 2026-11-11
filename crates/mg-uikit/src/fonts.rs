//! The suite's fonts: egui's own (Ubuntu Light for text, Hack for
//! monospace) and Ubuntu Bold as a family of its own, [`BOLD`], for
//! section headings (the only bold there is). The font and its license
//! (`fonts/UFL.txt`) are in this crate.

/// The bold font family's name (Ubuntu Bold, the bold of egui's own
/// Ubuntu): `egui::FontFamily::Name(BOLD.into())`, or [`bold`].
pub const BOLD: &str = "bold";

/// The bold font family, for section headings.
pub fn bold() -> egui::FontFamily {
    egui::FontFamily::Name(BOLD.into())
}

/// egui's default fonts and the bold family: Ubuntu Bold, then egui's
/// emoji fonts for the glyphs it lacks. (Start from these to install
/// fonts of one's own: a family a text style names must stay bound.)
pub fn definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Ubuntu-Bold".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../fonts/Ubuntu-Bold.ttf"
        ))),
    );
    let fallbacks = fonts.families.get(&egui::FontFamily::Proportional).cloned();
    let mut bold = vec!["Ubuntu-Bold".to_string()];
    bold.extend(fallbacks.unwrap_or_default().into_iter().skip(1));
    fonts.families.insert(egui::FontFamily::Name(BOLD.into()), bold);
    fonts
}

/// Adds the bold font family to `ctx`'s fonts (once a context; it is there
/// from the next frame): Ubuntu Bold, then egui's emoji fonts for the
/// glyphs it lacks.
pub fn install_fonts(ctx: &egui::Context) {
    let done = egui::Id::new("moonglow-fonts");
    if ctx.data(|d| d.get_temp::<bool>(done)).is_some() {
        return;
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
    ctx.set_fonts(definitions());
}

/// Whether `ctx`'s fonts have the bold family yet ([`install_fonts`]
/// takes effect a frame later). False before the context's first frame
/// has ended, when egui has no fonts to ask.
pub fn has_bold(ctx: &egui::Context) -> bool {
    ctx.cumulative_pass_nr() > 0 && ctx.fonts(|f| f.families().contains(&bold()))
}
