//! Where a module names a row of a 2DA: the blueprints', areas' and placed
//! objects' fields that hold a row of it (a creature's appearance, an
//! item's base item, a placeable's, the classes, feats and spells of a
//! creature, an item property's subtype), and its scripts: a constant
//! of `nwscript` that stands for the row (`SPELL_FIREBALL`), and a
//! `Get2DAString` of the table at that row.
//!
//! The 2DA editor's **Find References to Row** asks it, with the tables'
//! own cells ([`doctor::column_kinds`](crate::doctor::column_kinds)).

use mg_core::{Codepage, ResType};
use mg_gff::{Gff, GffPath, Step, Struct, Value};
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::rows::{list_table, row_table};
use mg_script::lex::{Token, TokenKind, tokenize};
use mg_script::spec::Spec;

use crate::Module;
use crate::refs::Location;
use crate::rename::Usage;

/// The constants of `nwscript` whose values are rows of a table: their
/// names begin so (the longest beginning that fits is the constant's).
pub const CONSTANT_TABLES: [(&str, &str); 26] = [
    ("SPELL_", "spells"),
    ("SPELLABILITY_", "spells"),
    ("SPELL_SCHOOL_", "spellschools"),
    ("FEAT_", "feat"),
    ("CLASS_TYPE_", "classes"),
    ("RACIAL_TYPE_", "racialtypes"),
    ("APPEARANCE_TYPE_", "appearance"),
    ("BASE_ITEM_", "baseitems"),
    ("SKILL_", "skills"),
    ("VFX_DUR_", "visualeffects"),
    ("VFX_IMP_", "visualeffects"),
    ("VFX_FNF_", "visualeffects"),
    ("VFX_COM_", "visualeffects"),
    ("VFX_BEAM_", "visualeffects"),
    ("VFX_EYES_", "visualeffects"),
    ("AOE_", "vfx_persistent"),
    ("ITEM_PROPERTY_", "itempropdef"),
    ("PHENOTYPE_", "phenotype"),
    ("GENDER_", "gender"),
    ("POLYMORPH_TYPE_", "polymorph"),
    ("PACKAGE_", "packages"),
    ("TRAP_BASE_TYPE_", "traps"),
    ("TRACK_", "ambientmusic"),
    ("FOOTSTEP_TYPE_", "footstepsounds"),
    ("DISEASE_", "disease"),
    ("POISON_", "poison"),
];

/// The table whose rows a constant of `nwscript` stands for, by its name.
pub fn constant_table(name: &str) -> Option<&'static str> {
    CONSTANT_TABLES
        .iter()
        .filter(|(prefix, _)| name.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len())
        .map(|(_, table)| *table)
}

/// Where the module names row `row` of the table `table` (without
/// `.2da`): each field and each script line, in resource order. `game`
/// gives the item properties' tables (a property's subtype, its cost
/// table's and parameter table's rows) and `nwscript`'s constants.
pub fn row_uses(
    module: &Module,
    table: &str,
    row: usize,
    game: Option<&GameData>,
    cp: Codepage,
) -> Vec<Usage> {
    let spec = game.and_then(|g| g.resman.get_named("nwscript", ResType::NSS).ok());
    let spec = spec.map(|d| Spec::parse(&d));
    let mut keys: Vec<ResKey> = module.keys().copied().collect();
    keys.sort();
    let mut out = Vec::new();
    for key in keys {
        let Some(data) = module.get(&key) else { continue };
        if key.restype == ResType::NSS {
            out.extend(script_uses(key, data, table, row, spec.as_ref(), cp));
        } else if key.restype.is_gff()
            && let Ok(gff) = Gff::read(data)
        {
            out.extend(field_uses(key, &gff, table, row, game, cp));
        }
    }
    out
}

/// A field's whole value, where it is a number.
fn number(v: &Value) -> Option<i64> {
    v.as_integer()
}

/// The table an item property's field names a row of, by the property's
/// row of `itempropdef.2da`: `Subtype` its subtype table's, `CostValue`
/// its cost table's (by `iprp_costtable.2da`), `Param1Value` its
/// parameter table's (by `iprp_paramtable.2da`).
fn property_table(game: &GameData, property: &Struct, label: &str) -> Option<String> {
    let column_of = |table: &str, row: i64, column: &str| -> Option<String> {
        let t = game.table(table).ok()?;
        let cell = t.get(usize::try_from(row).ok()?, column)?;
        Some(cell.trim().to_ascii_lowercase())
    };
    match label {
        "Subtype" => {
            let p = property.integer("PropertyName")?;
            column_of("itempropdef", p, "SubTypeResRef")
        }
        "CostValue" => column_of("iprp_costtable", property.integer("CostTable")?, "Name"),
        "Param1Value" => column_of("iprp_paramtable", property.integer("Param1")?, "TableResRef"),
        _ => None,
    }
}

