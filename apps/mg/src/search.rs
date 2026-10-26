//! `mg search`: a module's blueprints, placed objects, areas and the rest
//! found by what they hold (`mg_module::search`).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::ResType;
use mg_module::Module;
use mg_module::search::{self, AppearanceOf, FieldValue, Query, Search, VarType};
use mg_rules::GameData;
use serde_json::json;

use crate::{Cli, Output};

/// What `mg search` is asked; one kind of search at a time.
#[derive(clap::Args)]
pub(crate) struct Args {
    module: PathBuf,
    /// Local variables by name (`*` matches any run of characters; case
    /// ignored), e.g. `*sDeidad*`.
    #[arg(long, value_name = "NAME")]
    var: Option<String>,
    /// With --var: only variables of this type (int, float, string,
    /// object, location).
    #[arg(long, value_name = "TYPE", requires = "var")]
    var_type: Option<String>,
    /// Item properties of this type (itempropdef.2da: a row number or its
    /// name, e.g. "Skill Bonus").
    #[arg(long, value_name = "TYPE")]
    property: Option<String>,
    /// With --property: of this subtype (a row of its subtype table or its
    /// name, e.g. Hide).
    #[arg(long, requires = "property")]
    subtype: Option<String>,
    /// With --property: of this parameter value (a row or its name).
    #[arg(long, requires = "property")]
    param: Option<String>,
    /// With --var, the value (`*` as in names); with --property, the
    /// property's value (a row of its cost table or its name, e.g. +5).
    #[arg(long)]
    value: Option<String>,
    /// Creatures with a level in this class (classes.2da: a row or its
    /// name).
    #[arg(long)]
    class: Option<String>,
    /// Creatures with this feat (feat.2da: a row or its name).
    #[arg(long)]
    feat: Option<String>,
    /// Creatures that know or have memorized this spell, or have it as a
    /// special ability (spells.2da: a row or its name).
    #[arg(long)]
    spell: Option<String>,
    /// Creatures with ranks in this skill (skills.2da: a row or its name).
    #[arg(long)]
    skill: Option<String>,
    /// With --skill: at least this many ranks (default 1).
    #[arg(long, requires = "skill")]
    min_rank: Option<u8>,
    /// Objects of this appearance (a row or its name; the table is --of's).
    #[arg(long, value_name = "ROW")]
    appearance: Option<String>,
    /// With --appearance: what it is of: creature (appearance.2da, the
    /// default), placeable, door, generic-door, waypoint or item (the base
    /// item).
    #[arg(long, requires = "appearance", value_name = "KIND")]
    of: Option<String>,
    /// Scripts attached to events and conversation lines, by name (`*` as
    /// in names).
    #[arg(long, value_name = "NAME")]
    script: Option<String>,
    /// Any field with this label (`*` as in names), alone or with --int or
    /// --text.
    #[arg(long, value_name = "LABEL")]
    field: Option<String>,
    /// Any field (or --field's) holding this whole number.
    #[arg(long, value_name = "N", conflicts_with = "text", allow_negative_numbers = true)]
    int: Option<i64>,
    /// Any field (or --field's) whose text contains this (case ignored;
    /// `*` as in names): strings, resrefs, every language of localized
    /// strings.
    #[arg(long)]
    text: Option<String>,
    /// Only these kinds of object: utc, utd, ute, uti, utm, utp, uts, utt,
    /// utw, are (areas), ifo (the module), dlg (conversations)...
    /// (comma-separated; default all). Items include those in inventories.
    #[arg(long = "type", value_delimiter = ',')]
    types: Vec<String>,
    /// Only objects placed in areas.
    #[arg(long, conflicts_with = "blueprints")]
    placed: bool,
    /// Only blueprints.
    #[arg(long)]
    blueprints: bool,
}

/// The game's data for the module (its haks' 2DAs and its talk table), if
/// the game is installed.
pub(crate) fn game_data(cli: &Cli, m: &Module, out: &mut Output) -> Option<GameData> {
    let gi = crate::install(cli).ok()?;
    let tlk = mg_tlk::Tlk::read(&std::fs::read(gi.talk_table(false)).ok()?).ok()?;
    let mut game = GameData::new(crate::module_resman(&gi, m, out).ok()?, tlk);
    if let Some(name) = m.custom_tlk().ok().flatten()
        && let Some(f) =
            mg_module::talk::find_any(m, &game.resman, &gi.tlk_dirs(), &name, Some(game.language))
    {
        game.set_custom_tlk(f.tlk().ok());
    }
    Some(game)
}

/// A row of a table given by number or name.
fn row(game: Option<&GameData>, table: &mg_rules::rows::RowTable, text: &str) -> Result<usize> {
    let choices = game.map(|g| search::table_choices(g, table)).unwrap_or_default();
    search::row_named(&choices, table.table, text).map_err(|e| anyhow!(e))
}

