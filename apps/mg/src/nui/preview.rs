//! `mg nui render` and `mg nui layout`: the NUI Creator's preview, which
//! is the GUI's (mg-ui, drawn with wgpu); the feature `nui-preview`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use mg_core::ResType;
use mg_module::Module;
use mg_resman::ResKey;
use serde_json::json;

use super::window_text;
use crate::{Cli, Output, install, module_resman, path_text};

/// How `render` draws a window and `layout` lays it out: the same options,
/// so that the rectangles are the picture's.
#[derive(clap::Args)]
pub(crate) struct Drawn {
    /// The window's size in points, WIDTHxHEIGHT, in place of its
    /// geometry's.
    #[arg(long, value_parser = parse_size)]
    size: Option<[f32; 2]>,
    /// The UI scale, as the game's option: 1 is 100%, 1.5 is 150%
    /// (0.25 to 4). A picture is this many pixels a point.
    #[arg(long, default_value_t = 1.0)]
    scale: f32,
    /// A state to draw controls in: STATE for every control, STATE:ID for
    /// the one with that ID (which wins over every control's). STATE is
    /// normal, hover, pressed or disabled. Again for more: --state
    /// hover:send --state disabled:cancel. A control its binds disable
    /// stays disabled.
    #[arg(long = "state", value_name = "STATE[:ID]", value_parser = parse_state)]
    states: Vec<(Option<String>, mg_ui::nui_render::Look)>,
    /// Binds' values from a JSON file, in place of their first ones: an
    /// object of bind names and values as NuiSetBind sets them (a string,
    /// number, boolean, an array for a list's rows, {"r","g","b","a"} for
    /// a color, {"x","y","w","h"} for a rectangle).
    #[arg(long, value_name = "FILE.json")]
    binds: Option<PathBuf>,
    /// A swap layout variant (a key of the settings' `views`) to draw in
    /// its group, or as the whole window, in place of what it holds at
    /// first, as NuiSetGroupLayout shows it.
    #[arg(long, value_name = "NAME")]
    variant: Option<String>,
}

impl Drawn {
    /// As the preview takes them: the binds' file read, a state given
    /// twice for one control refused.
    pub(super) fn options(&self) -> Result<mg_ui::nui_render::Options> {
        let mut states = std::collections::BTreeMap::new();
        for (id, look) in &self.states {
            if let Some(was) = states.insert(id.clone(), *look)
                && was != *look
            {
                bail!(
                    "--state: {} is given {} and {}",
                    id.as_ref().map_or("every control".to_owned(), |id| format!("{id:?}")),
                    was.name(),
                    look.name()
                );
            }
        }
        let binds = match &self.binds {
            None => Default::default(),
            Some(file) => {
                let text = std::fs::read(file).with_context(|| path_text(file))?;
                let value: serde_json::Value = serde_json::from_slice(&text)
                    .with_context(|| format!("{}: not JSON", path_text(file)))?;
                let serde_json::Value::Object(map) = value else {
                    bail!(
                        "{}: give the binds as one JSON object, a bind's name and its value: \
                         {{\"message\": \"Hear ye\", \"can_send\": true}}",
                        path_text(file)
                    );
                };
                map.into_iter().collect()
            }
        };
        Ok(mg_ui::nui_render::Options {
            scale: self.scale,
            size: self.size,
            states,
            binds,
            variant: self.variant.clone(),
        })
    }
}

/// A state given as STATE or STATE:ID.
fn parse_state(
    text: &str,
) -> std::result::Result<(Option<String>, mg_ui::nui_render::Look), String> {
    let (look, id) = match text.split_once(':') {
        Some((look, "")) => {
            return Err(format!("{text:?}: give the control's ID after {look}:"));
        }
        Some((look, id)) => (look, Some(id.to_owned())),
        None => (text, None),
    };
    Ok((id, look.parse()?))
}

