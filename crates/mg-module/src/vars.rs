//! Local variables (a `VarTable`): their types, and their values typed as
//! text. `mg areas --var`, the Variables window and Advanced Search read
//! them through this one place.

use mg_core::Codepage;
use mg_gff::{FieldType, Value};

/// The type of a local variable (`VarTable` `Type`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarType {
    Int = 1,
    Float = 2,
    String = 3,
    Object = 4,
    Location = 5,
}

impl VarType {
    pub const ALL: [VarType; 5] =
        [VarType::Int, VarType::Float, VarType::String, VarType::Object, VarType::Location];

    /// `int`, `float`, `string`, `object`, `location`.
    pub fn name(self) -> &'static str {
        match self {
            VarType::Int => "int",
            VarType::Float => "float",
            VarType::String => "string",
            VarType::Object => "object",
            VarType::Location => "location",
        }
    }

    pub fn from_name(name: &str) -> Option<VarType> {
        VarType::ALL.into_iter().find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }

    /// Its number, as `Type` holds it.
    pub fn code(self) -> u32 {
        self as u32
    }

    /// The type numbered `n`.
    pub fn of(n: i64) -> Option<VarType> {
        VarType::ALL.into_iter().find(|t| i64::from(t.code()) == n)
    }

    /// The type its value is kept as, for the types set from text: an
    /// object (an id the game gives as it runs) and a location (a struct)
    /// are not.
    pub fn field_type(self) -> Option<FieldType> {
        match self {
            VarType::Int => Some(FieldType::Int),
            VarType::Float => Some(FieldType::Float),
            VarType::String => Some(FieldType::String),
            VarType::Object | VarType::Location => None,
        }
    }

    /// `text` as a value of this type (text in `codepage`, as the game
    /// reads a module's strings).
    pub fn value(self, text: &str, codepage: Codepage) -> Result<Value, String> {
        let ty = self.field_type().ok_or_else(|| format!("{} isn't set from text", self.name()))?;
        Value::from_text(ty, text, None, codepage)
    }

    /// The type of a value typed with none given: int, else float, else
    /// string. A float is digits with a point, signed or not (`2.5`,
    /// `-.5`, `3.`), as the game's own text-to-float reads one: `nan`,
    /// `inf`, `infinity` and `1e5`, which Rust reads as numbers too, are
    /// the words a builder typed.
    pub fn of_text(text: &str) -> VarType {
        let t = text.trim();
        let digits = t.strip_prefix(['+', '-']).unwrap_or(t);
        let pointed = digits.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            && digits.bytes().filter(|b| *b == b'.').count() == 1
            && digits.len() > 1;
        if t.parse::<i32>().is_ok() {
            VarType::Int
        } else if pointed && t.parse::<f32>().is_ok_and(f32::is_finite) {
            VarType::Float
        } else {
            VarType::String
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_are_typed_from_text_in_the_codepage() {
        let cp = Codepage::WINDOWS_1252;
        assert_eq!(VarType::of(3), Some(VarType::String));
        assert_eq!(VarType::String.value("café", cp), Ok(Value::String(b"caf\xe9".to_vec())));
        assert_eq!(VarType::Int.value("12", cp), Ok(Value::Int(12)));
        assert!(VarType::Int.value("1.5", cp).is_err());
        assert!(VarType::Object.value("0", cp).is_err(), "not set from text");
        assert_eq!(VarType::of_text("2"), VarType::Int);
        assert_eq!(VarType::of_text("2.5"), VarType::Float);
        assert_eq!(VarType::of_text("two"), VarType::String);
    }

    /// A value typed with no type is a float only where it is digits with
    /// a point (the review's V24: `nan`, `inf` and `infinity` were stored
    /// as FLOAT NaN and infinity, and `1e5` as a float, where a builder
    /// meant the words).
    #[test]
    fn a_word_rust_reads_as_a_number_is_a_string() {
        for word in ["nan", "NaN", "inf", "-inf", "infinity", "+Infinity", "1e5", "1.5e3", "."] {
            assert_eq!(VarType::of_text(word), VarType::String, "{word}");
        }
        for float in ["2.5", " -0.25 ", "+3.", "-.5", "100.0"] {
            assert_eq!(VarType::of_text(float), VarType::Float, "{float}");
            assert!(VarType::Float.value(float.trim(), Codepage::WINDOWS_1252).is_ok(), "{float}");
        }
        for int in ["0", "-7", "+7"] {
            assert_eq!(VarType::of_text(int), VarType::Int, "{int}");
        }
        // (A number too large for an int is no float either: it has no point.)
        assert_eq!(VarType::of_text("99999999999"), VarType::String);
    }
}
