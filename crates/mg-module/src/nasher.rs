//! nasher projects: a module kept as a source tree of text files for version
//! control (<https://github.com/squattingmonk/nasher>). GFF resources are
//! JSON files in neverwinter.nim's format (`module.ifo.json`), scripts are
//! `.nss`, anything else is the file itself; `nasher.cfg` names the targets
//! (the files packed from the tree) and which files each one takes.
//!
//! Moonglow writes a resource's source file exactly as `nasher unpack` would
//! (see `docs/research/notes_nasher.md`), so saving a module changes only the
//! files of the resources that changed.

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
        let json: serde_json::Value =
            serde_json::from_slice(text).map_err(|e| bad(e.to_string()))?;
        from_json(&json, self.codepage).map_err(|e| bad(e.to_string()))
    }
}
