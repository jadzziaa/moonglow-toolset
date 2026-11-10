//! `mg texture convert`: textures converted between TGA, PNG and DDS
//! (BioWare's header and the standard one), from files or from the load
//! order, one to a file or many into a folder.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use mg_core::{ResRef, ResType};
use mg_image::{Format, Rgba, Texture, adjust, dds, encode};
use mg_resman::{ResKey, ResMan};
use serde_json::{Value, json};

use crate::{Cli, Output, added, path_text};

/// `mg texture`'s commands.
#[derive(clap::Subcommand)]
pub(crate) enum TextureCmd {
    /// Convert textures between TGA, PNG and DDS (BioWare's header, as
    /// the game's own are, or the standard one): one to a file (-o), or
    /// many into a folder (--to), each with what was written (format,
    /// size, mip levels, alpha). DDS is encoded DXT1 or DXT5 with every
    /// mip level; a DDS kept in its own format and size is copied, not
    /// encoded again. Fails if any texture can't be (once the rest are
    /// written).
    Convert(Convert),
}

/// Textures by file or by name, as `mg texture`'s commands take them.
#[derive(clap::Args)]
pub(crate) struct Inputs {
    /// A texture's file (.dds, .tga, .png), or a texture's name in the
    /// load order: `c_wolf.tga` that file, `c_wolf` the one the game
    /// draws (a DDS over a TGA). As many as needed.
    #[arg(value_name = "TEXTURE", required_unless_present = "list")]
    pub(crate) inputs: Vec<String>,
    /// More from a file, a line each (`-`: standard input; blank lines
    /// and lines beginning with `#` passed over).
    #[arg(long, value_name = "FILE")]
    pub(crate) list: Option<PathBuf>,
    /// A module (.mod, folder or nasher project) whose haks, and the
    /// module itself, are in the load order too.
    #[arg(long)]
    pub(crate) module: Option<PathBuf>,
    #[command(flatten)]
    pub(crate) added: added::Added,
}

#[derive(clap::Args)]
pub(crate) struct Convert {
    #[command(flatten)]
    inputs: Inputs,
    /// The file to write (one texture), its type by its extension
    /// (.dds, .tga, .png).
    #[arg(short, long, value_name = "FILE", conflicts_with_all = ["to", "kind"])]
    output: Option<PathBuf>,
    /// The folder to write into (made if missing), each texture by its
    /// name with the new type's extension.
    #[arg(long, value_name = "FOLDER", required_unless_present = "output")]
    to: Option<PathBuf>,
    /// With --to: the type to write.
    #[arg(long = "as", value_name = "TYPE", default_value = "dds")]
    kind: OutKind,
    /// DDS only: DXT1 (no alpha), DXT5 (alpha), or auto: a DDS's own
    /// format where it is copied, else DXT5 if any texel is less than
    /// opaque and DXT1 if none is. DXT1 asked of a picture with alpha
    /// keeps it in one bit (the game draws those texels black).
    #[arg(long, value_name = "FORMAT", default_value = "auto")]
    format: DxtChoice,
    /// DDS only: the standard header (`DDS `) rather than BioWare's,
    /// for other programs; the game reads both.
    #[arg(long)]
    standard: bool,
    /// DDS only: the largest level alone. (The client reads past a DDS
    /// of one level, noise at a distance: a texture's TXI can turn mip
    /// maps off instead.)
    #[arg(long)]
    no_mipmaps: bool,
    /// Make each side the nearest power of two (by ratio; halfway goes
    /// up), as the game's textures are.
    #[arg(long)]
    pow2: bool,
    /// The larger side at most this many texels, the picture scaled to
    /// fit and its shape kept (with --pow2, both sides halved until it
    /// fits).
    #[arg(long, value_name = "TEXELS", value_parser = clap::value_parser!(u32).range(1..=16384))]
    max_size: Option<u32>,
    /// Fill the color of hidden texels (alpha 0, or under ALPHA if given)
    /// from the visible ones around them, so filtering and mip levels
    /// don't bring what lay under a cut-out into its edge.
    #[arg(long, value_name = "ALPHA", num_args = 0..=1, default_missing_value = "1", require_equals = true)]
    bleed: Option<u8>,
    /// Write over files of the same names.
    #[arg(long)]
    force: bool,
    /// Say what would be written; write nothing.
    #[arg(long)]
    dry_run: bool,
}

