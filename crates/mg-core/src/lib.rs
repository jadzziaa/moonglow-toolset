//! Core types shared by every Moonglow crate: resource names ([`ResRef`]) and
//! types ([`ResType`]), languages and codepages, talk-table references
//! ([`StrRef`]), localized strings ([`LocString`]) and little-endian binary
//! readers and writers for the game's file formats.

pub mod bin;
pub mod lang;
pub mod locstring;
pub mod resref;
pub mod restype;
pub mod strref;

pub use lang::{Codepage, Gender, Language};
pub use locstring::{LocString, LocStringKey};
pub use resref::{ResRef, ResRefError};
pub use restype::ResType;
pub use strref::StrRef;
