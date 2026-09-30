//! The script editor: syntax-highlighted text, save, and compile.

use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, Ui};
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_resman::ResKey;
use mg_script::Compiler;
use mg_script::lex::{TokenKind, tokenize};

use crate::text::{decode, encode};
use crate::{Action, Moonglow};

/// A script's text in its editor.
#[derive(Debug, Clone)]
pub(crate) struct ScriptBuffer {
    pub(crate) text: String,
    /// The text as last loaded or saved.
    pub(crate) saved: String,
    /// The module's bytes the text was last compared with (address and
    /// length), to decode them again only when they change.
    source: (usize, usize),
}

/// The laid-out text of a script editor, reused while the text, width and
/// theme stay the same (a large script takes milliseconds to highlight and
/// lay out).
#[derive(Debug, Clone)]
pub(crate) struct LaidOut {
    text: String,
    wrap: f32,
    dark: bool,
    galley: std::sync::Arc<egui::Galley>,
}

impl ScriptBuffer {
    pub(crate) fn is_dirty(&self) -> bool {
        self.text != self.saved
    }
}

fn color(kind: TokenKind, dark: bool) -> Color32 {
    let (kw, com, lit, dir, konst, unknown) = if dark {
        (
            Color32::from_rgb(86, 156, 214),
            Color32::from_rgb(106, 153, 85),
            Color32::from_rgb(206, 145, 120),
            Color32::from_rgb(197, 134, 192),
            Color32::from_rgb(79, 193, 255),
            Color32::RED,
        )
    } else {
        (
            Color32::from_rgb(0, 0, 200),
            Color32::from_rgb(0, 128, 0),
            Color32::from_rgb(163, 21, 21),
            Color32::from_rgb(128, 0, 128),
            Color32::from_rgb(0, 112, 193),
            Color32::RED,
        )
    };
    match kind {
        TokenKind::Keyword => kw,
        TokenKind::LineComment | TokenKind::BlockComment { .. } => com,
        TokenKind::String { .. }
        | TokenKind::RawString { .. }
        | TokenKind::HashedString { .. }
        | TokenKind::Int
        | TokenKind::Float => lit,
        TokenKind::Directive => dir,
        TokenKind::BuiltinConstant => konst,
        TokenKind::Unknown => unknown,
        _ => Color32::PLACEHOLDER,
    }
}

/// Lays the text out with token colours.
pub(crate) fn highlight(text: &str, dark: bool, default: Color32) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font = FontId::monospace(13.0);
    for t in tokenize(text.as_bytes()) {
        let mut c = color(t.kind, dark);
        if c == Color32::PLACEHOLDER {
            c = default;
        }
        // Tokens end on character boundaries except inside malformed UTF-8,
        // which the text (a String) cannot contain.
        job.append(&text[t.span], 0.0, TextFormat::simple(font.clone(), c));
    }
    job
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(ws) = &app.ws else { return };
    let Some(bytes) = ws.module.get(&key) else {
        ui.label(format!("{key} is no longer in the module."));
        return;
    };
    let source = (bytes.as_ptr() as usize, bytes.len());
    let Moonglow { scripts, laid_out, actions, .. } = app;
    let buf = scripts.entry(key).or_insert_with(|| {
        let text = decode(bytes);
        ScriptBuffer { text: text.clone(), saved: text, source }
    });
    if buf.source != source {
        buf.source = source;
        let module_text = decode(bytes);
        if !buf.is_dirty() && buf.saved != module_text {
            // Changed underneath (undo, import): follow the module.
            buf.text = module_text.clone();
            buf.saved = module_text;
        }
    }
    let mut save = false;
    let mut compile = false;
    ui.horizontal(|ui| {
        save = ui.add_enabled(buf.is_dirty(), egui::Button::new("Save")).clicked();
        compile = ui.button("Compile").clicked();
        if buf.is_dirty() {
            ui.weak("modified");
        }
    });
    ui.separator();
    let dark = ui.visuals().dark_mode;
    let default = ui.visuals().text_color();
    let cached = laid_out.entry(key).or_insert(None);
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        if let Some(c) =
            cached.as_ref().filter(|c| c.wrap == wrap && c.dark == dark && c.text == text.as_str())
        {
            return c.galley.clone();
        }
        let mut job = highlight(text.as_str(), dark, default);
        job.wrap.max_width = wrap;
        let galley = ui.fonts_mut(|f| f.layout_job(job));
        *cached =
            Some(LaidOut { text: text.as_str().to_string(), wrap, dark, galley: galley.clone() });
        galley
    };
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut buf.text)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(30)
                .id(egui::Id::new(("script", key)))
                .layouter(&mut layouter),
        );
    });
    if save || compile {
        let text = buf.text.clone();
        if buf.is_dirty() {
            buf.saved = text.clone();
            actions.push(Action::Apply(Command::new(
                format!("Edit {key}"),
                vec![Edit::SetResource { key, data: Some(encode(&text)) }],
            )));
        }
        if compile {
            compile_one(app, key, &text);
        }
    }
}

/// Compiles one script (its text as in the editor) and stores the bytecode.
fn compile_one(app: &mut Moonglow, key: ResKey, text: &str) {
    let Some(ws) = &app.ws else { return };
    let source = encode(text);
    let module = &ws.module;
    let resman = app.game.as_ref().map(|g| &g.resman);
    let name = key.resref.to_lowercase().to_string();
    let result = {
        let mut c = Compiler::new(|n: &str, t: ResType| {
            if t == ResType::NSS && n.eq_ignore_ascii_case(&name) {
                return Some(source.clone());
            }
            let k = ResKey::parse(n, t)?;
            module
                .get(&k)
                .map(<[u8]>::to_vec)
                .or_else(|| resman?.get(&k).ok().map(|d| d.into_owned()))
        });
        c.compile(&name)
    };
    match result {
        Ok(out) => {
            let ncs = ResKey::new(key.resref, ResType::NCS);
            app.actions.push(Action::Apply(Command::new(
                format!("Compile {key}"),
                vec![Edit::SetResource { key: ncs, data: Some(out.ncs) }],
            )));
            app.log.info(format!("{key}: compiled"));
        }
        Err(e) => app.log.error(e.message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlighting_keeps_the_text() {
        let src = "void main() { // hi\n  int x = 0x1F; string s = \"é\"; }";
        let job = highlight(src, true, Color32::WHITE);
        assert_eq!(job.text, src);
        assert!(job.sections.len() > 10);
    }
}

#[cfg(test)]
mod perf {
    /// Timing only: `cargo test --release -p mg-ui --lib highlight_speed -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn highlight_speed() {
        let Some(root) = mg_testkit::nwn_root() else { return };
        let game =
            mg_rules::GameData::open(&mg_resman::GameInstall::new(&root, None, "en")).unwrap();
        let data = game.resman.get_named("nwscript", mg_core::ResType::NSS).unwrap();
        let text = String::from_utf8_lossy(&data).into_owned();
        let t = std::time::Instant::now();
        let job = super::highlight(&text, true, egui::Color32::WHITE);
        println!("highlight: {:?}, {} sections", t.elapsed(), job.sections.len());
    }
}
