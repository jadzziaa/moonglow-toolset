//! The model an object's appearance names, and why it may have none: the
//! one rule the blueprint previews (`mg_preview`), Verify's content doctor
//! (its `object-row` check) and the Properties' warnings under an
//! Appearance field go by.
//!
//! A placeable's `Appearance` is a placeables.2da row whose `ModelName` is
//! its model; a door's a doortypes.2da row (`Model`), or for a generic door
//! (`Appearance` 0) a genericdoors.2da row (`ModelName`) in
//! [`generic_door_field`]; a creature's `Appearance_Type` an appearance.2da
//! row whose `RACE` is its model, unless the row is a body of parts
//! (`MODELTYPE` P), whose `RACE` is the letter of its parts' names; an
//! item's `BaseItem` a baseitems.2da row (its model is put together from
//! the item's parts: only the row is checked here); a waypoint's
//! `Appearance` a waypoint.2da row whose `RESREF` is the flag the toolset
//! draws it as (the game draws none). A row has no usable
//! model when it isn't in the table, names none, is reserved (`USER`, the
//! rows a table keeps for builders, which Aurora shows as nothing), or
//! names a model the module, its haks and the game lack. A missing model
//! is worse than nothing to see: Aurora's area view often crashes on it.

use mg_2da::TwoDa;
use mg_core::ResType;
use mg_gff::Struct;
use mg_resman::{ResKey, ResMan};

use crate::ObjectKind;
use crate::rows::generic_door_field;

/// Where an object's appearance is read: the field, the row it holds, and
/// the table and column that name the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppearanceRow {
    /// The object's field that holds the row.
    pub field: &'static str,
    pub row: i64,
    pub table: &'static str,
    /// The column that names the model (an item's: its label).
    pub column: &'static str,
    /// The column a row that is there has something in (its label).
    label: &'static str,
    look: Look,
}

/// What the row's column holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Look {
    /// A model.
    Model,
    /// A model, unless the row is a body of parts (`MODELTYPE` P).
    Creature,
    /// Nothing to check but that the row is there.
    Row,
}

/// Why an appearance has no model to draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The table has no such row (or the row is empty).
    NotARow,
    /// The row is there, its model column empty.
    NamesNone,
    /// The row's model is `USER`: kept for builders, with no appearance.
    Reserved,
    /// The model the row names (lower case), which the module, its haks
    /// and the game lack.
    Missing(String),
    /// A body of parts' skeleton (its phenotype's, lower case), which the
    /// module, its haks and the game lack, as they lack its default
    /// phenotype's: its parts have nothing to hang on.
    Skeleton(String),
    /// A model one of the object's parts names (`field`, with its number),
    /// which the module, its haks and the game lack ([`crate::models`]).
    PartMissing(Box<PartModel>),
    /// A model that is there and can't be read (`field`: a part's; `None`:
    /// the row's own), and why: what Verify finds reading the module's
    /// and its haks' models.
    Unreadable(Box<Unread>),
}

/// A part's model ([`Problem::PartMissing`]): the object's field with the
/// part's number, and the model (lower case).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartModel {
    pub field: &'static str,
    pub model: String,
}

/// A model that is there and can't be read ([`Problem::Unreadable`]):
/// the part's field (`None`: the row's own model), the model and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unread {
    pub field: Option<&'static str>,
    pub model: String,
    pub why: String,
}

impl Problem {
    /// A problem of one of the object's parts (an item's piece, an armor's
    /// or a body's part), not of its appearance's row: the object is
    /// drawn, without it.
    pub fn is_part(&self) -> bool {
        match self {
            Problem::PartMissing(_) => true,
            Problem::Unreadable(u) => u.field.is_some(),
            _ => false,
        }
    }
}

impl ModelProblem {
    /// The object's field the problem is read from: a part's, or the
    /// row's.
    pub fn field(&self) -> &'static str {
        match &self.problem {
            Problem::PartMissing(p) => p.field,
            Problem::Unreadable(u) => u.field.unwrap_or(self.at.field),
            _ => self.at.field,
        }
    }
}