/// A size given as WIDTHxHEIGHT (or WIDTH,HEIGHT), in points.
fn parse_size(text: &str) -> std::result::Result<[f32; 2], String> {
    let (w, h) = text
        .split_once(['x', 'X', ','])
        .ok_or_else(|| format!("{text:?}: give the size as WIDTHxHEIGHT, such as 420x240"))?;
    let number = |t: &str| {
        t.trim()
            .parse::<f32>()
            .ok()
            .filter(|n| n.is_finite() && (1.0..=4096.0).contains(n))
            .ok_or_else(|| format!("{t:?}: a size is 1 to 4096 points"))
    };
    Ok([number(w)?, number(h)?])
}

/// A window to draw, with what it is drawn with: the module (or one made
/// of a .jui file and what is beside it), the game's resources with the
/// module's haks, and the talk tables.
struct Shown {
    module: Module,
    name: String,
    resman: Option<mg_resman::ResMan>,
    game: Option<mg_rules::GameData>,
}

impl Shown {
    fn open(cli: &Cli, path: &Path, name: Option<&str>, out: &mut Output) -> Result<Shown> {
        let file = crate::extension(path) == "jui";
        let (module, name) = match (file, name) {
            (true, None) => from_file(path)?,
            (true, Some(_)) => bail!(
                "{}: a .jui file is a window of its own; give a module and a window's name, \
                 or the file alone",
                path_text(path)
            ),
            (false, Some(name)) => (Module::open(path)?, name.to_owned()),
            (false, None) => {
                bail!("{}: which window? Give its name after the module", path_text(path))
            }
        };
        // Without the game the window is drawn as it is, without its skin:
        // said, not refused.
        let (resman, game) = match install(cli) {
            Ok(gi) => {
                // (A window's file, or a folder of windows, has no haks.)
                let rm = if module.contains(&Module::info_key()) {
                    module_resman(&gi, &module, out)?
                } else {
                    mg_resman::ResMan::for_game(&gi)?
                };
                let tlk = std::fs::read(gi.talk_table(false))
                    .ok()
                    .and_then(|b| mg_tlk::Tlk::read(&b).ok());
                match tlk {
                    Some(tlk) => {
                        let mut game = mg_rules::GameData::new(rm, tlk);
                        if let Some(custom) = module.custom_tlk().ok().flatten()
                            && let Some(f) = mg_module::talk::find_any(
                                &module,
                                &game.resman,
                                &gi.tlk_dirs(),
                                &custom,
                                Some(game.language),
                            )
                        {
                            game.set_custom_tlk(f.tlk().ok());
                        }
                        (None, Some(game))
                    }
                    None => (Some(rm), None),
                }
            }
            Err(e) => {
                out.note(format!(
                    "warning: {e:#}: the window is drawn without the game's skin, pictures and fonts"
                ));
                (None, None)
            }
        };
        Ok(Shown { module, name, resman, game })
    }

    fn window(&self) -> mg_ui::nui_render::Window<'_> {
        mg_ui::nui_render::Window {
            module: &self.module,
            resman: self.resman.as_ref().or(self.game.as_ref().map(|g| &g.resman)),
            game: self.game.as_ref(),
            name: &self.name,
        }
    }
}