/// What `--as` writes.
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OutKind {
    Dds,
    Tga,
    Png,
}

impl OutKind {
    fn extension(self) -> &'static str {
        match self {
            OutKind::Dds => "dds",
            OutKind::Tga => "tga",
            OutKind::Png => "png",
        }
    }

    fn of(path: &Path) -> Option<OutKind> {
        match crate::extension(path).as_str() {
            "dds" => Some(OutKind::Dds),
            "tga" => Some(OutKind::Tga),
            "png" => Some(OutKind::Png),
            _ => None,
        }
    }
}

/// What `--format` asks.
#[derive(clap::ValueEnum, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DxtChoice {
    Auto,
    Dxt1,
    Dxt5,
}

/// The kinds of texture file read.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    Dds,
    Tga,
    Png,
    Plt,
}

impl Kind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Kind::Dds => "dds",
            Kind::Tga => "tga",
            Kind::Png => "png",
            Kind::Plt => "plt",
        }
    }

    fn of_extension(ext: &str) -> Option<Kind> {
        match ext.to_ascii_lowercase().as_str() {
            "dds" => Some(Kind::Dds),
            "tga" => Some(Kind::Tga),
            "png" => Some(Kind::Png),
            "plt" => Some(Kind::Plt),
            _ => None,
        }
    }

    /// By the file's first bytes, for a file with no texture's extension:
    /// the magic of a standard DDS, a PNG and a PLT, else BioWare's DDS
    /// header where it holds together, else TGA.
    fn sniffed(data: &[u8]) -> Kind {
        let u32_at =
            |i: usize| data.get(i..i + 4).map(|b| u32::from_le_bytes(b.try_into().unwrap()));
        if data.starts_with(b"DDS ") {
            Kind::Dds
        } else if data.starts_with(b"\x89PNG") {
            Kind::Png
        } else if data.starts_with(b"PLT V1  ") {
            Kind::Plt
        } else if let (Some(w), Some(h), Some(c @ (3 | 4))) = (u32_at(0), u32_at(4), u32_at(8))
            && (1..=mg_image::MAX_SIDE).contains(&w)
            && (1..=mg_image::MAX_SIDE).contains(&h)
            && u32_at(12)
                == Some(
                    Format::level_size(if c == 3 { Format::Bc1 } else { Format::Bc3 }, w, h) as u32
                )
        {
            Kind::Dds
        } else {
            Kind::Tga
        }
    }

    fn restype(self) -> ResType {
        match self {
            Kind::Dds => ResType::DDS,
            Kind::Tga => ResType::TGA,
            Kind::Png => ResType::PNG,
            Kind::Plt => ResType::PLT,
        }
    }
}

/// A texture's bytes and where they came from.
pub(crate) struct Loaded {
    /// As it was given.
    pub(crate) given: String,
    /// The file's path, or the layer's label.
    pub(crate) from: String,
    /// The name to write it by (the file's stem, the resource's name).
    pub(crate) stem: String,
    pub(crate) kind: Kind,
    pub(crate) data: Vec<u8>,
}

/// Reads each texture given: a file where there is one, else by its name
/// from the load order (made once, the first time one is needed). One
/// that can't be is its error, by name.
pub(crate) fn load_all(
    cli: &Cli,
    inputs: &Inputs,
    out: &mut Output,
) -> Result<Vec<std::result::Result<Loaded, (String, String)>>> {
    let names = added::names(&inputs.inputs, inputs.list.as_deref())?;
    let mut rm: Option<ResMan> = None;
    let mut all = Vec::new();
    for given in names {
        let file = Path::new(&given);
        if file.is_file() {
            let loaded = std::fs::read(file).map(|data| {
                let ext = file.extension().and_then(|e| e.to_str()).unwrap_or_default();
                let kind = Kind::of_extension(ext).unwrap_or_else(|| Kind::sniffed(&data));
                let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned());
                Loaded {
                    given: given.clone(),
                    from: path_text(file),
                    stem: stem.unwrap_or_else(|| given.clone()),
                    kind,
                    data,
                }
            });
            all.push(loaded.map_err(|e| (given.clone(), format!("{}: {e}", file.display()))));
            continue;
        }
        if rm.is_none() {
            rm = Some(crate::resman_with(cli, inputs.module.as_deref(), &inputs.added, out)?);
        }
        let rm = rm.as_ref().expect("made above");
        all.push(by_name(rm, &given).map_err(|e| (given.clone(), format!("{e:#}"))));
    }
    Ok(all)
}