/// An appearance with no usable model: where it was read, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelProblem {
    pub at: AppearanceRow,
    pub problem: Problem,
}

impl std::fmt::Display for ModelProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let AppearanceRow { table, row, column, .. } = self.at;
        match &self.problem {
            Problem::NotARow => write!(f, "No such row: {table}.2da has no row {row}"),
            Problem::NamesNone => {
                write!(f, "No model: {table}.2da row {row} names none (its {column} is empty)")
            }
            Problem::Reserved => {
                write!(f, "No model: {table}.2da row {row} is reserved (its {column} is USER)")
            }
            Problem::Missing(model) => write!(
                f,
                "No model: {model} ({table}.2da row {row}, {column}) isn't in the module, its \
                 haks or the game"
            ),
            Problem::Skeleton(model) => write!(
                f,
                "No model: the skeleton {model} ({table}.2da row {row}, a body of parts) isn't \
                 in the module, its haks or the game"
            ),
            Problem::PartMissing(p) => {
                let PartModel { field, model } = &**p;
                write!(
                    f,
                    "No model: {model} (its {field}) isn't in the module, its haks or the game"
                )
            }
            Problem::Unreadable(u) if u.field.is_some() => {
                let Unread { field, model, why } = &**u;
                let field = field.unwrap_or_default();
                write!(
                    f,
                    "Unreadable model: {model} (its {field}) is there and can't be read: {why}"
                )
            }
            Problem::Unreadable(u) => write!(
                f,
                "Unreadable model: {} ({table}.2da row {row}, {column}) is there and can't be \
                 read: {}",
                u.model, u.why
            ),
        }
    }
}

impl std::error::Error for ModelProblem {}

impl AppearanceRow {
    /// Where an object of `kind` (a blueprint's or a placed object's
    /// fields) reads its appearance; `None` for the kinds with none
    /// checked here, an item without a `BaseItem` (an inventory's entry,
    /// which names its blueprint) and a waypoint without an `Appearance`
    /// (as many of BioWare's blueprints are). A field that is missing reads
    /// as 0, as the game reads it.
    pub fn of(kind: ObjectKind, object: &Struct) -> Option<AppearanceRow> {
        let int = |label: &str| object.integer(label).unwrap_or(0);
        let at = |field, table, column, label, look| AppearanceRow {
            field,
            row: int(field),
            table,
            column,
            label,
            look,
        };
        Some(match kind {
            ObjectKind::Placeable => {
                at("Appearance", "placeables", "ModelName", "Label", Look::Model)
            }
            ObjectKind::Door if int("Appearance") == 0 => {
                at(generic_door_field(object), "genericdoors", "ModelName", "Label", Look::Model)
            }
            ObjectKind::Door => at("Appearance", "doortypes", "Model", "Label", Look::Model),
            ObjectKind::Creature => {
                at("Appearance_Type", "appearance", "RACE", "LABEL", Look::Creature)
            }
            ObjectKind::Item => {
                object.integer("BaseItem")?;
                at("BaseItem", "baseitems", "label", "label", Look::Row)
            }
            // (Not one without the field: BioWare's blueprints often have
            // none, and Aurora places them with a flag, row 1.)
            ObjectKind::Waypoint => {
                object.integer("Appearance")?;
                at("Appearance", "waypoint", "RESREF", "LABEL", Look::Model)
            }
            _ => return None,
        })
    }

