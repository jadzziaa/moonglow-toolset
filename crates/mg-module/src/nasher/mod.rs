//! nasher projects: a module kept as a source tree of text files for version
//! control (<https://github.com/squattingmonk/nasher>). GFF resources are
//! JSON files in neverwinter.nim's format (`module.ifo.json`), or NWNT
//! files where the project's `gffFormat` is `nwnt` (`module.ifo.nwnt`,
//! WilliamDraco's `nwn_nwnt`: [`mg_gff::to_nwnt`]), scripts are
//! `.nss`, anything else is the file itself; `nasher.cfg` names the targets
//! (the files packed from the tree) and which files each one takes.
//!
//! Moonglow writes a resource's source file exactly as `nasher unpack` would
//! (see `docs/research/notes_nasher.md`), so saving a module changes only the
//! files of the resources that changed.

mod cfg;
pub(crate) mod glob;
mod project;
mod tlk;

pub use cfg::{Package, Section, Settings, Target, sections};
pub(crate) use project::scan;
pub use project::{Outside, Project, Resources, SaveReport};
pub use tlk::{
    TalkTable, format_of, in_language, kept_as, pack_talk_table, read_talk_table, talk_tables,
    write_talk_table,
};

use mg_core::{Codepage, ResType};
use mg_gff::{Gff, TextStyle, from_json, from_nwnt, to_json, to_json_text, to_nwnt};

use crate::ModuleError;

/// The resource types nasher keeps as JSON (neverwinter.nim's
/// `GffExtensions`).
pub const GFF_EXTENSIONS: [&str; 20] = [
    "utc", "utd", "ute", "uti", "utm", "utp", "uts", "utt", "utw", "git", "are", "gic", "ifo",
    "fac", "dlg", "itp", "bic", "jrl", "gff", "gui",
];

/// Whether nasher keeps resources of this type as JSON.
pub fn is_json_type(restype: ResType) -> bool {
    restype.extension().is_some_and(|e| GFF_EXTENSIONS.contains(&e))
}

/// How a resource's source file keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Form {
    /// The resource itself (a script, a 2DA, a picture).
    #[default]
    Plain,
    /// A GFF as neverwinter.nim's JSON (`x.utc.json`).
    Json,
    /// A GFF as NWNT (`x.utc.nwnt`).
    Nwnt,
}

impl Form {
    /// The form of a source file by its name, and the name of the resource
    /// it keeps (`x.utc.json` keeps `x.utc` as JSON).
    pub fn of(file_name: &str) -> (&str, Form) {
        let lower = file_name.to_ascii_lowercase();
        let cut = |n: usize| &file_name[..file_name.len() - n];
        if lower.ends_with(".json") {
            (cut(5), Form::Json)
        } else if lower.ends_with(".nwnt") {
            (cut(5), Form::Nwnt)
        } else {
            (file_name, Form::Plain)
        }
    }

    /// Whether the file is a GFF kept as text, not the resource itself.
    pub fn is_text(self) -> bool {
        self != Form::Plain
    }

    /// The extension a source file of this form adds to its resource's
    /// name.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Form::Plain => None,
            Form::Json => Some("json"),
            Form::Nwnt => Some("nwnt"),
        }
    }

    /// The form of a GFF kept in a project whose `gffFormat` is `format`.
    pub fn for_gff_format(format: &str) -> Option<Form> {
        match format {
            "json" => Some(Form::Json),
            "nwnt" => Some(Form::Nwnt),
            _ => None,
        }
    }
}

/// How a project's GFF files are converted (nasher's `truncateFloats` and
/// the codepage `nwn_gff` reads text in, windows-1252 unless its
/// `--nwn-encoding` says otherwise), and the form a GFF new to the project
/// is kept in (its `gffFormat`; a file already there keeps its own).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conversion {
    pub codepage: Codepage,
    pub float_places: u8,
    pub gff: Form,
}

impl Default for Conversion {
    fn default() -> Self {
        Conversion { codepage: Codepage::WINDOWS_1252, float_places: 4, gff: Form::Json }
    }
}

impl Conversion {
    /// A project's conversion, by its settings.
    pub fn of(settings: &Settings) -> Conversion {
        Conversion {
            codepage: settings.codepage,
            float_places: settings.truncate_floats,
            gff: Form::for_gff_format(&settings.gff_format).unwrap_or(Form::Json),
        }
    }

    /// A GFF resource's source text in `form`, as `nasher unpack` writes it
    /// ([`Conversion::to_source`], [`Conversion::to_nwnt_source`]); the
    /// resource itself for [`Form::Plain`].
    pub fn to_text(
        &self,
        form: Form,
        restype: ResType,
        resource: &[u8],
        gff: Option<&Gff>,
    ) -> Result<Vec<u8>, ModuleError> {
        let read;
        let gff = match (form, gff) {
            (Form::Plain, _) => return Ok(resource.to_vec()),
            (_, Some(gff)) => gff,
            (_, None) => {
                read = Gff::read(resource).map_err(|e| ModuleError::Source {
                    name: format!("{restype:?}"),
                    message: e.to_string(),
                })?;
                &read
            }
        };
        Ok(match form {
            Form::Nwnt => self.to_nwnt_source(restype, gff)?,
            _ => self.to_source(restype, gff)?,
        }
        .into_bytes())
    }

