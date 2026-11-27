//! `mg merge-gff BASE OURS THEIRS`: two copies of a GFF resource (an area,
//! `module.ifo`, a blueprint), both made from a third, merged; git's merge
//! driver contract (`mg merge-gff %O %A %B`): the merge is written over
//! OURS, and the exit is a failure when the two changed something each
//! its own way.
//!
//! Each file is a GFF, its JSON (neverwinter.nim's, as a nasher project
//! keeps it) or its NWNT (`nwn_nwnt`'s, as a project with `gffFormat =
//! nwnt` keeps it), told by what is in it: git hands a driver files
//! without their names. A merge the same as OURS leaves the file untouched
//! (so a file nobody changed keeps every byte), one the same as THEIRS is
//! THEIRS's bytes, and any other is written in OURS's form: as `nasher
//! unpack` writes JSON or NWNT (with the project's float places and
//! codepage, from the nasher project `--project` names, else the one the
//! file is in, else the one the working folder is in: git runs a driver at
//! the repository's top), or as a GFF.
//!
//! On a conflict nothing is written over OURS: the edits that make OURS
//! the merge are written beside the file instead, for `mg apply`
//! ([`conflict_file`]), theirs' side of each conflict among them.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use mg_core::Codepage;
use mg_edit::Edit;
use mg_edit::diff::{gff_edits, place, place_order};
use mg_gff::{Gff, TextStyle, Value};
use mg_module::merge::{Conflict, LINES_CONFLICT, Prefer};
use mg_module::nasher::{Conversion, Form, Package, Settings};
use mg_resman::ResKey;
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
    /// The nasher project whose settings convert the text (`--project`).
    pub project: Option<&'a Path>,
}

/// A file as read: its tree, its bytes and the form it was in.
struct Read {
    gff: Gff,
    data: Vec<u8>,
    form: Form,
}

/// The form the bytes are in, by what they begin with: JSON (an object),
/// NWNT (`data_type`, as `nwn_nwnt` tells it) or a GFF.
fn form_of(data: &[u8]) -> Form {
    let data = data.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(data);
    let start = data.iter().position(|b| !b.is_ascii_whitespace()).unwrap_or(data.len());
    match &data[start..] {
        [b'{', ..] => Form::Json,
        d if d.starts_with(b"data_type") => Form::Nwnt,
        _ => Form::Plain,
    }
}

fn read(path: &Path, conv: &Conversion) -> Result<Read> {
    let data = std::fs::read(path).with_context(|| path.display().to_string())?;
    let name = path.display().to_string();
    let form = form_of(&data);
    let gff = if form.is_text() {
        conv.from_text(form, &name, &data).map_err(|e| anyhow::anyhow!("{e}"))?
    } else {
        Gff::read(&data).with_context(|| format!("{name} is no GFF, GFF JSON or NWNT"))?
    };
    Ok(Read { gff, data, form })
}

/// The settings of the nasher project at or above `dir`, if it is in one.
fn project_settings(dir: &Path) -> Option<Settings> {
    let dir = std::path::absolute(dir).ok()?;
    dir.ancestors().find(|d| Package::is_package(d)).map(Settings::read)
}