fn field_uses(
    key: ResKey,
    gff: &Gff,
    table: &str,
    row: usize,
    game: Option<&GameData>,
    cp: Codepage,
) -> Vec<Usage> {
    let file_type = String::from_utf8_lossy(&gff.file_type).into_owned();
    let mut found: Vec<GffPath> = Vec::new();
    gff.root.walk_fields(&mut |f| {
        let steps = &f.path.0;
        let above: Vec<&str> = steps[..steps.len() - 1].iter().map(Step::label).collect();
        // A list whose items stand for the rows, by their place (a
        // creature's skills): an item of some rank at the row's.
        if f.label == "Rank"
            && let Some((list, index)) = steps.len().checked_sub(2).and_then(|i| steps[i].item())
            && index == row
            && number(f.value).is_some_and(|r| r != 0)
            && list_table(&file_type, &above[..above.len() - 1], list)
                .is_some_and(|t| t.table.eq_ignore_ascii_case(table))
        {
            found.push(f.path.clone());
            return;
        }
        let Some(value) = number(f.value) else { return };
        if usize::try_from(value).ok() != Some(row) {
            return;
        }
        let named = match row_table(&file_type, &above, f.label) {
            Some(t) => t.table.eq_ignore_ascii_case(table),
            None => {
                f.list.is_some_and(|l| l == "PropertiesList")
                    && game
                        .and_then(|g| property_table(g, f.owner, f.label))
                        .is_some_and(|t| t.eq_ignore_ascii_case(table))
            }
        };
        if named {
            found.push(f.path.clone());
        }
    });
    found
        .into_iter()
        .map(|path| {
            let place = format!("{} › {}", crate::search::place(key, gff, &path, cp), path.label());
            Usage { from: key, path: Location::Gff(path), place }
        })
        .collect()
}