    /// The model the row names (lower case), by `table` (the row's table
    /// as the game resolves it; `None`: there is none) and the resources
    /// the model is looked for in. `Ok(None)`: the row is there and has no
    /// model of its own to check (a body of parts, a base item).
    pub fn model(
        &self,
        table: Option<&TwoDa>,
        resman: &ResMan,
    ) -> Result<Option<String>, ModelProblem> {
        let fail = |problem| Err(ModelProblem { at: *self, problem });
        let cell = |column: &str| {
            let r = usize::try_from(self.row).ok()?;
            table?.get(r, column).map(str::trim).filter(|v| !v.is_empty())
        };
        let model = cell(self.column);
        if cell(self.label).is_none() && (self.look == Look::Row || model.is_none()) {
            return fail(Problem::NotARow);
        }
        let parts = cell("MODELTYPE").is_some_and(|t| t.to_ascii_uppercase().starts_with('P'));
        if self.look == Look::Row || (self.look == Look::Creature && parts) {
            return Ok(None);
        }
        let Some(model) = model else { return fail(Problem::NamesNone) };
        if model.eq_ignore_ascii_case("USER") {
            return fail(Problem::Reserved);
        }
        let model = model.to_ascii_lowercase();
        if !ResKey::parse(&model, ResType::MDL).is_some_and(|k| resman.contains(&k)) {
            return fail(Problem::Missing(model));
        }
        Ok(Some(model))
    }

    /// The models the object's parts name ([`crate::models`]), of a row
    /// that is there ([`model`](Self::model)): an item's (its pieces, a
    /// cloak's, an armor's parts) and a body of parts' (its skeleton, its
    /// head and parts). Those found, each with its part's field, and
    /// those that aren't there. `tables` gives the 2DAs by name
    /// (baseitems, cloakmodel, appearance, gender, phenotype; `None`:
    /// not there).
    pub fn parts<'t>(
        &self,
        object: &Struct,
        tables: &dyn Fn(&str) -> Option<&'t TwoDa>,
        resman: &ResMan,
    ) -> PartModels {
        use crate::models;
        let mut out = PartModels::default();
        let named = match self.table {
            "baseitems" => {
                let Some(baseitems) = tables("baseitems") else { return out };
                models::item_models(object, baseitems, tables("cloakmodel"))
            }
            "appearance" => {
                let Some(appearance) = tables("appearance") else { return out };
                let skeletons =
                    models::skeletons(object, appearance, tables("gender"), tables("phenotype"));
                let Some(skeletons) = skeletons else { return out };
                match skeletons.iter().find(|s| models::has_model(resman, s)) {
                    Some(s) => out.found.push((self.field, s.clone())),
                    None => {
                        let problem = Problem::Skeleton(skeletons[0].clone());
                        out.problems.push(ModelProblem { at: *self, problem });
                        return out;
                    }
                }
                models::body_models(object, &skeletons)
            }
            _ => return out,
        };
        let mut missing = Vec::new();
        for n in named {
            match n.models.iter().find(|m| models::has_model(resman, m)) {
                Some(m) => out.found.push((n.field, m.clone())),
                None => missing.push((n.field, n.models[0].clone())),
            }
        }
        // An item with no model of its own is drawn as its base item's
        // DefaultModel (not armor: a body wears it).
        let item = self.table == "baseitems";
        let armor = item
            && usize::try_from(self.row)
                .ok()
                .and_then(|r| tables("baseitems")?.get(r, "ModelType"))
                == Some("3");
        let drawn_instead = (item && !armor && out.found.is_empty())
            .then(|| {
                let r = usize::try_from(self.row).ok()?;
                let d = tables("baseitems")?.get(r, "DefaultModel")?.trim().to_ascii_lowercase();
                models::has_model(resman, &d).then_some(d)
            })
            .flatten();
        // (An item none of whose own models are there and whose
        // DefaultModel is, is drawn as that, as the game draws the many of
        // its own that are so: its potions, most of its miscellaneous
        // items. Not a problem.)
        if drawn_instead.is_some() {
            return out;
        }
        for (field, model) in missing {
            let problem = Problem::PartMissing(Box::new(PartModel { field, model }));
            out.problems.push(ModelProblem { at: *self, problem });
        }
        out
    }
}

/// The models of an object's parts ([`AppearanceRow::parts`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PartModels {
    /// Those that are there: each part's field and its model (lower case).
    pub found: Vec<(&'static str, String)>,
    /// Those that aren't.
    pub problems: Vec<ModelProblem>,
}