/// A texture by its name in the load order: with an extension that file,
/// without one the texture the game draws (a DDS over a TGA, as
/// `ResMan::texture` finds it), else a PLT of the name.
fn by_name(rm: &ResMan, given: &str) -> Result<Loaded> {
    let (stem, kind) = match given.rsplit_once('.') {
        Some((stem, ext)) => match Kind::of_extension(ext) {
            Some(kind) => (stem, Some(kind)),
            None => bail!("no such file, nor a texture's name (.dds, .tga, .png, .plt)"),
        },
        None => (given, None),
    };
    let resref =
        ResRef::from_str(stem).map_err(|_| anyhow!("no such file, nor a texture's name"))?;
    let (at, kind) = match kind {
        Some(kind) => {
            let key = ResKey::new(resref, kind.restype());
            (rm.find(&key).ok_or_else(|| anyhow!("{key} is found nowhere"))?, kind)
        }
        None => match rm.texture_layer(resref) {
            Some((t, at)) => (at, if t == ResType::DDS { Kind::Dds } else { Kind::Tga }),
            None => {
                let key = ResKey::new(resref, ResType::PLT);
                let at = rm
                    .find(&key)
                    .ok_or_else(|| anyhow!("{resref} is found nowhere (as .dds, .tga or .plt)"))?;
                (at, Kind::Plt)
            }
        },
    };
    let key = ResKey::new(resref, kind.restype());
    let data = rm.get_in(at, &key).with_context(|| key.to_string())?.into_owned();
    Ok(Loaded {
        given: given.to_string(),
        from: rm.layers()[at].label.clone(),
        stem: resref.to_string(),
        kind,
        data,
    })
}

/// The most pixels a PNG may unpack to: the largest texture's.
const PNG_PIXELS: u64 = mg_image::MAX_SIDE as u64 * mg_image::MAX_SIDE as u64;

/// A texture file read: DDS and TGA as the game keeps them, a PNG turned
/// over to the game's row order (the bottom first).
pub(crate) fn decode(l: &Loaded) -> Result<Texture> {
    let fail = |e: mg_image::ImageError| anyhow!("{}: {e}", l.given);
    match l.kind {
        Kind::Dds => dds::read(&l.data).map_err(fail),
        Kind::Tga => mg_image::tga::read(&l.data).map_err(fail),
        Kind::Png => {
            let picture = mg_image::png::read_picture(&l.data, PNG_PIXELS).map_err(fail)?;
            let alpha = picture.alpha_used();
            Ok(picture.top_down().into_texture(alpha))
        }
        Kind::Plt => bail!("{}: a PLT holds a shade and a layer per texel, not colors", l.given),
    }
}

/// A format's name, as the reports give it.
pub(crate) fn format_name(f: Format) -> &'static str {
    match f {
        Format::Rgba8 => "rgba8",
        Format::Bc1 => "dxt1",
        Format::Bc2 => "dxt3",
        Format::Bc3 => "dxt5",
        Format::Bc4 => "ati1",
        Format::Bc5 => "ati2",
    }
}

/// The size a texture is written at: the nearest powers of two if asked,
/// then the larger side brought within `max`.
fn target_size(w: u32, h: u32, pow2: bool, max: Option<u32>) -> (u32, u32) {
    let (mut w, mut h) = (w, h);
    if pow2 {
        (w, h) = (adjust::nearest_power_of_two(w), adjust::nearest_power_of_two(h));
    }
    if let Some(max) = max
        && w.max(h) > max
    {
        if pow2 {
            while w.max(h) > max {
                (w, h) = ((w / 2).max(1), (h / 2).max(1));
            }
        } else {
            let s = f64::from(max) / f64::from(w.max(h));
            w = ((f64::from(w) * s).round() as u32).clamp(1, max);
            h = ((f64::from(h) * s).round() as u32).clamp(1, max);
        }
    }
    (w, h)
}

/// One texture converted: the file's bytes and what the report says of it.
struct Made {
    bytes: Vec<u8>,
    report: Value,
    line: String,
    notes: Vec<String>,
}