fn shown(v: &Option<Value>, cp: Codepage) -> String {
    let Some(v) = v else { return "(none)".into() };
    let short = |s: String| crate::diff::truncate_ellipsis(s, 60);
    match v {
        Value::String(b) | Value::ResRef(b) => format!("\"{}\"", short(cp.decode(b).into_owned())),
        // (Its English, else another language's, as lists show it.)
        Value::LocString(s) => match s.strings.is_empty() {
            false => format!("\"{}\"", short(s.shown_in(mg_core::Language::ENGLISH, cp))),
            true if !s.strref.is_none() => format!("strref {}", s.strref.0),
            true => "\"\"".into(),
        },
        Value::Void(b) => format!("{} bytes", b.len()),
        // (A conversation's lines: each list's length.)
        Value::Struct(s)
            if !s.fields.is_empty()
                && s.fields.iter().all(|f| matches!(f.value, Value::List(_))) =>
        {
            let len = |v: &Value| if let Value::List(l) = v { l.len() } else { 0 };
            (s.fields.iter())
                .map(|f| format!("{} {}", f.label, len(&f.value)))
                .collect::<Vec<_>>()
                .join(", ")
        }
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

fn parse_prefer<T>(prefer: Option<&str>, ours: T, theirs: T) -> Result<Option<T>> {
    match prefer {
        None => Ok(None),
        Some("ours" | "a") => Ok(Some(ours)),
        Some("theirs" | "b") => Ok(Some(theirs)),
        Some(other) => bail!("--prefer ours or theirs, not {other}"),
    }
}

pub(crate) fn merge_gff(args: &Args<'_>) -> Result<Output> {
    let prefer = parse_prefer(args.prefer, Prefer::Ours, Prefer::Theirs)?;
    let near = match args.path {
        Some(p) => p.parent().map(Path::to_path_buf).unwrap_or_default(),
        None => args.ours.parent().map(Path::to_path_buf).unwrap_or_default(),
    };
    let near = if near.as_os_str().is_empty() { PathBuf::from(".") } else { near };
    let settings = match args.project {
        Some(p) if Package::is_package(p) => Settings::read(p),
        Some(p) => bail!("--project {}: no nasher.cfg there", p.display()),
        None => {
            project_settings(&near).or_else(|| project_settings(Path::new("."))).unwrap_or_default()
        }
    };
    let conv = Conversion::of(&settings);
    let cp = conv.codepage;
    let (base, ours, theirs) =
        (read(args.base, &conv)?, read(args.ours, &conv)?, read(args.theirs, &conv)?);
    let merged =
        mg_module::merge::merge(&base.gff, &ours.gff, &theirs.gff, prefer.unwrap_or(Prefer::Ours));
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
    if merged.conflicts.iter().any(|c| c.path == LINES_CONFLICT) {
        out.note(format!(
            "{name}: one side deleted, moved or renumbered lines of the conversation and the \
             other changed its lines: merged by number, a change would land on another line, so \
             the lines are one side's or the other's, whole"
        ));
    }
    if crate::diff::mark_unsettled(&mut out, merged.conflicts.len(), prefer.is_some(), true, || {
        format!(
            "{name}: {} values were changed by both, each its own way: nothing written (--prefer \
             ours or theirs takes one side's)",
            merged.conflicts.len()
        )
    }) {
        conflict_file(args, &mut out, &base, &ours, &theirs, &merged, cp)?;
        return Ok(out);
    }
    let target = args.output.unwrap_or(args.ours);
    let (bytes, taken) = if merged.gff == ours.gff {
        (ours.data, "ours")
    } else if merged.gff == theirs.gff && theirs.form == ours.form {
        (theirs.data, "theirs")
    } else if ours.form == Form::Nwnt {
        if let Some(unkept) = mg_gff::nwnt_unkept(&merged.gff.root, cp) {
            bail!("{name}: {unkept}");
        }
        (mg_gff::to_nwnt(&merged.gff, cp, conv.float_places).into_bytes(), "merged")
    } else if ours.form == Form::Json {
        let j = mg_gff::to_json(&merged.gff, cp).with_context(|| name.clone())?;
        let style = TextStyle { float_places: Some(conv.float_places), ..TextStyle::NASHER };
        (mg_gff::to_json_text(&j, style).into_bytes(), "merged")
    } else {
        (merged.gff.to_bytes().with_context(|| name.clone())?, "merged")
    };
    let unchanged = taken == "ours" && args.output.is_none();
    if !unchanged {
        crate::write_bytes_whole(target, &bytes)?;
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

/// The name of the conflict file beside `file`: `area1.git.json.conflicts`
/// (not `.json`, which a nasher project's sources take for a resource).
fn conflict_file_path(file: &Path) -> PathBuf {
    let mut name = file.file_name().unwrap_or_default().to_os_string();
    name.push(".conflicts");
    file.with_file_name(name)
}

/// The resource a file is, by its name (`area1.git.json` is `area1.git`),
/// else by its name and its GFF's type (git's temporary copies have none).
fn resource_of(file: &Path, gff: &Gff) -> Option<ResKey> {
    let name = file.file_name()?.to_string_lossy().to_ascii_lowercase();
    let name = [".json", ".nwnt"].iter().find_map(|x| name.strip_suffix(x)).unwrap_or(&name);
    ResKey::from_filename(name).or_else(|| {
        let stem = Path::new(name).file_stem()?.to_string_lossy().into_owned();
        ResKey::from_filename(&format!("{stem}.{}", gff.file_type_str().to_ascii_lowercase()))
    })
}

/// What edits do to each list's items: by the list (its struct's steps
/// and its label), the places they insert or remove items at and the
/// items they change inside. Places are doubled: item `i` is `2i`, and
/// an item inserted before it `2i - 1`; a list set whole is every place.
type Touched = std::collections::HashMap<(Vec<mg_gff::Step>, String), (Vec<i64>, Vec<i64>)>;

fn touched(edits: &[Edit]) -> Touched {
    let mut out = Touched::new();
    for e in edits {
        let at = place(e);
        for (d, step) in at.0.iter().enumerate() {
            let last = d + 1 == at.0.len();
            let (list, i) = match step {
                mg_gff::Step::Item(l, i) => (l, *i as i64),
                // (A list set whole.)
                mg_gff::Step::Field(l) if last && matches!(e, Edit::SetField { .. }) => {
                    let entry = out.entry((at.0[..d].to_vec(), l.clone())).or_default();
                    entry.0.extend([i64::MIN / 2, i64::MAX / 2]);
                    continue;
                }
                mg_gff::Step::Field(_) => continue,
            };
            let entry = out.entry((at.0[..d].to_vec(), list.clone())).or_default();
            match e {
                Edit::InsertItem { .. } if last => entry.0.push(2 * i - 1),
                Edit::RemoveItem { .. } if last => entry.0.push(2 * i),
                _ => entry.1.push(2 * i),
            }
        }
    }
    out
}

/// Whether two conflicts' edits can't be taken one without the other: one
/// inserts or removes items of a list over a stretch the other changes
/// within. (Items both change inside, and items both insert at one place,
/// are not: the one later in the file is made first.)
fn overlap(a: &[Edit], b: &[Edit]) -> bool {
    let (ta, tb) = (touched(a), touched(b));
    let crosses = |x: &(Vec<i64>, Vec<i64>), y: &(Vec<i64>, Vec<i64>)| {
        let (Some(lo), Some(hi)) = (x.0.iter().min(), x.0.iter().max()) else { return false };
        let one_insert = |t: &(Vec<i64>, Vec<i64>)| match t.0[..] {
            [p] if p % 2 != 0 => Some(p),
            _ => None,
        };
        if one_insert(x).is_some() && one_insert(x) == one_insert(y) {
            return y.1.iter().any(|c| (lo..=hi).contains(&c));
        }
        y.0.iter().chain(&y.1).any(|c| (lo..=hi).contains(&c))
    };
    ta.iter().any(|(list, x)| tb.get(list).is_some_and(|y| crosses(x, y) || crosses(y, x)))
}

/// On a conflict, the edits that make OURS the merge, for `mg apply`,
/// written beside the file (`--path`, else OURS: [`conflict_file_path`]):
/// first what merged (theirs' changes that conflict with nothing), then
/// theirs' side of each conflict, each edit saying which in its `note`.
/// Applied whole, the file makes OURS the merge with theirs' side of every
/// conflict; a conflict's edits taken out of it keep ours' side of that
/// one. Each conflict's edits are the difference between the merge with
/// ours' side of every conflict and the merge with theirs' side of that
/// one, and they go last place first (a list's later items before its
/// earlier ones), so that the places they name hold whichever others are
/// kept; conflicts whose edits overlap are taken together ([`overlap`]).
/// The JSON's `conflicts` say which edits are each one's.
fn conflict_file(
    args: &Args<'_>,
    out: &mut Output,
    base: &Read,
    ours: &Read,
    theirs: &Read,
    merged: &mg_module::merge::Merged,
    cp: Codepage,
) -> Result<()> {
    let file = args.path.unwrap_or(args.ours);
    let Some(key) = resource_of(file, &ours.gff) else {
        out.note(format!("{}: no resource name to write its conflicts for", file.display()));
        return Ok(());
    };
    let n = merged.conflicts.len();
    let mut edits: Vec<(Edit, String)> = (gff_edits(key, &ours.gff, &merged.gff)?.into_iter())
        .map(|e| (e, "merged: theirs' change, which conflicts with nothing (keep it)".to_string()))
        .collect();
    // Theirs' side of the conflicts of a group, as edits of the merge.
    let side_of = |group: &[usize]| -> Result<Vec<Edit>> {
        let choose = |i: usize| if group.contains(&i) { Prefer::Theirs } else { Prefer::Ours };
        let theirs_side =
            mg_module::merge::merge_choosing(&base.gff, &ours.gff, &theirs.gff, &choose);
        Ok(gff_edits(key, &merged.gff, &theirs_side.gff)?)
    };
    let mut sides: Vec<(Vec<usize>, Vec<Edit>)> = Vec::with_capacity(n);
    for k in 0..n {
        sides.push((vec![k], side_of(&[k])?));
    }
    // Conflicts whose edits overlap (one inserts or removes items of a
    // list where the other edits: a list's order and a value in it) can't
    // be taken one without the other: they are one group, its edits made
    // for both.
    while let Some((a, b)) = (0..sides.len())
        .flat_map(|a| (a + 1..sides.len()).map(move |b| (a, b)))
        .find(|&(a, b)| overlap(&sides[a].1, &sides[b].1))
    {
        let (mut group, _) = sides.remove(b);
        group.extend(sides[a].0.iter().copied());
        group.sort_unstable();
        let edits = side_of(&group)?;
        sides[a] = (group, edits);
    }
    let first_place = |e: &[Edit]| e.first().map(|e| place_order(&place(e)));
    sides.sort_by(|(ka, a), (kb, b)| {
        first_place(b).cmp(&first_place(a)).then(kb.last().cmp(&ka.last()))
    });
    let mut of_conflict = vec![Vec::new(); n];
    for (group, side) in sides {
        let each = (group.iter())
            .map(|&k| {
                let c = &merged.conflicts[k];
                format!("{} (ours {}, theirs {})", c.path, shown(&c.ours, cp), shown(&c.theirs, cp))
            })
            .collect::<Vec<_>>();
        let numbers = group.iter().map(|k| (k + 1).to_string()).collect::<Vec<_>>();
        let note = match group.len() {
            1 => format!(
                "conflict {} of {n}, theirs' side: {}; take this out to keep ours'",
                numbers[0], each[0]
            ),
            _ => format!(
                "conflicts {} of {n}, theirs' side of each (their edits overlap, so they go \
                 together): {}; take these out to keep ours'",
                numbers.join(" and "),
                each.join("; ")
            ),
        };
        for e in side {
            for &k in &group {
                of_conflict[k].push(edits.len());
            }
            edits.push((e, note.clone()));
        }
    }
    let label = format!(
        "Merge {key}: theirs' changes, and theirs' side of {n} conflict{}",
        if n == 1 { "" } else { "s" }
    );
    let command = mg_edit::Command::new(label, edits.iter().map(|(e, _)| e.clone()).collect());
    let mut json = mg_edit::wire::command_to_json(&command, cp)?;
    if let Some(list) = json["edits"].as_array_mut() {
        for (e, (_, note)) in list.iter_mut().zip(&edits) {
            e[mg_edit::wire::NOTE] = json!(note);
        }
    }
    let path = conflict_file_path(file);
    let text = serde_json::to_string_pretty(&json)? + "\n";
    crate::write_bytes_whole(&path, text.as_bytes())?;
    if let Some(list) = out.json["conflicts"].as_array_mut() {
        for (c, at) in list.iter_mut().zip(&of_conflict) {
            c["edits"] = json!(at);
        }
    }
    out.json["conflict_file"] = json!(path.display().to_string());
    out.note(format!(
        "{}: the edits that make it the merge, theirs' side of each conflict among them (take a \
         conflict's out to keep ours'): mg apply MODULE {}",
        path.display(),
        path.display()
    ));
    Ok(())
}

/// `mg merge-claims BASE OURS THEIRS`: two copies of a project's claims
/// table merged by area ([`mg_module::claims::merge`]), git's merge driver
/// as [`merge_gff`] is: written over OURS (or `output`) in OURS's
/// encoding, a failure where an area was changed by both, each its own
/// way, unless `prefer` takes a side; a merge that is OURS leaves the file
/// as it was.
pub(crate) fn merge_claims(args: &Args<'_>) -> Result<Output> {
    use mg_module::claims::{Side, decode};
    let prefer = parse_prefer(args.prefer, Side::Ours, Side::Theirs)?;
    let bytes = |p: &Path| std::fs::read(p).with_context(|| p.display().to_string());
    let (base, ours, theirs) = (bytes(args.base)?, bytes(args.ours)?, bytes(args.theirs)?);
    let ((b, _), (o, utf8), (t, _)) = (decode(&base), decode(&ours), decode(&theirs));
    let name = args.path.unwrap_or(args.ours).display().to_string();
    let merged = mg_module::claims::merge(&b, &o, &t, prefer.unwrap_or(Side::Ours))
        .map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
    let text = |v: &Option<String>| v.clone().unwrap_or_else(|| "(none)".into());
    let mut out = Output::new(json!({
        "file": name,
        "conflicts": merged.conflicts.iter().map(|c| json!({
            "area": c.area, "base": c.base, "ours": c.ours, "theirs": c.theirs,
        })).collect::<Vec<_>>(),
    }));
    for c in &merged.conflicts {
        out.line(format!(
            "CONFLICT {}: {} -> ours {}, theirs {}",
            c.area,
            text(&c.base),
            text(&c.ours),
            text(&c.theirs)
        ));
    }
    if crate::diff::mark_unsettled(&mut out, merged.conflicts.len(), prefer.is_some(), true, || {
        format!(
            "{name}: {} areas were changed by both, each its own way: nothing written (--prefer \
             ours or theirs takes one side's)",
            merged.conflicts.len()
        )
    }) {
        return Ok(out);
    }
    let target = args.output.unwrap_or(args.ours);
    let unchanged = merged.text == *o && args.output.is_none();
    if !unchanged {
        let data = if utf8 {
            merged.text.into_bytes()
        } else {
            Codepage::WINDOWS_1252.encode_lossy(&merged.text)
        };
        crate::write_bytes_whole(target, &data)?;
    }
    out.json["written"] = json!((!unchanged).then(|| target.display().to_string()));
    out.note(match merged.conflicts.len() {
        0 if unchanged => format!("{name}: merged (ours already was the merge)"),
        0 => format!("{name}: merged"),
        n => format!(
            "{name}: merged, {n} areas changed by both taken from {}",
            if prefer == Some(Side::Theirs) { "theirs" } else { "ours" }
        ),
    });
    Ok(out)
}