impl crate::GameData {
    /// [`AppearanceRow::of`] the object and [`AppearanceRow::model`] of it
    /// by the game's tables and resources: why its appearance has no
    /// usable model, if it has none; else the first of its parts' models
    /// that isn't there ([`appearance_problems`](Self::appearance_problems)).
    pub fn appearance_problem(&self, kind: ObjectKind, object: &Struct) -> Option<ModelProblem> {
        self.appearance_problems(kind, object).into_iter().next()
    }

    /// Why the object's appearance has no usable model (one problem), or
    /// else which of the models its parts name aren't there
    /// ([`AppearanceRow::parts`]): what the Properties' warning under its
    /// Appearance says.
    pub fn appearance_problems(&self, kind: ObjectKind, object: &Struct) -> Vec<ModelProblem> {
        let Some(at) = AppearanceRow::of(kind, object) else { return Vec::new() };
        let table = self.table(at.table).ok();
        if let Err(e) = at.model(table.as_deref(), &self.resman) {
            return vec![e];
        }
        let names = ["baseitems", "cloakmodel", "appearance", "gender", "phenotype"];
        let tables: Vec<(&str, std::sync::Arc<TwoDa>)> =
            names.iter().filter_map(|n| Some((*n, self.table(n).ok()?))).collect();
        let lookup = |name: &str| tables.iter().find(|(n, _)| *n == name).map(|(_, t)| &**t);
        at.parts(object, &lookup, &self.resman).problems
    }

    /// `choices`, a list of the rows of an object's appearance (a
    /// placeable's, a creature's, a door's or a waypoint's), each row
    /// with no model to draw ([`appearance_problem`](Self::appearance_problem))
    /// marked [`NO_MODEL`] after its name: what the Properties' Appearance
    /// lists show. A door's row 0, the generic door (its model another
    /// field's), is not judged. Made once until the tables or resources
    /// change ([`invalidate`](Self::invalidate)), not each frame.
    pub fn appearance_choices(
        &self,
        kind: ObjectKind,
        choices: &[crate::Choice],
    ) -> std::sync::Arc<Vec<crate::Choice>> {
        let field = match kind {
            ObjectKind::Creature => "Appearance_Type",
            _ => "Appearance",
        };
        let key = (kind, choices.len(), choices.first().map(|c| c.text.clone()));
        let cache = &self.marked;
        if let Some(made) = cache.read().expect("marked cache poisoned").get(&key) {
            return made.clone();
        }
        let marked: Vec<crate::Choice> = choices
            .iter()
            .map(|c| {
                let mut object = Struct::new(0);
                object.set(field, mg_gff::Value::Int(c.row as i32));
                let judged = !(kind == ObjectKind::Door && c.row == 0);
                match judged.then(|| self.appearance_problem(kind, &object)).flatten() {
                    Some(_) => crate::Choice { row: c.row, text: format!("{}{NO_MODEL}", c.text) },
                    None => c.clone(),
                }
            })
            .collect();
        let marked = std::sync::Arc::new(marked);
        cache.write().expect("marked cache poisoned").insert(key, marked.clone());
        marked
    }
}

impl crate::GameData {
    /// Forgets the Appearance lists' marks (a model added to the
    /// resources or taken out: a row may have one now, or none), as
    /// [`invalidate`](Self::invalidate) does with the rest.
    pub fn forget_appearance_marks(&self) {
        self.marked.write().expect("marked cache poisoned").clear();
    }
}

/// What an Appearance list says after a row with no model to draw.
pub const NO_MODEL: &str = "  (no model)";

#[cfg(test)]
mod tests {
    use mg_gff::Value;
    use mg_resman::{LayerClass, MemContainer, priority};

    use super::*;