/// A module of a window's file: NAME.jui, the NAME.txt beside it, and the
/// pictures it names that are beside it too.
fn from_file(path: &Path) -> Result<(Module, String)> {
    let stem =
        path.file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    let jui = ResKey::parse(&stem, ResType::JUI)
        .ok_or_else(|| anyhow!("{}: {stem:?} is not a resource's name", path_text(path)))?;
    let data = std::fs::read(path).with_context(|| path_text(path))?;
    let window = mg_nui::parse(&data).with_context(|| path_text(path))?;
    let mut m = Module::new();
    m.set(jui, data);
    let mut settings = mg_nui::Settings::default();
    let txt = path.with_extension("txt");
    if txt.is_file() {
        let bytes = std::fs::read(&txt).with_context(|| path_text(&txt))?;
        settings = mg_nui::Settings::parse(&bytes).with_context(|| path_text(&txt))?;
        m.set(ResKey::new(jui.resref, ResType::TXT), bytes);
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    for picture in mg_nui::image_names(&window, &settings) {
        for ty in [ResType::TGA, ResType::PNG, ResType::DDS, ResType::JPG, ResType::BMP] {
            let Some(key) = ResKey::parse(&picture, ty) else { continue };
            if let Ok(bytes) = std::fs::read(dir.join(key.to_string())) {
                m.set(key, bytes);
            }
        }
    }
    Ok((m, stem))
}

/// `mg nui render`.
pub(super) fn render(
    cli: &Cli,
    module: &Path,
    name: Option<&str>,
    png: Option<&Path>,
    options: &mg_ui::nui_render::Options,
) -> Result<Output> {
    let mut out = Output::default();
    let shown = Shown::open(cli, module, name, &mut out)?;
    let gpu = mg_render::Gpu::headless().ok_or_else(|| {
        anyhow!("no GPU adapter to draw with (a Vulkan, Metal, DirectX 12 or OpenGL driver)")
    })?;
    let picture = mg_ui::nui_render::render(&gpu, &shown.window(), options)
        .with_context(|| window_text(module, &shown.name))?;
    let file = png.map_or_else(|| PathBuf::from(format!("{}.png", shown.name)), Path::to_path_buf);
    std::fs::write(&file, picture.png()).with_context(|| path_text(&file))?;
    let layout = &picture.layout;
    out.json = json!({
        "module": path_text(module),
        "window": shown.name,
        "png": path_text(&file),
        "width": picture.width,
        "height": picture.height,
        "scale": layout.scale,
        "issues": layout.issues,
        "not_drawn": layout.not_drawn,
    });
    for issue in &layout.issues {
        out.note(format!("warning: {issue}"));
    }
    if !layout.not_drawn.is_empty() {
        out.note(format!(
            "warning: not drawn (see them in the game): {}",
            layout.not_drawn.join(", ")
        ));
    }
    out.line(format!("{}: {} × {} pixels", path_text(&file), picture.width, picture.height));
    Ok(out)
}

/// `mg nui layout`.
pub(super) fn layout(
    cli: &Cli,
    module: &Path,
    name: Option<&str>,
    options: &mg_ui::nui_render::Options,
) -> Result<Output> {
    let mut out = Output::default();
    let shown = Shown::open(cli, module, name, &mut out)?;
    let layout = mg_ui::nui_render::layout(&shown.window(), options)
        .with_context(|| window_text(module, &shown.name))?;
    layout_output(module, &layout, out)
}

/// `mg nui layout`'s answer: the layout as JSON, or a line a control.
fn layout_output(module: &Path, l: &mg_ui::nui_render::Layout, mut out: Output) -> Result<Output> {
    let mut json = serde_json::to_value(l)?;
    json["module"] = path_text(module).into();
    out.json = json;
    let b = &l.body;
    out.line(format!(
        "{}: {} × {} points at {}%, body at {} {}, {} × {}",
        l.window,
        l.width,
        l.height,
        (l.scale * 100.0).round(),
        b.x,
        b.y,
        b.w,
        b.h
    ));
    for e in &l.elements {
        let a = &e.area;
        let mut line = format!("{}\t{}", e.path, e.kind);
        if let Some(row) = e.row {
            line += &format!(" (row {row})");
        }
        if let Some(id) = &e.id {
            line += &format!(" #{id}");
        }
        line += &format!("\t{} {} {} × {}", a.x, a.y, a.w, a.h);
        let mut notes: Vec<String> = e.state.iter().map(|s| (*s).to_owned()).collect();
        for (on, what) in [
            (e.hidden, "hidden"),
            (e.zero_sized, "zero-sized"),
            (e.out_of_view && !e.zero_sized, "out of view"),
            (e.clipped && !e.out_of_view, "clipped"),
        ] {
            if on {
                notes.push(what.into());
            }
        }
        if !e.overlaps.is_empty() {
            notes.push(format!("overlaps {}", e.overlaps.join(", ")));
        }
        for cut in &e.cut_text {
            notes.push(format!("cuts {cut:?}"));
        }
        if let Some(h) = e.needs_height {
            notes.push(format!("its text needs a height of {h}"));
        }
        if !notes.is_empty() {
            line += &format!("\t{}", notes.join("; "));
        }
        out.line(line);
    }
    for issue in &l.issues {
        out.note(format!("warning: {issue}"));
    }
    Ok(out)
}