/// The search asked for.
fn what(a: &Args, game: Option<&GameData>) -> Result<Search> {
    let kinds = [
        a.var.is_some(),
        a.property.is_some(),
        a.class.is_some(),
        a.feat.is_some(),
        a.spell.is_some(),
        a.skill.is_some(),
        a.appearance.is_some(),
        a.script.is_some(),
        a.field.is_some() || a.int.is_some() || a.text.is_some(),
    ];
    match kinds.iter().filter(|k| **k).count() {
        0 => bail!(
            "say what to search for: --var, --property, --class, --feat, --spell, --skill, \
             --appearance, --script, or --field, --int or --text"
        ),
        1 => {}
        _ => bail!("search for one kind of thing at a time"),
    }
    let row_of = |s: Search, text: &str| -> Result<usize> {
        row(game, &s.table().expect("a search of a table"), text)
    };
    Ok(if let Some(name) = &a.var {
        let kind = a
            .var_type
            .as_deref()
            .map(|t| VarType::from_name(t).with_context(|| format!("{t}: not a variable type")))
            .transpose()?;
        Search::Variable { name: name.clone(), kind, value: a.value.clone() }
    } else if let Some(p) = &a.property {
        let property = u16::try_from(row_of(
            Search::ItemProperty { property: 0, subtype: None, value: None, param: None },
            p,
        )?)?;
        let parts = game.map(|g| search::property_parts(g, property, None)).unwrap_or_default();
        let pick = |choices: &[mg_rules::Choice], what: &str, text: &str| {
            search::row_named(choices, what, text).map_err(|e| anyhow!(e))
        };
        let subtype = match &a.subtype {
            Some(s) => Some(u16::try_from(pick(&parts.subtypes, "the subtype", s)?)?),
            None => None,
        };
        // (The parameter's table may be the subtype's.)
        let params = match (game, subtype) {
            (Some(g), Some(_)) => search::property_parts(g, property, subtype).params,
            _ => parts.params.clone(),
        };
        Search::ItemProperty {
            property,
            subtype,
            value: match &a.value {
                Some(v) => Some(u16::try_from(pick(&parts.values, "the value", v)?)?),
                None => None,
            },
            param: match &a.param {
                Some(v) => Some(u8::try_from(pick(&params, "the parameter", v)?)?),
                None => None,
            },
        }
    } else if let Some(c) = &a.class {
        Search::Class(row_of(Search::Class(0), c)? as i64)
    } else if let Some(f) = &a.feat {
        Search::Feat(row_of(Search::Feat(0), f)? as i64)
    } else if let Some(s) = &a.spell {
        Search::Spell(row_of(Search::Spell(0), s)? as i64)
    } else if let Some(s) = &a.skill {
        let skill = row_of(Search::Skill { skill: 0, min_rank: 1 }, s)?;
        Search::Skill { skill, min_rank: a.min_rank.unwrap_or(1) }
    } else if let Some(r) = &a.appearance {
        let of = match &a.of {
            Some(o) => AppearanceOf::from_name(o).with_context(|| {
                format!("{o}: creature, placeable, door, generic-door, waypoint or item")
            })?,
            None => AppearanceOf::Creature,
        };
        Search::Appearance { of, row: row(game, &of.table(), r)? as i64 }
    } else if let Some(s) = &a.script {
        Search::Script(s.clone())
    } else {
        let value = match (a.int, &a.text) {
            (Some(n), _) => Some(FieldValue::Number(n)),
            (None, Some(t)) => Some(FieldValue::Text(t.clone())),
            (None, None) => None,
        };
        Search::Field { label: a.field.clone(), value }
    })
}

pub(crate) fn run(cli: &Cli, a: &Args) -> Result<Output> {
    let m = Module::open(&a.module)?;
    let types = (a.types.iter())
        .map(|t| {
            ResType::from_extension(t.trim_start_matches('.'))
                .filter(|t| t.is_gff())
                .with_context(|| format!("{t}: not a kind of object (utc, uti, are, …)"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut out = Output::default();
    let game = game_data(cli, &m, &mut out);
    let q = Query {
        search: what(a, game.as_ref())?,
        and: Vec::new(),
        types,
        placed: if a.placed {
            Some(true)
        } else if a.blueprints {
            Some(false)
        } else {
            None
        },
    };
    let cp = crate::module_codepage(cli, &m);
    let hits = search::search(&m, &q, game.as_ref(), cp);
    out.json = json!({
        "found": hits.iter().map(|h| json!({
            "resource": h.key.to_string(),
            "path": h.path,
            "place": h.place,
            "match": h.matched,
            "type": h.object.extension(),
            "placed": h.placed(),
            "area": h.placed().then(|| h.key.resref.to_lowercase().to_string()),
            "index": h.instance().map(|(_, i)| i),
            "carried": h.carried.map(|r| format!("{}.uti", r.to_lowercase())),
        })).collect::<Vec<_>>(),
    });
    for h in &hits {
        out.line(format!("{}\t{}\t{} {}", h.place, h.matched, h.key, h.path));
    }
    if game.is_none() {
        out.note("warning: no game install found: what is found is named by numbers");
    }
    out.note(format!("{} found", hits.len()));
    Ok(out)
}