    /// A GFF resource's NWNT, as `nasher unpack` writes it with `gffFormat
    /// nwnt` (`nwn_nwnt -p truncateFloats`): without the module's `Mod_ID`
    /// and an area's `Version`, as its JSON. A file with a field NWNT can't
    /// keep (a label with a dot in it) is refused.
    pub fn to_nwnt_source(&self, restype: ResType, gff: &Gff) -> Result<String, ModuleError> {
        if let Some(path) = mg_gff::nwnt_unkept_label(&gff.root) {
            return Err(ModuleError::Source {
                name: format!("{restype:?}"),
                message: format!(
                    "{path}: NWNT can't keep a label with a dot in it; keep the project's GFF \
                     files as JSON (gffFormat json)"
                ),
            });
        }
        let label = match restype {
            ResType::IFO => Some("Mod_ID"),
            ResType::ARE => Some("Version"),
            _ => None,
        };
        match label.filter(|l| gff.root.contains(l)) {
            Some(label) => {
                let mut without = gff.clone();
                without.root.fields.retain(|f| f.label.as_bytes() != label.as_bytes());
                Ok(to_nwnt(&without, self.codepage, self.float_places))
            }
            None => Ok(to_nwnt(gff, self.codepage, self.float_places)),
        }
    }

    /// A GFF resource's source text, as `nasher unpack` writes it: without
    /// the module's `Mod_ID` and an area's `Version` (nasher drops both; the
    /// version changes at every save), keys sorted, floats rounded.
    pub fn to_source(&self, restype: ResType, gff: &Gff) -> Result<String, ModuleError> {
        let mut json = to_json(gff, self.codepage).map_err(|e| ModuleError::Source {
            name: format!("{restype:?}"),
            message: e.to_string(),
        })?;
        if let Some(obj) = json.as_object_mut() {
            match restype {
                ResType::IFO => _ = obj.shift_remove("Mod_ID"),
                ResType::ARE => _ = obj.shift_remove("Version"),
                _ => {}
            }
        }
        let style = TextStyle { float_places: Some(self.float_places), ..TextStyle::NASHER };
        Ok(to_json_text(&json, style))
    }

    /// A GFF resource from its source text: NWNT where `name` ends in
    /// `.nwnt`, else JSON.
    pub fn from_source(&self, name: &str, text: &[u8]) -> Result<Gff, ModuleError> {
        let form = if Form::of(name).1 == Form::Nwnt { Form::Nwnt } else { Form::Json };
        self.from_text(form, name, text)
    }

    /// A GFF resource from its source text in `form` (JSON for
    /// [`Form::Plain`]).
    pub fn from_text(&self, form: Form, name: &str, text: &[u8]) -> Result<Gff, ModuleError> {
        let bad = |message: String| ModuleError::Source { name: name.to_string(), message };
        if form == Form::Nwnt {
            // (Stray bytes that aren't UTF-8 are read as the project's code
            // page has them, as in JSON below.)
            let text = match std::str::from_utf8(text) {
                Ok(t) => std::borrow::Cow::Borrowed(t),
                Err(_) => std::borrow::Cow::Owned(self.mended(text)),
            };
            return from_nwnt(&text, self.codepage).map_err(|e| bad(e.to_string()));
        }
        let json: serde_json::Value = match serde_json::from_slice(text) {
            Ok(json) => json,
            // A file that isn't UTF-8 throughout (text another tool wrote
            // out as the game's bytes, in a Polish or Russian module): the
            // stray bytes are read as the project's code page has them,
            // which is what they were, and the rest as it is.
            Err(first) if std::str::from_utf8(text).is_err() => {
                serde_json::from_str(&self.mended(text)).map_err(|_| bad(first.to_string()))?
            }
            Err(e) => return Err(bad(e.to_string())),
        };
        from_json(&json, self.codepage).map_err(|e| bad(e.to_string()))
    }

    /// Text that isn't UTF-8 throughout, its stray bytes read in the
    /// project's code page.
    fn mended(&self, text: &[u8]) -> String {
        let mut mended = String::with_capacity(text.len());
        for chunk in text.utf8_chunks() {
            mended.push_str(chunk.valid());
            mended.push_str(&self.codepage.decode(chunk.invalid()));
        }
        mended
    }
}

#[cfg(test)]
mod source_tests {
    use super::*;

    /// A source file with bytes that aren't UTF-8 inside a string (text
    /// another tool wrote out as the game's own bytes) opens: the stray
    /// bytes are read as the code page has them.
    #[test]
    fn a_source_that_is_not_utf_8_throughout_is_read() {
        let conv = Conversion::default();
        let mut text =
            b"{\"__data_type\": \"UTI \", \"Tag\": {\"type\": \"cexostring\", \"value\": \"za"
                .to_vec();
        text.push(0xB3); // (ł in windows-1250; a lone byte is no UTF-8.)
        text.extend_from_slice(b"\"}}");
        assert!(std::str::from_utf8(&text).is_err());
        let gff = conv.from_source("x.uti.json", &text).expect("read");
        assert_eq!(gff.root.string("Tag"), Some(&b"za\xB3"[..]), "the byte it was");
        // What is no JSON at all still says so.
        assert!(conv.from_source("x.uti.json", b"{\"a\": \xB3}").is_err());
    }
}
