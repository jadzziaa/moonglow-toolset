//! `mg bake-walkmesh`: Bake Walkmesh to Transform (and Unbake) from the
//! command line, as the area view's Position menu does
//! ([`mg_module::bake`]).

use std::path::Path;

use anyhow::{Result, anyhow, bail};
use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath, Workspace};
use mg_gff::Value;
use mg_module::Module;
use mg_module::bake;
use mg_resman::ResKey;
use serde_json::json;

use crate::{Cli, Output};

/// What `mg bake-walkmesh` was asked.
pub(crate) struct Asked<'a> {
    pub module: &'a Path,
    pub area: &'a str,
    pub objects: &'a [usize],
    pub unbake: bool,
    pub dry_run: bool,
    pub plugins: &'a [std::path::PathBuf],
    pub strict: bool,
}

pub(crate) fn run(cli: &Cli, a: &Asked) -> Result<Output> {
    let m = crate::open_module(a.module)?;
    let codepage = crate::module_codepage(cli, &m);
    let mut notes = Output::default();
    let guard = crate::save_guard(cli, &m, a.plugins, &mut notes)?.map(|g| g.strict(a.strict));
    let gi = crate::install(cli)?;
    let pf = &m.project_file;
    let rules = pf.bake.ready().map_err(|e| anyhow!("{e}"))?;
    let rows = pf.baked_rows().map_err(|e| anyhow!("{e}"))?;
    let rows_path = pf.bake_rows_path().ok_or_else(|| anyhow!("the project file has no place"))?;
    let area = ResRef::from_str(a.area).map_err(|e| anyhow!("--area {}: {e}", a.area))?;
    let git_key = ResKey::new(area, ResType::GIT);
    let git = match m.gff(&git_key) {
        Some(g) => g?,
        None => bail!("the module has no area {}", a.area),
    };
    let list = git.root.list("Placeable List").unwrap_or(&[]);
    let object = |n: usize| {
        list.get(n).ok_or_else(|| {
            anyhow!("{} has {} placeables: --object {n} is none of them", a.area, list.len())
        })
    };
    let appearance = |s: &mg_gff::Struct| s.integer("Appearance").unwrap_or(0) as u32;
    let mut edits = Vec::new();
    let mut each = Vec::new();
    let set = |n: usize, row: u32| Edit::SetField {
        key: git_key,
        path: GffPath::root().item("Placeable List", n),
        label: "Appearance".into(),
        value: Some(Value::Dword(row)),
    };
    if a.unbake {
        for &n in a.objects {
            let was = appearance(object(n)?);
            let row = bake::unbaked_row(&rows, was).map_err(|e| anyhow!("--object {n}: {e}"))?;
            edits.push(set(n, row));
            each.push(json!({ "object": n, "was": was, "row": row }));
        }
        let cmd = Command::new("Unbake Walkmesh", edits);
        let lines = each
            .iter()
            .map(|e| format!("{}[{}]: row {} again", a.area, e["object"], e["row"]))
            .collect();
        let mut out = crate::edits::run(
            Workspace::new(m),
            cmd,
            a.dry_run,
            Some(lines),
            false,
            codepage,
            guard.as_ref(),
            Some(&gi),
        )?;
        if let serde_json::Value::Object(o) = &mut out.json {
            o.insert("unbaked".into(), each.into());
        }
        out.notes.splice(0..0, notes.notes);
        return Ok(out);
    }
    let rm = crate::module_resman(&gi, &m, &mut notes)?;
    let mut requests = Vec::new();
    for &n in a.objects {
        let s = object(n)?;
        let t = bake::transform_of(s).map_err(|e| anyhow!("--object {n}: not baked: {e}"))?;
        let matrix = mg_preview::visual_matrix(t.scale[0], t.rotate, t.translate);
        requests.push(bake::Request {
            appearance: appearance(s),
            transform: t,
            matrix: matrix.to_cols_array_2d(),
        });
    }
    let plan = bake::plan(&rules, &rows, &rm, codepage, &requests).map_err(|e| anyhow!("{e}"))?;
    for (&n, b) in a.objects.iter().zip(&plan.each) {
        let b = b.as_ref().map_err(|e| anyhow!("--object {n}: not baked: {e}"))?;
        if appearance(object(n)?) != b.row {
            edits.push(set(n, b.row));
        }
        each.push(json!({
            "object": n,
            "row": b.row,
            "copy_of": b.copy_of,
            "model": b.model,
            "original": b.original,
            "label": b.label,
            "reused": b.reused,
        }));
    }
    // The hak at the top of the module's list, where it isn't in it.
    let info = m.info()?;
    let listed = mg_module::haks_of(&info.root);
    if !listed.iter().any(|h| h.eq_ignore_ascii_case(&plan.hak)) {
        edits.push(Edit::SetField {
            key: Module::info_key(),
            path: GffPath::root(),
            label: mg_schema::ifo::MOD_HAK_LIST.label.to_string(),
            value: Some(mg_module::attach::hak_list(&info.root, std::slice::from_ref(&plan.hak))),
        });
    }
    let hak_path = gi
        .user_hak_dir()
        .ok_or_else(|| anyhow!("no user folder to keep the hak in (--user-dir)"))?
        .join(format!("{}.hak", plan.hak));
    let mut lines: Vec<String> = plan.describe().lines().map(str::to_string).collect();
    if !a.dry_run {
        if !plan.files.is_empty() {
            let files = (plan.files.iter())
                .map(|(k, d)| (*k, mg_module::hak_edit::Source::Bytes(d.as_slice().into())))
                .collect();
            if let Some(dir) = hak_path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            mg_module::hak_edit::write_into(&hak_path, files).map_err(|e| anyhow!("{e}"))?;
            lines.push(format!("Written into {}", hak_path.display()));
        }
        if plan.rows_changed {
            plan.rows.write(&rows_path).map_err(|e| anyhow!("{e}"))?;
            lines.push(format!("Written {}", rows_path.display()));
        }
    }
    let cmd = Command::new("Bake Walkmesh to Transform", edits);
    let mut out = crate::edits::run(
        Workspace::new(m),
        cmd,
        a.dry_run,
        Some(lines),
        false,
        codepage,
        guard.as_ref(),
        Some(&gi),
    )?;
    if let serde_json::Value::Object(o) = &mut out.json {
        o.insert("baked".into(), each.into());
        o.insert("hak".into(), hak_path.display().to_string().into());
        o.insert("rows_file".into(), rows_path.display().to_string().into());
        let files: Vec<String> = plan.files.iter().map(|(k, _)| k.to_string()).collect();
        o.insert("files".into(), files.into());
    }
    out.notes.splice(0..0, notes.notes);
    Ok(out)
}
