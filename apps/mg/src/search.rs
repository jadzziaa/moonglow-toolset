//! `mg search`: a module's blueprints, placed objects, areas and the rest
//! found by what they hold (`mg_module::search`).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail};
use mg_core::ResType;
use mg_module::search::{self, AppearanceOf, FieldValue, Query, Search, VarType};
use mg_rules::GameData;
use serde_json::json;

use crate::{Cli, Output};

/// What `mg search` is asked: one condition of each kind, all of them
/// met by what is found (a creature with a class and a feat, an item
/// with a property and a variable).
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
    /// With --class: this many levels in it, or a range (`5..10`, `10..`,
    /// `..3`).
    #[arg(long, requires = "class", value_name = "N|MIN..MAX")]
    levels: Option<String>,
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
    /// Any field (or --field's), whole number or float, holding a number
    /// in this range (`5..10`, `0.5..`, `..3`; a float alone, `1.1`, that
    /// float).
    #[arg(
        long,
        value_name = "RANGE",
        conflicts_with_all = ["text", "int"],
        allow_negative_numbers = true
    )]
    number: Option<String>,
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
    /// Names and texts (--var, --value, --script, --field, --text) are
    /// regular expressions (`^kz_(rent|own)`), found anywhere unless
    /// anchored (`^…$`), letters A to Z in either case; not `*` patterns.
    #[arg(long)]
    regex: bool,
}

/// A row of a table given by number or name.
fn row(game: Option<&GameData>, table: &mg_rules::rows::RowTable, text: &str) -> Result<usize> {
    let choices = game.map(|g| search::table_choices(g, table)).unwrap_or_default();
    search::row_named(&choices, table.table, text).map_err(|e| anyhow!(e))
}

/// The conditions asked for, in the order the options are listed: what
/// is found meets all of them.
fn what(a: &Args, game: Option<&GameData>) -> Result<Vec<Search>> {
    if a.var.is_some() && a.property.is_some() && a.value.is_some() {
        bail!("--value is a variable's and a property's: search for one of them with it");
    }
    let row_of = |s: Search, text: &str| -> Result<usize> {
        row(game, &s.table().expect("a search of a table"), text)
    };
    let mut out = Vec::new();
    if let Some(name) = &a.var {
        let kind = a
            .var_type
            .as_deref()
            .map(|t| VarType::from_name(t).with_context(|| format!("{t}: not a variable type")))
            .transpose()?;
        out.push(Search::Variable { name: name.clone(), kind, value: a.value.clone() });
    }
    if let Some(p) = &a.property {
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
        out.push(Search::ItemProperty {
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
        });
    }
    if let Some(c) = &a.class {
        let class = row_of(Search::Class(0), c)? as i64;
        out.push(match &a.levels {
            None => Search::Class(class),
            Some(levels) => {
                let (min, max) = match FieldValue::number(levels).map_err(anyhow::Error::msg)? {
                    FieldValue::Number(n) => (n, n),
                    FieldValue::Range { min, max } => (
                        min.map_or(1, |m| m.ceil() as i64),
                        max.map_or(i64::from(u8::MAX), |m| m.floor() as i64),
                    ),
                    FieldValue::Text(_) => unreachable!("a number or a range"),
                };
                Search::ClassLevel { class, min, max }
            }
        });
    }
    if let Some(f) = &a.feat {
        out.push(Search::Feat(row_of(Search::Feat(0), f)? as i64));
    }
    if let Some(s) = &a.spell {
        out.push(Search::Spell(row_of(Search::Spell(0), s)? as i64));
    }
    if let Some(s) = &a.skill {
        let skill = row_of(Search::Skill { skill: 0, min_rank: 1 }, s)?;
        out.push(Search::Skill { skill, min_rank: a.min_rank.unwrap_or(1) });
    }
    if let Some(r) = &a.appearance {
        let of = match &a.of {
            Some(o) => AppearanceOf::from_name(o).with_context(|| {
                format!("{o}: creature, placeable, door, generic-door, waypoint or item")
            })?,
            None => AppearanceOf::Creature,
        };
        out.push(Search::Appearance { of, row: row(game, &of.table(), r)? as i64 });
    }
    if let Some(s) = &a.script {
        out.push(Search::Script(s.clone()));
    }
    if a.field.is_some() || a.int.is_some() || a.text.is_some() || a.number.is_some() {
        let value = match (a.int, &a.text, &a.number) {
            (Some(n), _, _) => Some(FieldValue::Number(n)),
            (_, Some(t), _) => Some(FieldValue::Text(t.clone())),
            (_, _, Some(n)) => Some(FieldValue::number(n).map_err(anyhow::Error::msg)?),
            _ => None,
        };
        out.push(Search::Field { label: a.field.clone(), value });
    }
    if out.is_empty() {
        bail!(
            "say what to search for: --var, --property, --class, --feat, --spell, --skill, \
             --appearance, --script, or --field, --int, --number or --text (several: all of \
             them met)"
        );
    }
    Ok(out)
}

pub(crate) fn run(cli: &Cli, a: &Args) -> Result<Output> {
    let m = crate::open_module(&a.module)?;
    let types =
        crate::parse_types(&a.types, ResType::is_gff, "a kind of object (utc, uti, are, …)")?;
    let mut out = Output::default();
    let game = crate::game_data_if_any(cli, &m, &mut out);
    let mut and = what(a, game.as_ref())?;
    let search = and.remove(0);
    let q = Query {
        search,
        and,
        types,
        placed: if a.placed {
            Some(true)
        } else if a.blueprints {
            Some(false)
        } else {
            None
        },
        regex: a.regex,
    };
    q.check().map_err(anyhow::Error::msg)?;
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
            "spawned": h.spawned.map(|r| format!("{}.utc", r.to_lowercase())),
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
