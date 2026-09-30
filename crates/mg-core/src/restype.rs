//! Resource types: the numeric ids that KEY, BIF and ERF files store next to
//! each resource name, and the file extensions they map to.

use std::fmt;

/// A resource type id. Unknown ids are kept as they are, so containers with
/// types this table does not list still round-trip.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResType(pub u16);

macro_rules! restypes {
    ($($name:ident = $id:literal, $ext:literal;)*) => {
        impl ResType {
            $(pub const $name: ResType = ResType($id);)*

            /// Every type in the table, in id order.
            pub const ALL: &'static [ResType] = &[$(ResType::$name),*];

            /// The file extension (lowercase, without the dot), if the id is known.
            pub fn extension(self) -> Option<&'static str> {
                match self.0 {
                    $($id => Some($ext),)*
                    _ => None,
                }
            }

            /// The type for a file extension (any case, without the dot).
            pub fn from_extension(ext: &str) -> Option<ResType> {
                let ext = ext.to_ascii_lowercase();
                match ext.as_str() {
                    $($ext => Some(ResType::$name),)*
                    _ => None,
                }
            }
        }
    };
}

// Ids from BioWare's KEY/BIF documentation plus the EE additions, matching
// neverwinter.nim and nwn.py. `INVALID` (0xFFFF) marks unused container slots.
restypes! {
    RES = 0, "res";
    BMP = 1, "bmp";
    MVE = 2, "mve";
    TGA = 3, "tga";
    WAV = 4, "wav";
    WFX = 5, "wfx";
    PLT = 6, "plt";
    INI = 7, "ini";
    BMU = 8, "bmu";
    MPG = 9, "mpg";
    TXT = 10, "txt";
    PLH = 2000, "plh";
    TEX = 2001, "tex";
    MDL = 2002, "mdl";
    THG = 2003, "thg";
    FNT = 2005, "fnt";
    LUA = 2007, "lua";
    SLT = 2008, "slt";
    NSS = 2009, "nss";
    NCS = 2010, "ncs";
    MOD = 2011, "mod";
    ARE = 2012, "are";
    SET = 2013, "set";
    IFO = 2014, "ifo";
    BIC = 2015, "bic";
    WOK = 2016, "wok";
    TWODA = 2017, "2da";
    TLK = 2018, "tlk";
    TXI = 2022, "txi";
    GIT = 2023, "git";
    BTI = 2024, "bti";
    UTI = 2025, "uti";
    BTC = 2026, "btc";
    UTC = 2027, "utc";
    DLG = 2029, "dlg";
    ITP = 2030, "itp";
    BTT = 2031, "btt";
    UTT = 2032, "utt";
    DDS = 2033, "dds";
    BTS = 2034, "bts";
    UTS = 2035, "uts";
    LTR = 2036, "ltr";
    GFF = 2037, "gff";
    FAC = 2038, "fac";
    BTE = 2039, "bte";
    UTE = 2040, "ute";
    BTD = 2041, "btd";
    UTD = 2042, "utd";
    BTP = 2043, "btp";
    UTP = 2044, "utp";
    DFT = 2045, "dft";
    GIC = 2046, "gic";
    GUI = 2047, "gui";
    CSS = 2048, "css";
    CCS = 2049, "ccs";
    BTM = 2050, "btm";
    UTM = 2051, "utm";
    DWK = 2052, "dwk";
    PWK = 2053, "pwk";
    BTG = 2054, "btg";
    UTG = 2055, "utg";
    JRL = 2056, "jrl";
    SAV = 2057, "sav";
    UTW = 2058, "utw";
    FOURPC = 2059, "4pc";
    SSF = 2060, "ssf";
    HAK = 2061, "hak";
    NWM = 2062, "nwm";
    BIK = 2063, "bik";
    NDB = 2064, "ndb";
    PTM = 2065, "ptm";
    PTT = 2066, "ptt";
    BAK = 2067, "bak";
    DAT = 2068, "dat";
    SHD = 2069, "shd";
    XBC = 2070, "xbc";
    WBM = 2071, "wbm";
    MTR = 2072, "mtr";
    KTX = 2073, "ktx";
    TTF = 2074, "ttf";
    SQL = 2075, "sql";
    TML = 2076, "tml";
    SQ3 = 2077, "sq3";
    LOD = 2078, "lod";
    GIF = 2079, "gif";
    PNG = 2080, "png";
    JPG = 2081, "jpg";
    CAF = 2082, "caf";
    JUI = 2083, "jui";
    IDS = 9996, "ids";
    ERF = 9997, "erf";
    BIF = 9998, "bif";
    KEY = 9999, "key";
}

impl ResType {
    /// Marks an unused slot in KEY and ERF tables.
    pub const INVALID: ResType = ResType(0xFFFF);

    /// Whether the resource is stored in the GFF container format.
    pub fn is_gff(self) -> bool {
        matches!(
            self,
            ResType::ARE
                | ResType::IFO
                | ResType::BIC
                | ResType::GIT
                | ResType::UTI
                | ResType::UTC
                | ResType::DLG
                | ResType::ITP
                | ResType::UTT
                | ResType::UTS
                | ResType::GFF
                | ResType::FAC
                | ResType::UTE
                | ResType::UTD
                | ResType::UTP
                | ResType::GIC
                | ResType::GUI
                | ResType::UTM
                | ResType::JRL
                | ResType::UTW
                | ResType::PTM
                | ResType::PTT
        )
    }
}

impl fmt::Debug for ResType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension() {
            Some(ext) => write!(f, "ResType({}={ext})", self.0),
            None => write!(f, "ResType({})", self.0),
        }
    }
}

impl fmt::Display for ResType {
    /// The extension, or `#id` for unknown types.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.extension() {
            Some(ext) => f.write_str(ext),
            None => write!(f, "#{}", self.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_round_trip() {
        for &t in ResType::ALL {
            let ext = t.extension().unwrap();
            assert_eq!(ResType::from_extension(ext), Some(t), "{ext}");
            assert_eq!(ResType::from_extension(&ext.to_ascii_uppercase()), Some(t));
        }
    }

    #[test]
    fn table_is_sorted_and_unique() {
        assert!(ResType::ALL.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn well_known_ids() {
        assert_eq!(ResType::TWODA.0, 2017);
        assert_eq!(ResType::from_extension("utc"), Some(ResType(2027)));
        assert_eq!(ResType(1234).extension(), None);
        assert_eq!(ResType(1234).to_string(), "#1234");
        assert!(ResType::UTC.is_gff());
        assert!(!ResType::MDL.is_gff());
    }
}