fn convert_one(l: &Loaded, a: &Convert, kind: OutKind) -> Result<Made> {
    let tex = decode(l)?;
    let (sw, sh) = (tex.width, tex.height);
    let (w, h) = target_size(sw, sh, a.pow2, a.max_size);
    let resized = (w, h) != (sw, sh);
    let mut notes = Vec::new();
    // The picture as it will be written (for an encode).
    let prepared = || {
        let mut rgba = tex.stored_rgba();
        if let Some(below) = a.bleed {
            rgba = adjust::bleed(&rgba, below);
        }
        if resized {
            rgba = adjust::resize(&rgba, w, h);
        }
        rgba
    };
    let source = json!({
        "type": l.kind.name(),
        "format": format_name(tex.format),
        "width": sw,
        "height": sh,
        "mips": tex.mips.len(),
    });
    let (bytes, format, mips, has_alpha, reencoded, container) = match kind {
        OutKind::Dds => {
            let (t, reencoded) = dds_texture(&tex, a, resized, prepared, &mut notes)?;
            let bytes = match a.standard {
                true => dds::write_standard(&t),
                false => dds::write_bioware(&t),
            }
            .map_err(|e| anyhow!("{}: {e}", l.given))?;
            let container = if a.standard { "standard" } else { "bioware" };
            let alpha = t.alpha_used();
            (bytes, format_name(t.format), t.mips.len(), alpha, reencoded, Some(container))
        }
        OutKind::Tga => {
            let rgba = prepared();
            let alpha = rgba.alpha_used();
            let format = if alpha { "rgba8" } else { "rgb8" };
            (mg_image::tga::write(&rgba, alpha), format, 1, alpha, true, None)
        }
        OutKind::Png => {
            let rgba = prepared();
            let alpha = rgba.alpha_used();
            let top = rgba.top_down();
            let bytes = mg_image::png::write_rgba(top.width, top.height, &top.data)
                .map_err(|e| anyhow!("{}: {e}", l.given))?;
            (bytes, "rgba8", 1, alpha, true, None)
        }
    };
    let alpha_word = if has_alpha { ", alpha" } else { "" };
    let line =
        format!("{format} {w}×{h}, {mips} level{}{alpha_word}", if mips == 1 { "" } else { "s" });
    let report = json!({
        "input": l.given,
        "from": l.from,
        "type": kind.extension(),
        "container": container,
        "format": format,
        "width": w,
        "height": h,
        "mips": mips,
        "has_alpha": has_alpha,
        "reencoded": reencoded,
        "source": source,
    });
    Ok(Made { bytes, report, line, notes })
}

/// The DDS texture to write: the input's own blocks where its format and
/// size are kept (its mip levels made where it lacks them), else the
/// picture encoded. Whether it was encoded.
fn dds_texture(
    tex: &Texture,
    a: &Convert,
    resized: bool,
    prepared: impl FnOnce() -> Rgba,
    notes: &mut Vec<String>,
) -> Result<(Texture, bool)> {
    let compressed = tex.format != Format::Rgba8;
    let in_container = a.standard || matches!(tex.format, Format::Bc1 | Format::Bc3);
    let format_kept = match a.format {
        DxtChoice::Auto => true,
        DxtChoice::Dxt1 => tex.format == Format::Bc1,
        DxtChoice::Dxt5 => tex.format == Format::Bc3,
    };
    let want = |t: &Texture| match a.no_mipmaps {
        true => 1,
        false => (32 - t.width.max(t.height).leading_zeros()) as usize,
    };
    if compressed && in_container && format_kept && !resized && a.bleed.is_none() {
        let mut t = tex.clone();
        let want = want(&t);
        if t.mips.len() > want {
            t.mips.truncate(want);
        } else if t.mips.len() < want {
            match t.format {
                Format::Bc1 | Format::Bc3 => {
                    // The levels it lacks, made from its largest.
                    let rgba = t.stored_rgba();
                    let punch = t.format == Format::Bc1 && rgba.alpha_used();
                    let full = encode::compress(&rgba, t.format, true, punch);
                    let have = t.mips.len();
                    t.mips.extend(full.mips.into_iter().skip(have));
                }
                f => notes.push(format!(
                    "{} of {want} mip levels kept: {} levels are not made",
                    t.mips.len(),
                    format_name(f)
                )),
            }
        }
        if t.alpha_mean.is_none() {
            let mean = match t.format {
                Format::Bc1 => 1.0,
                _ => encode::mean_alpha(&t.stored_rgba()),
            };
            t.alpha_mean = Some(mean);
        }
        return Ok((t, false));
    }
    let rgba = prepared();
    let alpha = rgba.alpha_used();
    let format = match a.format {
        DxtChoice::Auto if alpha => Format::Bc3,
        DxtChoice::Auto | DxtChoice::Dxt1 => Format::Bc1,
        DxtChoice::Dxt5 => Format::Bc3,
    };
    let punch = format == Format::Bc1 && alpha;
    if punch {
        notes.push(
            "DXT1 keeps alpha in one bit, texels under half transparent: the game draws them \
             black (DXT5 keeps the alpha it draws)"
                .to_string(),
        );
    }
    Ok((encode::compress(&rgba, format, !a.no_mipmaps, punch), true))
}

