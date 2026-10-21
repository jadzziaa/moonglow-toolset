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
//! the item's parts: only the row is checked here). A row has no usable
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
        }
    }
}

impl std::error::Error for ModelProblem {}

impl AppearanceRow {
    /// Where an object of `kind` (a blueprint's or a placed object's
    /// fields) reads its appearance; `None` for the kinds with none
    /// checked here, and an item without a `BaseItem` (an inventory's
    /// entry, which names its blueprint). A field that is missing reads as
    /// 0, as the game reads it.
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
}

impl crate::GameData {
    /// [`AppearanceRow::of`] the object and [`AppearanceRow::model`] of it
    /// by the game's tables and resources: why its appearance has no
    /// usable model, if it has none.
    pub fn appearance_problem(&self, kind: ObjectKind, object: &Struct) -> Option<ModelProblem> {
        let at = AppearanceRow::of(kind, object)?;
        let table = self.table(at.table).ok();
        at.model(table.as_deref(), &self.resman).err()
    }
}

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
             2 Ghost c_ghost99 S\n",
        );
        put("genericdoors", ResType::TWODA, "2DA V2.0\n\n  Label ModelName\n0 Wood door_x\n");
        put("baseitems", ResType::TWODA, "2DA V2.0\n\n  label\n0 shortsword\n1 ****\n");
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
    }
}
