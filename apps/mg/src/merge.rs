//! `mg merge-gff BASE OURS THEIRS`: two copies of a GFF resource (an area,
//! `module.ifo`, a blueprint), both made from a third, merged; git's merge
//! driver contract (`mg merge-gff %O %A %B`): the merge is written over
//! OURS, and the exit is a failure when the two changed something each
//! its own way.
//!
//! Each file is a GFF or its JSON (neverwinter.nim's, as a nasher project
//! keeps it), told by what is in it: git hands a driver files without
//! their names. A merge the same as OURS leaves the file untouched (so a
//! file nobody changed keeps every byte), one the same as THEIRS is
//! THEIRS's bytes, and any other is written in OURS's form: as `nasher
//! unpack` writes JSON (with the project's float places and codepage, from
//! the nasher project the file is in), or as a GFF.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mg_core::Codepage;
use mg_gff::{Gff, TextStyle, Value};
use mg_module::nasher::{Conversion, Package, Settings};
use mg_schema::merge::{Conflict, Prefer};
use serde_json::json;

use crate::Output;

pub(crate) struct Args<'a> {
    pub base: &'a Path,
    pub ours: &'a Path,
    pub theirs: &'a Path,
    pub prefer: Option<&'a str>,
    pub output: Option<&'a Path>,
    /// The file's own path (git's `%P`): where its nasher project is
    /// looked for, and its name in the report.
    pub path: Option<&'a Path>,
}

/// A file as read: its tree, its bytes and whether it was JSON.
struct Read {
    gff: Gff,
    data: Vec<u8>,
    json: bool,
}

/// Whether the bytes are JSON (an object), not a GFF.
fn is_json(data: &[u8]) -> bool {
    let data = data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data);
    data.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{')
}

fn read(path: &Path, conv: &Conversion) -> Result<Read> {
    let data = std::fs::read(path).with_context(|| path.display().to_string())?;
    let name = path.display().to_string();
    let json = is_json(&data);
    let gff = if json {
        conv.from_source(&name, &data).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        Gff::read(&data).with_context(|| format!("{name} is no GFF or GFF JSON"))?
    };
    Ok(Read { gff, data, json })
}

/// The settings of the nasher project at or above `dir`, if it is in one.
fn project_settings(dir: &Path) -> Option<Settings> {
    let dir = std::path::absolute(dir).ok()?;
    dir.ancestors().find(|d| Package::is_package(d)).map(Settings::read)
}

fn shown(v: &Option<Value>, cp: Codepage) -> String {
    let Some(v) = v else { return "(none)".into() };
    let short = |s: String| match s.chars().count() > 60 {
        true => s.chars().take(59).chain(['…']).collect(),
        false => s,
    };
    match v {
        Value::String(b) | Value::ResRef(b) => format!("\"{}\"", short(cp.decode(b).into_owned())),
        Value::LocString(s) => match s.strings.first() {
            Some((_, text)) => format!("\"{}\"", short(cp.decode(text).into_owned())),
            None if !s.strref.is_none() => format!("strref {}", s.strref.0),
            None => "\"\"".into(),
        },
        Value::Void(b) => format!("{} bytes", b.len()),
        Value::Struct(s) => format!("a struct of {} fields", s.fields.len()),
        Value::List(l) => format!("a list of {}", l.len()),
        Value::Float(f) => format!("{f}"),
        Value::Double(f) => format!("{f}"),
        other => {
            let text = format!("{other:?}");
            // `Int(3)` as `3`.
            text.split_once('(').map_or(text.clone(), |(_, v)| v.trim_end_matches(')').into())
        }
    }
}

fn conflict_json(c: &Conflict, cp: Codepage) -> serde_json::Value {
    json!({
        "path": c.path,
        "base": c.base.as_ref().map(|_| shown(&c.base, cp)),
        "ours": c.ours.as_ref().map(|_| shown(&c.ours, cp)),
        "theirs": c.theirs.as_ref().map(|_| shown(&c.theirs, cp)),
    })
}

pub(crate) fn merge_gff(args: &Args<'_>) -> Result<Output> {
    let prefer = match args.prefer {
        None => None,
        Some("ours" | "a") => Some(Prefer::Ours),
        Some("theirs" | "b") => Some(Prefer::Theirs),
        Some(other) => bail!("--prefer ours or theirs, not {other}"),
    };
    let near = match args.path {
        Some(p) => p.parent().map(Path::to_path_buf).unwrap_or_default(),
        None => args.ours.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    let near = if near.as_os_str().is_empty() { PathBuf::from(".") } else { near };
    let settings = project_settings(&near).unwrap_or_default();
    let conv = Conversion { codepage: settings.codepage, float_places: settings.truncate_floats };
    let cp = conv.codepage;
    let (base, ours, theirs) =
        (read(args.base, &conv)?, read(args.ours, &conv)?, read(args.theirs, &conv)?);
    let merged =
        mg_schema::merge::merge(&base.gff, &ours.gff, &theirs.gff, prefer.unwrap_or(Prefer::Ours));
    let name = args.path.unwrap_or(args.ours).display().to_string();
    let mut out = Output::new(json!({
        "file": name,
        "conflicts": merged.conflicts.iter().map(|c| conflict_json(c, cp)).collect::<Vec<_>>(),
    }));
    for c in &merged.conflicts {
        out.line(format!(
            "CONFLICT {}: {} -> ours {}, theirs {}",
            c.path,
            shown(&c.base, cp),
            shown(&c.ours, cp),
            shown(&c.theirs, cp)
        ));
    }
    let unsettled = !merged.conflicts.is_empty() && prefer.is_none();
    out.json["merged"] = json!(!unsettled);
    if unsettled {
        out.failed = true;
        out.json["written"] = json!(null);
        out.note(format!(
            "{name}: {} values were changed by both, each its own way: nothing written (--prefer \
             ours or theirs takes one side's)",
            merged.conflicts.len()
        ));
        return Ok(out);
    }
    let target = args.output.unwrap_or(args.ours);
    let (bytes, taken) = if merged.gff == ours.gff {
        (ours.data, "ours")
    } else if merged.gff == theirs.gff && theirs.json == ours.json {
        (theirs.data, "theirs")
    } else if ours.json {
        let j = mg_gff::to_json(&merged.gff, cp).with_context(|| name.clone())?;
        let style = TextStyle { float_places: Some(conv.float_places), ..TextStyle::NASHER };
        (mg_gff::to_json_text(&j, style).into_bytes(), "merged")
    } else {
        (merged.gff.to_bytes().with_context(|| name.clone())?, "merged")
    };
    let unchanged = taken == "ours" && args.output.is_none();
    if !unchanged {
        std::fs::write(target, &bytes).with_context(|| target.display().to_string())?;
    }
    out.json["written"] = json!((!unchanged).then(|| target.display().to_string()));
    out.json["result"] = json!(taken);
    out.note(match (taken, merged.conflicts.len()) {
        ("ours", 0) => format!("{name}: merged (ours already was the merge)"),
        ("theirs", 0) => format!("{name}: merged (theirs is the merge)"),
        (_, 0) => format!("{name}: merged"),
        (_, n) => format!(
            "{name}: merged, {n} values changed by both taken from {}",
            match prefer {
                Some(Prefer::Theirs) => "theirs",
                _ => "ours",
            }
        ),
    });
    Ok(out)
}
