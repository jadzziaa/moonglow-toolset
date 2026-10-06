//! nasher projects: a module kept as a source tree of text files for version
//! control (<https://github.com/squattingmonk/nasher>). GFF resources are
//! JSON files in neverwinter.nim's format (`module.ifo.json`), scripts are
//! `.nss`, anything else is the file itself; `nasher.cfg` names the targets
//! (the files packed from the tree) and which files each one takes.
//!
//! Moonglow writes a resource's source file exactly as `nasher unpack` would
//! (see `docs/research/notes_nasher.md`), so saving a module changes only the
//! files of the resources that changed.

mod cfg;
mod glob;
mod project;

pub use cfg::{Package, Section, Settings, Target, sections};
pub use project::{Outside, Project, Resources, SaveReport};

use mg_core::{Codepage, ResType};
use mg_gff::{Gff, TextStyle, from_json, to_json, to_json_text};

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

/// How a project's GFF files are converted (nasher's `truncateFloats` and
/// the codepage `nwn_gff` reads text in, windows-1252 unless its
/// `--nwn-encoding` says otherwise).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Conversion {
    pub codepage: Codepage,
    pub float_places: u8,
}

impl Default for Conversion {
    fn default() -> Self {
        Conversion { codepage: Codepage::WINDOWS_1252, float_places: 4 }
    }
}

impl Conversion {
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

    /// A GFF resource from its source text.
    pub fn from_source(&self, name: &str, text: &[u8]) -> Result<Gff, ModuleError> {
        let bad = |message: String| ModuleError::Source { name: name.to_string(), message };
        let json: serde_json::Value = match serde_json::from_slice(text) {
            Ok(json) => json,
            // A file that isn't UTF-8 throughout (text another tool wrote
            // out as the game's bytes, in a Polish or Russian module): the
            // stray bytes are read as the project's code page has them,
            // which is what they were, and the rest as it is.
            Err(first) if std::str::from_utf8(text).is_err() => {
                let mut mended = String::with_capacity(text.len());
                for chunk in text.utf8_chunks() {
                    mended.push_str(chunk.valid());
                    mended.push_str(&self.codepage.decode(chunk.invalid()));
                }
                serde_json::from_str(&mended).map_err(|_| bad(first.to_string()))?
            }
            Err(e) => return Err(bad(e.to_string())),
        };
        from_json(&json, self.codepage).map_err(|e| bad(e.to_string()))
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
