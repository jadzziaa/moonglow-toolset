//! Core types shared by every Moonglow crate: resource names ([`ResRef`]) and
//! types ([`ResType`]), languages and codepages, talk-table references
//! ([`StrRef`]), localized strings ([`LocString`]) and little-endian binary
//! readers and writers for the game's file formats, and the order names are
//! listed in ([`text`]), and dates from the clock ([`time`]).

pub mod bin;
pub mod lang;
pub mod locstring;
pub mod release;
pub mod resref;
pub mod restype;
pub mod sha1;
pub mod sha256;
pub mod strref;
pub mod text;
pub mod time;

pub use lang::{Codepage, Gender, Language};
pub use locstring::{LocString, LocStringKey};
pub use resref::{ResRef, ResRefError};
pub use restype::ResType;
pub use strref::StrRef;