/// The script's lines that name the row: a constant of `nwscript` that
/// stands for it, and `Get2DAString("table", …, row)` (the row a number,
/// or a constant of `nwscript`'s or of the script's own; the table's name
/// written, or a string constant of the script's).
fn script_uses(
    key: ResKey,
    src: &[u8],
    table: &str,
    row: usize,
    spec: Option<&Spec>,
    cp: Codepage,
) -> Vec<Usage> {
    let toks: Vec<Token> = tokenize(src).into_iter().filter(|t| t.kind.is_significant()).collect();
    let text = |t: &Token| String::from_utf8_lossy(&src[t.span.clone()]).into_owned();
    let own = mg_script::outline::outline(src);
    let int_of = |name: &str| -> Option<i64> {
        let given = spec
            .and_then(|s| s.constants.iter().find(|c| c.name == name))
            .map(|c| c.value.clone())
            .or_else(|| own.globals.iter().find(|g| g.name == name).and_then(|g| g.value.clone()));
        crate::table_desc::whole(given?.trim())
    };
    let string_of = |name: &str| -> Option<String> {
        let g = own.globals.iter().find(|g| g.name == name && g.ty == "string")?;
        Some(g.value.as_deref()?.trim().trim_matches('"').to_string())
    };
    let row = row as i64;
    let mut at: Vec<(usize, String)> = Vec::new();
    for (i, t) in toks.iter().enumerate() {
        if t.kind != TokenKind::Ident {
            continue;
        }
        let name = text(t);
        if constant_table(&name).is_some_and(|c| c.eq_ignore_ascii_case(table))
            && spec.is_some_and(|s| s.constants.iter().any(|c| c.name == name))
            && int_of(&name) == Some(row)
        {
            at.push((t.span.start, name));
            continue;
        }
        if name != "Get2DAString" || toks.get(i + 1).map(text).as_deref() != Some("(") {
            continue;
        }
        // Its arguments, split at the commas outside brackets.
        let mut args: Vec<Vec<&Token>> = vec![Vec::new()];
        let mut depth = 0;
        for t in &toks[i + 2..] {
            match text(t).as_str() {
                "(" => depth += 1,
                ")" if depth == 0 => break,
                ")" => depth -= 1,
                "," if depth == 0 => {
                    args.push(Vec::new());
                    continue;
                }
                _ => {}
            }
            args.last_mut().expect("one at least").push(t);
        }
        let [name_arg, _, row_arg] = &args[..] else { continue };
        let named = match &name_arg[..] {
            [t] if matches!(t.kind, TokenKind::String { .. }) => {
                Some(text(t).trim_matches('"').into())
            }
            [t] if t.kind == TokenKind::Ident => string_of(&text(t)),
            _ => None,
        };
        let at_row = match &row_arg[..] {
            [t] if t.kind == TokenKind::Int => crate::table_desc::whole(&text(t)),
            [t] if t.kind == TokenKind::Ident => int_of(&text(t)),
            _ => None,
        };
        if named.is_some_and(|n: String| n.eq_ignore_ascii_case(table)) && at_row == Some(row) {
            at.push((t.span.start, "Get2DAString".into()));
        }
    }
    let lines = crate::rename::line_starts(src);
    at.into_iter()
        .map(|(start, what)| {
            let m = crate::rename::mention_at(key, src, &lines, start, cp);
            let place = format!("{} line {}: {what}  ({})", key.resref, m.line, m.text);
            Usage { from: key, path: Location::Line(m.line), place }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use mg_core::ResRef;
    use mg_gff::Gff;

    use super::*;

    fn key(name: &str, t: ResType) -> ResKey {
        ResKey::new(ResRef::from_str(name).unwrap(), t)
    }

    #[test]
    fn a_constant_s_table_is_its_longest_beginning_s() {
        assert_eq!(constant_table("SPELL_FIREBALL"), Some("spells"));
        assert_eq!(constant_table("SPELL_SCHOOL_EVOCATION"), Some("spellschools"));
        assert_eq!(constant_table("SPELLABILITY_AURA_BLINDING"), Some("spells"));
        assert_eq!(constant_table("VFX_IMP_HEALING_X"), Some("visualeffects"));
        assert_eq!(constant_table("OBJECT_TYPE_CREATURE"), None);
    }

    #[test]
    fn a_row_is_found_in_fields_and_scripts() {
        let mut m = Module::new();
        // A creature blueprint of appearance 6, with feat 6 and a skill
        // of rank 4 at skills.2da's row 6; a placeable of appearance 6
        // (placeables.2da's, not appearance's).
        let mut utc = Gff::new(*b"UTC ");
        utc.root.set("Appearance_Type", Value::Word(6));
        let mut feat = Struct::new(1);
        feat.set("Feat", Value::Word(6));
        utc.root.set("FeatList", Value::List(vec![feat]));
        let skills: Vec<Struct> = (0..8)
            .map(|i| {
                let mut s = Struct::new(0);
                s.set("Rank", Value::Byte(if i == 6 { 4 } else { 0 }));
                s
            })
            .collect();
        utc.root.set("SkillList", Value::List(skills));
        m.set(key("guard", ResType::UTC), utc.to_bytes().unwrap());
        let mut utp = Gff::new(*b"UTP ");
        utp.root.set("Appearance", Value::Dword(6));
        m.set(key("chest", ResType::UTP), utp.to_bytes().unwrap());
        // An area with a placed creature of appearance 6.
        let mut git = Gff::new(*b"GIT ");
        let mut placed = Struct::new(4);
        placed.set("Appearance_Type", Value::Word(6));
        placed.set("Tag", Value::String(b"BOB".to_vec()));
        git.root.set("Creature List", Value::List(vec![placed]));
        m.set(key("keep", ResType::GIT), git.to_bytes().unwrap());
        let src = b"const int MY_ROW = 6;\nconst string T = \"Appearance\";\nvoid main()\n{\n    \
                    string a = Get2DAString(\"appearance\", \"LABEL\", 6);\n    string b = \
                    Get2DAString(T, \"LABEL\", MY_ROW);\n    string c = Get2DAString(\"feat\", \
                    \"LABEL\", 6);\n    int n = APPEARANCE_TYPE_CAT_PANTHER;\n}\n";
        m.set(key("look", ResType::NSS), src.to_vec());
        let cp = Codepage::WINDOWS_1252;
        let places = |table: &str| -> Vec<String> {
            row_uses(&m, table, 6, None, cp).into_iter().map(|u| u.place).collect()
        };
        let appearance = places("appearance");
        assert_eq!(appearance.len(), 4, "{appearance:#?}");
        assert!(
            appearance[0].starts_with("guard.utc") && appearance[0].ends_with("Appearance_Type")
        );
        assert!(appearance[1].contains("creature BOB"), "{appearance:#?}");
        assert!(appearance[2].contains("line 5: Get2DAString"), "{appearance:#?}");
        assert!(appearance[3].contains("line 6: Get2DAString"), "{appearance:#?}");
        let feat = places("feat");
        assert_eq!(feat.len(), 2, "{feat:#?}");
        assert!(feat[0].ends_with("Feat") && feat[1].contains("line 7"), "{feat:#?}");
        let skills = places("skills");
        assert_eq!(skills.len(), 1, "{skills:#?}");
        assert!(skills[0].ends_with("Rank"), "{skills:#?}");
        let placeables = places("placeables");
        assert_eq!(placeables.len(), 1, "{placeables:#?}");
        assert!(row_uses(&m, "appearance", 7, None, cp).is_empty());
        // (A constant of nwscript's needs the game's nwscript: without it,
        // `APPEARANCE_TYPE_CAT_PANTHER` is no row's.)
    }
}