    fn resman() -> ResMan {
        let mut mem = MemContainer::new();
        let mut put = |name: &str, t: ResType, data: &str| {
            mem.insert(ResKey::parse(name, t).unwrap(), data.as_bytes());
        };
        put(
            "placeables",
            ResType::TWODA,
            "2DA V2.0\n\n  Label ModelName\n0 Chair plc_chair\n1 Gone plc_x99\n2 Empty ****\n\
             3 Mine USER\n4 **** ****\n",
        );
        put(
            "appearance",
            ResType::TWODA,
            "2DA V2.0\n\n  LABEL RACE MODELTYPE\n0 Dwarf D P\n1 Badger c_badger S\n\
             2 Ghost c_ghost99 S\n3 Gnome G P\n",
        );
        put("genericdoors", ResType::TWODA, "2DA V2.0\n\n  Label ModelName\n0 Wood door_x\n");
        put(
            "baseitems",
            ResType::TWODA,
            "2DA V2.0\n\n  label ItemClass ModelType DefaultModel\n\
             0 shortsword WSwSs 2 it_bag\n1 **** **** **** ****\n2 ring IT_Ring 0 it_bag\n\
             3 armor Armor 3 it_bag\n4 amulet IT_Amul 0 ****\n",
        );
        put("gender", ResType::TWODA, "2DA V2.0\n\n  LABEL GENDER\n0 Male M\n1 Female F\n");
        put("phenotype", ResType::TWODA, "2DA V2.0\n\n  Label DefaultPhenoType\n0 Normal 0\n");
        for model in [
            "pmd0",
            "pmd0_head001",
            "wswss_b_011",
            "wswss_m_011",
            "it_bag",
            "pfh0_chest012",
            "it_ring_001",
        ] {
            put(model, ResType::MDL, "");
        }
        put(
            "waypoint",
            ResType::TWODA,
            "2DA V2.0\n\n  LABEL RESREF STRREF\n0 **** **** ****\n1 blue gi_waypoint01 67281\n\
             2 red gi_waypoint02 67282\n",
        );
        put("gi_waypoint01", ResType::MDL, "");
        put("plc_chair", ResType::MDL, "");
        put("c_badger", ResType::MDL, "");
        let mut rm = ResMan::new();
        rm.add(priority::KEY, "game", LayerClass::Key, mem);
        rm
    }

    fn check(kind: ObjectKind, fields: &[(&str, i64)]) -> Result<Option<String>, ModelProblem> {
        let rm = resman();
        let mut s = Struct::default();
        for (label, v) in fields {
            s.set(label, Value::Dword(*v as u32));
        }
        let at = AppearanceRow::of(kind, &s).expect("an appearance");
        let data = rm.get(&ResKey::parse(at.table, ResType::TWODA).unwrap()).unwrap();
        let table = TwoDa::parse(&data, Default::default()).unwrap();
        at.model(Some(&table), &rm)
    }

    fn problem(kind: ObjectKind, fields: &[(&str, i64)]) -> String {
        check(kind, fields).expect_err("a problem").to_string()
    }