/// `mg texture convert`.
pub(crate) fn convert(cli: &Cli, a: &Convert) -> Result<Output> {
    let mut out = Output::default();
    let loaded = load_all(cli, &a.inputs, &mut out)?;
    // Where each goes.
    let (kind, targets): (OutKind, Vec<PathBuf>) = match (&a.output, &a.to) {
        (Some(file), _) => {
            let Some(kind) = OutKind::of(file) else {
                bail!(
                    "{}: the file's extension says what to write (.dds, .tga, .png)",
                    file.display()
                )
            };
            if loaded.len() != 1 {
                bail!("{} textures: give --to FOLDER to write them into", loaded.len());
            }
            (kind, vec![file.clone()])
        }
        (None, Some(dir)) => {
            let targets = (loaded.iter())
                .map(|l| {
                    let stem = match l {
                        Ok(l) => l.stem.clone(),
                        Err((given, _)) => given.clone(),
                    };
                    dir.join(format!("{stem}.{}", a.kind.extension()))
                })
                .collect();
            (a.kind, targets)
        }
        (None, None) => bail!("give -o FILE or --to FOLDER"),
    };
    if a.no_mipmaps && kind == OutKind::Dds {
        out.note(
            "--no-mipmaps: the client reads past a DDS of one level (noise at a distance); a \
             TXI can turn mip maps off instead",
        );
    }
    // Two textures to one name, and files there already, before anything
    // is written.
    let mut seen = std::collections::HashMap::new();
    let mut twice = Vec::new();
    for (l, path) in loaded.iter().zip(&targets) {
        if l.is_err() {
            continue;
        }
        let key = path_text(path).to_ascii_lowercase();
        if let Some(first) = seen.insert(key, path) {
            twice.push(path_text(first));
        }
    }
    if !twice.is_empty() {
        bail!("two textures would be written to one file: {}", twice.join(", "));
    }
    let existing: Vec<String> = (loaded.iter().zip(&targets))
        .filter(|(l, p)| l.is_ok() && p.exists())
        .map(|(_, p)| path_text(p))
        .collect();
    if !existing.is_empty() && !a.force && !a.dry_run {
        bail!("{} written already: --force writes over", existing.join(", "));
    }
    if let (Some(dir), false) = (&a.to, a.dry_run) {
        std::fs::create_dir_all(dir).with_context(|| path_text(dir))?;
    }
    let (mut files, mut failed) = (Vec::new(), Vec::new());
    for (l, path) in loaded.iter().zip(&targets) {
        let made = match l {
            Ok(l) => convert_one(l, a, kind),
            Err((given, e)) => Err(anyhow!("{given}: {e}")),
        };
        let given = match l {
            Ok(l) => &l.given,
            Err((given, _)) => given,
        };
        match made {
            Ok(mut made) => {
                if !a.dry_run {
                    match a.output.is_some() {
                        true => crate::write_bytes_whole(path, &made.bytes)?,
                        false => crate::write_one_of_many(path, &made.bytes)?,
                    }
                }
                made.report["output"] = path_text(path).into();
                made.report["size"] = made.bytes.len().into();
                out.line(format!("{given}\t{}\t{}", path.display(), made.line));
                for n in made.notes {
                    out.note(format!("{given}: {n}"));
                }
                files.push(made.report);
            }
            Err(e) => {
                out.line(format!("{given}\tnot converted: {e:#}"));
                failed.push(json!({ "input": given, "error": format!("{e:#}") }));
            }
        }
    }
    let done = if a.dry_run { "would be written" } else { "written" };
    out.note(format!("{} of {} textures {done}", files.len(), files.len() + failed.len()));
    out.failed = !failed.is_empty();
    out.json = json!({ "written": !a.dry_run, "files": files, "failed": failed });
    Ok(out)
}
