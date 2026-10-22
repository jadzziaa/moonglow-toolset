//! `mg nui`: the NUI Creator's windows from the command line, with the
//! checks and the generator the window uses (`mg-nui`).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use clap::Subcommand;
use mg_core::ResType;
use mg_edit::{Command, Workspace};
use mg_module::{Module, build};
use mg_nui::Severity;
use mg_resman::ResKey;
use serde_json::json;

use crate::{Cli, Output, compiled, install, module_resman, path_text};

/// `mg nui`'s commands.
#[derive(Subcommand)]
pub(crate) enum NuiCmd {
    /// Add a new NUI window to a module and save: its layout (NAME.jui, a
    /// window with a Close button) and what the NUI Creator keeps about it
    /// (NAME.txt). Refuses a name whose resources the module already has.
    New {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name, the ResRef of its resources: 1 to 14 lower
        /// case letters, digits or underscores (its scripts add `_o` and
        /// `_e`).
        name: String,
        /// List the resources it would add; change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Check a NUI window as the NUI Creator does: the layout is JSON the
    /// game's NUI functions take, and every bind has a value. Prints each
    /// problem; exits with an error if one is an error. Changes nothing.
    Validate {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (NAME.jui).
        name: String,
    },
    /// Write a NUI window's scripts into a module and save, as Build does
    /// in the NUI Creator: the include that opens it (NAME_o.nss) and its
    /// event script (NAME_e.nss, compiled; the code written into it is
    /// kept), and the module's scripts that include the opener compiled
    /// again, all as `mg compile` compiles. The window's scripts are all
    /// or nothing: a window with errors, an opener or event script edited
    /// by hand, or one that does not compile, changes nothing. A script of
    /// yours that includes the opener and does not compile is printed and
    /// keeps the compiled script it had.
    Generate {
        /// A module archive, folder or nasher project.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (NAME.jui).
        name: String,
        /// Check and compile, and list the resources it would change;
        /// change nothing.
        #[arg(long)]
        dry_run: bool,
    },
    /// Draw a NUI window into a PNG as the NUI Creator's preview draws it:
    /// the game's skin, pictures and fonts (from the install and the
    /// module's haks), nothing selected. It is not the game: verify spacing
    /// and events there. Needs a GPU; changes nothing.
    Render {
        /// A module archive, folder or nasher project; or a window's
        /// NAME.jui file, with its NAME.txt and the pictures it names
        /// beside it.
        module: PathBuf,
        /// The window's name: the ResRef of its layout (not given with a
        /// .jui file).
        name: Option<String>,
        /// The PNG to write (default: NAME.png in the current folder).
        #[arg(long, short)]
        out: Option<PathBuf>,
        /// The window's size in points, WIDTHxHEIGHT, in place of its
        /// geometry's.
        #[arg(long, value_parser = parse_size)]
        size: Option<[f32; 2]>,
        /// The UI scale, as the game's option: 1 is 100%, 1.5 is 150%
        /// (0.25 to 4). The picture is this many pixels a point.
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
    },
    /// Where the NUI Creator's preview puts each control of a NUI window:
    /// its rectangle in the window's points (the window's top left at 0,
    /// 0), and whether it is clipped, overlaps a neighbor, has no room or
    /// cuts its text. The same layout `render` draws. Changes nothing.
    Layout {
        /// A module archive, folder or nasher project; or a window's
        /// NAME.jui file, with its NAME.txt beside it.
        module: PathBuf,
        /// The window's name (not given with a .jui file).
        name: Option<String>,
        /// The window's size in points, WIDTHxHEIGHT, in place of its
        /// geometry's.
        #[arg(long, value_parser = parse_size)]
        size: Option<[f32; 2]>,
        /// The UI scale it is laid out at (0.25 to 4).
        #[arg(long, default_value_t = 1.0)]
        scale: f32,
    },
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

pub(crate) fn run(cli: &Cli, cmd: &NuiCmd) -> Result<Output> {
    match cmd {
        NuiCmd::New { module, name, dry_run } => {
            let ws = Workspace::new(Module::open(module)?);
            let made =
                mg_nui::create(&ws.module, name).with_context(|| window_text(module, name))?;
            write(ws, made, module, name, *dry_run, Output::default())
        }
        NuiCmd::Generate { module, name, dry_run } => {
            let ws = Workspace::new(Module::open(module)?);
            let mut out = Output::default();
            let gi = install(cli)?;
            let rm = module_resman(&gi, &ws.module, &mut out)?;
            // What `mg compile` and the window remember of the scripts
            // they compiled is kept in one place: these are noted there.
            let dir = build::compiled_dir();
            let made_with = build::made_with(Some(module), None, Some(&gi.root), &rm);
            let known = build::Remembered::kept(dir.as_deref(), Some(module), made_with);
            let how = mg_nui::Build { known: Some(&known), ..mg_nui::Build::new(&rm) };
            let made = mg_nui::generate(&ws.module, name, &how)
                .with_context(|| window_text(module, name))?;
            // (A script of the module's that includes the opener and does
            // not compile: said as `mg compile` says it.)
            let failed = compiled(&mut out, &made.failed);
            let mut out = write(ws, made.command, module, name, *dry_run, out)?;
            out.json["failed"] = failed["failed"].clone();
            out.json["errors"] = failed["errors"].clone();
            if out.json["saved"] == true
                && let Some(dir) = &dir
                && let Err(e) = made.remembered.write(&build::Remembered::file(dir, module))
            {
                out.note(format!("warning: what was compiled is not remembered: {e}"));
            }
            Ok(out)
        }
        NuiCmd::Validate { module, name } => validate(module, name),
        NuiCmd::Render { module, name, out, size, scale } => {
            render(cli, module, name.as_deref(), out.as_deref(), *size, *scale)
        }
        NuiCmd::Layout { module, name, size, scale } => {
            let mut out = Output::default();
            let shown = Shown::open(cli, module, name.as_deref(), &mut out)?;
            let options = mg_ui::nui_render::Options { scale: *scale, size: *size };
            let layout = mg_ui::nui_render::layout(&shown.window(), &options)
                .with_context(|| window_text(module, &shown.name))?;
            layout_output(module, &layout, out)
        }
    }
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
                            && let Some(f) =
                                mg_module::talk::find(&game.resman, &gi.tlk_dirs(), &custom)
                        {
                            game.set_custom_tlk(mg_tlk::Tlk::read(&f.data).ok());
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
fn render(
    cli: &Cli,
    module: &Path,
    name: Option<&str>,
    png: Option<&Path>,
    size: Option<[f32; 2]>,
    scale: f32,
) -> Result<Output> {
    let mut out = Output::default();
    let shown = Shown::open(cli, module, name, &mut out)?;
    let gpu = mg_render::Gpu::headless().ok_or_else(|| {
        anyhow!("no GPU adapter to draw with (a Vulkan, Metal, DirectX 12 or OpenGL driver)")
    })?;
    let options = mg_ui::nui_render::Options { scale, size };
    let picture = mg_ui::nui_render::render(&gpu, &shown.window(), &options)
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
        let mut notes: Vec<String> = Vec::new();
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

/// The window an error is about: `MODULE: window NAME`.
fn window_text(module: &Path, name: &str) -> String {
    format!("{}: window {name}", path_text(module))
}

/// Applies what `new` or `generate` made (all of it or none) and saves,
/// or with `dry_run` only checks that it applies.
fn write(
    mut ws: Workspace,
    made: Command,
    module: &Path,
    name: &str,
    dry_run: bool,
    mut out: Output,
) -> Result<Output> {
    let mut resources: Vec<ResKey> = Vec::new();
    for e in &made.edits {
        if !resources.contains(e.key()) {
            resources.push(*e.key());
        }
    }
    // (Said before it is written, a dry run too.)
    let left = left_out(&ws.module, &resources);
    ws.apply(made).with_context(|| window_text(module, name))?;
    let saved = !dry_run && !resources.is_empty();
    if saved {
        ws.save()?;
    }
    out.json = json!({
        "module": path_text(module),
        "window": name,
        "resources": resources.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "saved": saved,
        "dry_run": dry_run,
    });
    if let Some(left) = left {
        out.note(format!("warning: {}", left.said(&ws.module)));
        out.json["left_out"] = left.files(&ws.module).into();
        out.json["include"] = left.line.into();
    }
    for r in &resources {
        out.line(r.to_string());
    }
    let what = if dry_run { "would change" } else { "changed" };
    let plural = if resources.len() == 1 { "" } else { "s" };
    out.line(format!("{name}: {what} {} resource{plural}", resources.len()));
    Ok(out)
}

/// A window's files a nasher project's include leaves out: on disk where
/// it doesn't look, or to be written there.
struct LeftOut {
    files: Vec<PathBuf>,
    /// The `include` line that takes them in.
    line: String,
}

impl LeftOut {
    /// The files, as nasher.cfg's patterns name them: from the project's
    /// folder, with `/`.
    fn files(&self, m: &Module) -> Vec<String> {
        let root = m.project.as_deref().map(|p| p.root().to_path_buf()).unwrap_or_default();
        (self.files.iter())
            .map(|f| f.strip_prefix(&root).unwrap_or(f).display().to_string().replace('\\', "/"))
            .collect()
    }

    fn said(&self, m: &Module) -> String {
        let includes = m.project.as_deref().map(|p| p.target().includes.join(", "));
        format!(
            "{}: nasher.cfg's include ({}) leaves {} out: nasher won't pack {} and it won't be \
             read back. Add this line to the target's sources in nasher.cfg: {}",
            self.files(m).join(", "),
            includes.unwrap_or_default(),
            if self.files.len() == 1 { "it" } else { "them" },
            if self.files.len() == 1 { "it" } else { "them" },
            self.line
        )
    }
}

/// Which of `keys` the module's nasher project leaves out, if it is in one.
fn left_out(m: &Module, keys: &[ResKey]) -> Option<LeftOut> {
    let p = m.project.as_deref()?;
    let names: Vec<String> = keys.iter().map(|k| k.to_string().to_ascii_lowercase()).collect();
    let mut files: Vec<PathBuf> = (p.left_out.iter())
        .filter(|f| {
            let name = f.file_name().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
            names.contains(&name)
        })
        .cloned()
        .collect();
    files.extend(p.uncovered(keys));
    files.sort();
    files.dedup();
    let line = p.include_for(&files)?;
    Some(LeftOut { files, line })
}

/// `mg nui validate`: the NUI Creator's checks of one window, reported as
/// `mg verify` reports its findings.
fn validate(module: &Path, name: &str) -> Result<Output> {
    let m = Module::open(module)?;
    // (Any window the module has: not only one named as `new` names them.)
    let key = |ty: ResType| {
        ResKey::parse(name, ty).ok_or_else(|| anyhow!("{name:?} is not a resource's name"))
    };
    let jui = key(ResType::JUI)?;
    let txt = key(ResType::TXT)?;
    // In a nasher project whose include leaves the window out, that is
    // why it isn't in the module.
    let left = left_out(&m, &[jui, txt]);
    let Some(data) = m.get(&jui) else {
        match left {
            Some(left) => bail!("{jui} is not in the module: {}", left.said(&m)),
            None => bail!("{jui} is not in the module"),
        }
    };
    let window = mg_nui::parse(data).with_context(|| format!("{}: {jui}", path_text(module)))?;
    let settings = m
        .get(&txt)
        .map(mg_nui::Settings::parse)
        .transpose()
        .with_context(|| format!("{}: {txt}", path_text(module)))?
        .unwrap_or_default();
    let mut findings = mg_nui::validate(&window, &settings);
    if let Some(left) = left {
        findings.push(mg_nui::Diagnostic {
            severity: Severity::Warning,
            path: "nasher.cfg".into(),
            message: left.said(&m),
        });
    }
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count();
    let severity = |s: Severity| match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    let mut out = Output::new(json!({
        "module": path_text(module),
        "window": name,
        "errors": errors,
        "warnings": findings.len() - errors,
        "findings": findings.iter().map(|d| json!({
            "severity": severity(d.severity),
            "path": d.path,
            "message": d.message,
        })).collect::<Vec<_>>(),
    }));
    for d in &findings {
        out.line(format!("{}\t{}: {}", severity(d.severity), d.path, d.message));
    }
    out.line(format!("{} problems", findings.len()));
    if errors > 0 {
        out.failed = true;
        out.note(format!("mg: {jui} has errors"));
    }
    Ok(out)
}