    /// The Properties' Appearance lists mark the rows with no model to
    /// draw, and only those; a door's generic row 0 is not judged.
    #[test]
    fn an_appearance_list_marks_the_rows_with_no_model() {
        let game = crate::GameData::new(resman(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let names = |table: &str| {
            let columns = crate::ChoiceColumns { name: None, label: Some("Label") };
            game.choices(table, columns).unwrap()
        };
        let rows = names("placeables");
        let marked = game.appearance_choices(ObjectKind::Placeable, &rows);
        let texts: Vec<&str> = marked.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            texts,
            ["Chair", "Gone  (no model)", "Empty  (no model)", "Mine  (no model)"],
            "row 4, empty, is no choice"
        );
        assert!(std::sync::Arc::ptr_eq(
            &marked,
            &game.appearance_choices(ObjectKind::Placeable, &rows)
        ));
        // (Made anew once a model came or went.)
        game.forget_appearance_marks();
        let again = game.appearance_choices(ObjectKind::Placeable, &rows);
        assert!(!std::sync::Arc::ptr_eq(&marked, &again) && marked == again);
        let columns = crate::ChoiceColumns { name: None, label: Some("LABEL") };
        let creatures = game.choices("appearance", columns).unwrap();
        let marked = game.appearance_choices(ObjectKind::Creature, &creatures);
        let texts: Vec<&str> = marked.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(
            texts,
            ["Dwarf", "Badger", "Ghost  (no model)", "Gnome  (no model)"],
            "a body of parts by its skeleton"
        );
        // (Row 0 of the doors is the generic door: another field's model.)
        let doors = vec![crate::Choice { row: 0, text: "Generic".into() }];
        assert_eq!(game.appearance_choices(ObjectKind::Door, &doors)[0].text, "Generic");
    }

    #[test]
    fn an_appearance_s_row_must_name_a_model_that_is_there() {
        let p = ObjectKind::Placeable;
        assert_eq!(check(p, &[("Appearance", 0)]), Ok(Some("plc_chair".into())));
        assert_eq!(
            problem(p, &[("Appearance", 1)]),
            "No model: plc_x99 (placeables.2da row 1, ModelName) isn't in the module, its haks \
             or the game"
        );
        assert_eq!(
            problem(p, &[("Appearance", 2)]),
            "No model: placeables.2da row 2 names none (its ModelName is empty)"
        );
        assert_eq!(
            problem(p, &[("Appearance", 3)]),
            "No model: placeables.2da row 3 is reserved (its ModelName is USER)"
        );
        // An empty row is none, as one past the end.
        for row in [4, 5] {
            let e = problem(p, &[("Appearance", row)]);
            assert_eq!(e, format!("No such row: placeables.2da has no row {row}"));
        }
        // No such table: no such row.
        let rm = resman();
        let at = AppearanceRow::of(p, &Struct::default()).unwrap();
        assert_eq!(at.model(None, &rm).unwrap_err().problem, Problem::NotARow);
    }

    #[test]
    fn doors_creatures_and_items_by_their_own_tables() {
        // A generic door (Appearance 0) by EE's GenericType_New.
        let e = check(ObjectKind::Door, &[("Appearance", 0), ("GenericType_New", 0)]).unwrap_err();
        assert_eq!(e.at.field, "GenericType_New");
        assert_eq!(e.problem, Problem::Missing("door_x".into()));
        // A body of parts names a letter, not a model.
        let c = ObjectKind::Creature;
        assert_eq!(check(c, &[("Appearance_Type", 0)]), Ok(None));
        assert_eq!(check(c, &[("Appearance_Type", 1)]), Ok(Some("c_badger".into())));
        assert_eq!(
            check(c, &[("Appearance_Type", 2)]).unwrap_err().problem,
            Problem::Missing("c_ghost99".into())
        );
        // An item: the row alone (an inventory's entry has no BaseItem).
        let i = ObjectKind::Item;
        assert_eq!(check(i, &[("BaseItem", 0)]), Ok(None));
        assert_eq!(problem(i, &[("BaseItem", 1)]), "No such row: baseitems.2da has no row 1");
        assert_eq!(AppearanceRow::of(i, &Struct::default()), None);
        // A waypoint: its flag, a waypoint.2da row's RESREF (row 0 is
        // blank, as the game's: a waypoint with no Appearance has none).
        let w = ObjectKind::Waypoint;
        assert_eq!(check(w, &[("Appearance", 1)]), Ok(Some("gi_waypoint01".into())));
        assert_eq!(
            check(w, &[("Appearance", 2)]).unwrap_err().problem,
            Problem::Missing("gi_waypoint02".into())
        );
        assert_eq!(problem(w, &[("Appearance", 0)]), "No such row: waypoint.2da has no row 0");
        assert_eq!(AppearanceRow::of(w, &Struct::default()), None);
    }

    /// An item's own models and a body's skeleton and parts are judged
    /// past the row: each one that isn't there.
    #[test]
    fn an_item_s_and_a_body_s_parts_must_be_there() {
        let game = crate::GameData::new(resman(), mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let said = |kind: ObjectKind, fields: &[(&str, i64)]| -> Vec<String> {
            let mut s = Struct::default();
            for (label, v) in fields {
                s.set(label, Value::Dword(*v as u32));
            }
            game.appearance_problems(kind, &s).iter().map(|p| p.to_string()).collect()
        };
        let i = ObjectKind::Item;
        // A short sword's top piece; its others are there.
        let sword = [("BaseItem", 0), ("ModelPart1", 11), ("ModelPart2", 11), ("ModelPart3", 4)];
        assert_eq!(
            said(i, &sword),
            ["No model: wswss_t_004 (its ModelPart3) isn't in the module, its haks or the game"]
        );
        // A ring with no model of its own is drawn as its DefaultModel, as
        // the game draws its own so; an amulet's names none that is there.
        assert_eq!(said(i, &[("BaseItem", 2), ("ModelPart1", 1)]), Vec::<String>::new());
        assert_eq!(said(i, &[("BaseItem", 2), ("ModelPart1", 9)]), Vec::<String>::new());
        assert_eq!(
            said(i, &[("BaseItem", 4), ("ModelPart1", 2)]),
            ["No model: it_amul_002 (its ModelPart1) isn't in the module, its haks or the game"]
        );
        // Armor: a part a man or a woman has will do; a part 0 is none.
        let armor = [("BaseItem", 3), ("ArmorPart_Torso", 12), ("ArmorPart_Neck", 0)];
        assert_eq!(said(i, &armor), Vec::<String>::new());
        let p = game.appearance_problem(i, &{
            let mut s = Struct::default();
            s.set("BaseItem", Value::Int(3));
            s.set("ArmorPart_RShoul", Value::Byte(7));
            s
        });
        let p = p.expect("a shoulder that isn't there");
        assert_eq!(p.field(), "ArmorPart_RShoul");
        assert!(p.problem.is_part());
        assert_eq!(
            p.problem,
            Problem::PartMissing(Box::new(PartModel {
                field: "ArmorPart_RShoul",
                model: "pmh0_shor007".into()
            }))
        );
        // (The row first: one with none says only that.)
        assert_eq!(said(i, &[("BaseItem", 1)]), ["No such row: baseitems.2da has no row 1"]);

        // A body of parts: its skeleton, then its head and parts.
        let c = ObjectKind::Creature;
        assert_eq!(
            said(c, &[("Appearance_Type", 0), ("Appearance_Head", 1)]),
            Vec::<String>::new()
        );
        assert_eq!(
            said(c, &[("Appearance_Type", 0), ("Appearance_Head", 2), ("BodyPart_LShin", 1)]),
            [
                "No model: pmd0_shinl001 (its BodyPart_LShin) isn't in the module, its haks or the \
                 game",
                "No model: pmd0_head002 (its Appearance_Head) isn't in the module, its haks or the \
                 game"
            ]
        );
        let gnome = game.appearance_problem(c, &{
            let mut s = Struct::default();
            s.set("Appearance_Type", Value::Word(3));
            s.set("Gender", Value::Byte(1));
            s
        });
        let gnome = gnome.expect("no skeleton");
        assert!(!gnome.problem.is_part() && gnome.field() == "Appearance_Type");
        assert_eq!(
            gnome.to_string(),
            "No model: the skeleton pfg0 (appearance.2da row 3, a body of parts) isn't in the \
             module, its haks or the game"
        );
        let unread = ModelProblem {
            at: AppearanceRow::of(c, &Struct::default()).unwrap(),
            problem: Problem::Unreadable(Box::new(Unread {
                field: Some("Appearance_Head"),
                model: "pmd0_head009".into(),
                why: "truncated".into(),
            })),
        };
        assert_eq!(
            unread.to_string(),
            "Unreadable model: pmd0_head009 (its Appearance_Head) is there and can't be read: \
             truncated"
        );
    }
}
