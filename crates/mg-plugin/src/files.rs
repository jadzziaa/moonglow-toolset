//! What a plugin reaches outside the module, each with the user's leave:
//! a file or a folder of theirs to read (they choose it), and haks of
//! their hak folder to put resources into (they agree to each); and
//! pictures decoded for it.
//!
//! The plugin's code never opens a file: the host asks the user, and the
//! API reads what they chose (nothing above a chosen folder) and hands
//! over bytes. What goes into a hak is handed back with the job's edits
//! for the host to write.

use std::path::{Component, Path, PathBuf};
use std::rc::Rc;

use mg_core::ResType;
use mg_edit::{Edit, GffPath};
use mlua::{Lua, Table, Value as LuaValue};

use crate::runtime::{Shared, This, codepage, fail};
use crate::{Answer, HakData, HakWrite, MAX_FILE, Question};

/// The names of what these functions hand out, as the reference writes
/// them.
pub(crate) const NAMES: [&str; 11] = [
    "folder:write",
    "file.name",
    "file.bytes",
    "folder.name",
    "folder:files",
    "folder:bytes",
    "folder:text",
    "folder:to_hak",
    "image.width",
    "image.height",
    "image:pixel",
];

/// The most files of a folder that are listed: a folder with more is
/// not listed at all.
const MAX_LISTED: usize = 20_000;
/// The most pixels a picture may have (a small file can unpack to a
/// great many).
const MAX_PIXELS: u64 = 1 << 26;

/// A file's bytes, if it is not too large for a plugin.
pub(crate) fn read(path: &Path) -> mlua::Result<Vec<u8>> {
    let shown = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let size = std::fs::metadata(path).or_else(|e| fail(format!("{shown}: {e}")))?.len();
    if size > MAX_FILE {
        return fail(format!(
            "{shown} has {} MB; a plugin reads files of up to {}",
            size >> 20,
            MAX_FILE >> 20
        ));
    }
    std::fs::read(path).or_else(|e| fail(format!("{shown}: {e}")))
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The files in `dir` and the folders in it, by their paths from `dir`
/// with `/`, sorted. Links are not followed out of it. A folder of more
/// than `most` files is an error, told as soon as it is known (a part of
/// them, whichever the system gave first, would be taken for them all),
/// and the walk ends between two folders when `stopped` says so.
fn listing(dir: &Path, most: usize, stopped: &dyn Fn() -> bool) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(sub) = todo.pop() {
        if stopped() {
            return Err("canceled".into());
        }
        let Ok(entries) = std::fs::read_dir(dir.join(&sub)) else { continue };
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            let path = sub.join(e.file_name());
            if kind.is_dir() {
                todo.push(path);
            } else if kind.is_file() {
                if out.len() == most {
                    return Err(format!(
                        "the folder {} has more than {most} files: choose a folder within it",
                        name_of(dir)
                    ));
                }
                let parts: Vec<String> = path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The file `name` (a path with `/`) of folder `dir`, and nothing outside
/// it.
fn inside(dir: &Path, name: &str) -> mlua::Result<PathBuf> {
    let given = Path::new(name);
    let plain = !name.is_empty()
        && !name.contains('\\')
        && given.components().all(|c| matches!(c, Component::Normal(_)));
    if !plain {
        return fail(format!("{name:?} is not a file of the folder"));
    }
    let path = dir.join(given);
    // (A link in the folder may lead out of it.)
    let (Ok(real), Ok(root)) = (path.canonicalize(), dir.canonicalize()) else {
        return fail(format!("{name}: no such file in the folder"));
    };
    if !real.starts_with(&root) || !real.is_file() {
        return fail(format!("{name:?} is not a file of the folder"));
    }
    Ok(real)
}

/// A question's `title` and what else its table has.
fn asked(options: &Option<Table>) -> mlua::Result<(String, Vec<String>)> {
    let Some(t) = options else { return Ok((String::new(), Vec::new())) };
    let title = t.get::<Option<String>>("title")?.unwrap_or_default();
    let extensions = t
        .get::<Option<Vec<String>>>("extensions")?
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();
    Ok((title, extensions))
}

/// `ctx.ui:open_file` and `ctx.ui:open_folder`.
pub(crate) fn questions(lua: &Lua, sh: &Rc<Shared>, ui: &Table) -> mlua::Result<()> {
    let s = sh.clone();
    ui.set(
        "open_file",
        lua.create_function(move |lua, (_, options): (This, Option<Table>)| {
            let (title, extensions) = asked(&options)?;
            let Some(Answer::Path(path)) = s.host.ask(&Question::File { title, extensions }) else {
                return Ok(LuaValue::Nil);
            };
            Ok(LuaValue::Table(file_table(lua, &path)?))
        })?,
    )?;
    let s = sh.clone();
    ui.set(
        "open_folder",
        lua.create_function(move |lua, (_, options): (This, Option<Table>)| {
            let (title, _) = asked(&options)?;
            let Some(Answer::Path(dir)) = s.host.ask(&Question::Folder { title }) else {
                return Ok(LuaValue::Nil);
            };
            Ok(LuaValue::Table(folder_table(lua, &s, dir)?))
        })?,
    )?;
    Ok(())
}

/// A file of the user's, as a plugin gets one: its name and bytes.
pub(crate) fn file_table(lua: &Lua, path: &Path) -> mlua::Result<Table> {
    let file = lua.create_table()?;
    file.set("name", name_of(path))?;
    file.set("bytes", lua.create_string(read(path)?)?)?;
    Ok(file)
}

/// A folder of the user's, as a plugin gets one: its files to read, and
/// to write into with their leave.
pub(crate) fn folder_table(lua: &Lua, s: &Rc<Shared>, dir: PathBuf) -> mlua::Result<Table> {
    if !dir.is_dir() {
        return fail(format!("{} is not a folder", dir.display()));
    }
    {
        let folder = lua.create_table()?;
        folder.set("name", name_of(&dir))?;
        let (d, sh) = (dir.clone(), s.clone());
        folder.set(
            "files",
            lua.create_function(move |_, _: This| {
                listing(&d, MAX_LISTED, &|| sh.stopped()).or_else(fail)
            })?,
        )?;
        let d = dir.clone();
        folder.set(
            "bytes",
            lua.create_function(move |lua, (_, name): (This, String)| {
                lua.create_string(read(&inside(&d, &name)?)?)
            })?,
        )?;
        let (d, sh) = (dir.clone(), s.clone());
        folder.set(
            "to_hak",
            lua.create_function(
                move |_, (_, hak, file, name): (This, String, String, Option<String>)| {
                    let path = inside(&d, &file)?;
                    let name = name.unwrap_or_else(|| name_of(&path));
                    put(&sh, &hak, &name, HakData::File(path))
                },
            )?,
        )?;
        let (d, sh) = (dir.clone(), s.clone());
        folder.set(
            "write",
            lua.create_function(move |_, (_, name, bytes): (This, String, mlua::LuaString)| {
                write(&sh, &d, &name, bytes.as_bytes().to_vec())
            })?,
        )?;
        folder.set(
            "text",
            lua.create_function(move |lua, (_, name): (This, String)| {
                let bytes = read(&inside(&dir, &name)?)?;
                // UTF-8 if it is, else as the module's codepage has it.
                match String::from_utf8(bytes) {
                    Ok(text) => lua.create_string(text.trim_start_matches('\u{feff}')),
                    Err(e) => lua.create_string(codepage().decode(e.as_bytes()).as_bytes()),
                }
            })?,
        )?;
        Ok(folder)
    }
}

/// A file for a folder the user chose, with their leave for the first one
/// of each folder: written when the handler has returned.
fn write(s: &Shared, dir: &Path, name: &str, bytes: Vec<u8>) -> mlua::Result<()> {
    if !crate::is_file_name(name) {
        return fail(format!(
            "{name:?} is not a file's name (a name alone: no folders, nothing hidden)"
        ));
    }
    if bytes.len() as u64 > MAX_FILE {
        return fail(format!("{name} is too large to write"));
    }
    // (A link in the folder may lead out of it.)
    let path = dir.join(name);
    if path.symlink_metadata().is_ok() {
        let (Ok(real), Ok(root)) = (path.canonicalize(), dir.canonicalize()) else {
            return fail(format!("{name} is not a file of the folder"));
        };
        if real.parent() != Some(&root) || !real.is_file() {
            return fail(format!("{name} is not a file of the folder"));
        }
    }
    let known = s.folders.borrow().iter().any(|f| f.dir == dir);
    if !known {
        let text = format!(
            "{} wants to write files into the folder {} (those of the same names are \
             replaced). Allow it?",
            s.who,
            dir.display()
        );
        if s.host.ask(&Question::Confirm(text)) != Some(Answer::Yes) {
            return fail(format!("writing into {} was not allowed", dir.display()));
        }
        s.folders
            .borrow_mut()
            .push(crate::FolderWrite { dir: dir.to_path_buf(), files: Vec::new() });
    }
    s.budget_held.keep(bytes.len())?;
    let mut folders = s.folders.borrow_mut();
    let files = &mut folders.iter_mut().find(|f| f.dir == dir).expect("added").files;
    match files.iter_mut().find(|(n, _)| n == name) {
        Some(old) => {
            s.budget_held.give_back(old.1.len());
            old.1 = bytes;
        }
        None => files.push((name.to_string(), bytes)),
    }
    Ok(())
}

/// A hak's name as a plugin may give it: what a file of the hak folder
/// can be called, and nothing that leads elsewhere.
fn hak_name(name: &str) -> mlua::Result<String> {
    let name = name.strip_suffix(".hak").unwrap_or(name);
    let fine = !name.is_empty()
        && name.len() <= 32
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    if !fine {
        return fail(format!(
            "{name:?} is not a hak's name (letters, digits, _ and -, at most 32)"
        ));
    }
    Ok(name.to_ascii_lowercase())
}

/// A resource for a hak of the user's hak folder, with their leave for
/// the first one of each hak.
fn put(s: &Shared, hak: &str, name: &str, data: HakData) -> mlua::Result<()> {
    let hak = hak_name(hak)?;
    let key = mg_module::hak_edit::key_for(name).or_else(|why| fail(format!("{name}: {why}")))?;
    let known = s.haks.borrow().iter().any(|h| h.name == hak);
    if !known {
        let text = format!(
            "{} wants to put resources into the hak {hak}.hak in your hak folder (added to it \
             if it is there; a copy of it is kept as {hak}.hak.bak). Allow it?",
            s.who
        );
        if s.host.ask(&Question::Confirm(text)) != Some(Answer::Yes) {
            return fail(format!("writing the hak {hak} was not allowed"));
        }
        s.haks.borrow_mut().push(HakWrite { name: hak.clone(), files: Vec::new(), held: None });
    }
    // (Past what a job holds in memory, its bytes wait in a scratch file.)
    let mut held = None;
    let data = match data {
        HakData::Bytes(bytes) => {
            let total = s.held_bytes.get() + bytes.len();
            if total > crate::HELD_IN_MEMORY {
                let scratch = s.held.borrow_mut().get_or_insert_with(Default::default).clone();
                let path = scratch.keep(&key.to_string(), &bytes).or_else(fail)?;
                held = Some(scratch);
                HakData::File(path)
            } else {
                s.held_bytes.set(total);
                HakData::Bytes(bytes)
            }
        }
        other => other,
    };
    let mut haks = s.haks.borrow_mut();
    let entry = haks.iter_mut().find(|h| h.name == hak).expect("added");
    if held.is_some() {
        entry.held = held;
    }
    let files = &mut entry.files;
    match files.iter_mut().find(|(k, _)| *k == key) {
        Some(old) => old.1 = data,
        None => files.push((key, data)),
    }
    Ok(())
}

/// `ctx.hak`: resources for a hak of the user's hak folder, and the
/// module's list of haks.
pub(crate) fn hak(lua: &Lua, sh: &Rc<Shared>) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let s = sh.clone();
    t.set(
        "write",
        lua.create_function(
            move |_, (_, hak, name, bytes): (This, String, String, mlua::LuaString)| {
                put(&s, &hak, &name, HakData::Bytes(bytes.as_bytes().to_vec().into()))
            },
        )?,
    )?;
    let s = sh.clone();
    t.set(
        "attach",
        lua.create_function(move |_, (_, hak): (This, String)| {
            let hak = hak_name(&hak)?;
            let key = mg_module::Module::info_key();
            let list = {
                let mut ws = s.ws.borrow_mut();
                let info = ws.doc(&key).or_else(fail)?;
                mg_module::attach::hak_list(&info.root, &[hak])
            };
            s.apply(Edit::SetField {
                key,
                path: GffPath::root(),
                label: "Mod_HakList".into(),
                value: Some(list),
            })
        })?,
    )?;
    Ok(t)
}

/// What kind of picture bytes are: the kind given, else by how they
/// begin.
fn kind_of(bytes: &[u8], kind: Option<&str>) -> Result<&'static str, String> {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    Ok(match kind.map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "png",
        Some("tga") => "tga",
        Some("dds") => "dds",
        Some(other) => return Err(format!("{other:?} is not a kind of picture (png, tga, dds)")),
        None if bytes.starts_with(PNG) => "png",
        None if bytes.starts_with(b"DDS ") => "dds",
        None => "tga",
    })
}

/// The width and height a picture's header declares, read before any of
/// it is decoded (a few bytes can declare a great many pixels): refused
/// past [`MAX_PIXELS`]. A header cut short is `(0, 0)`: decoding it says
/// what is wrong with it.
fn size_of(bytes: &[u8], kind: Option<&str>) -> Result<(u32, u32), String> {
    let le16 =
        |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
    let four = |at: usize| bytes.get(at..at + 4).map(|b| [b[0], b[1], b[2], b[3]]);
    let le32 = |at: usize| four(at).map(u32::from_le_bytes);
    let be32 = |at: usize| four(at).map(u32::from_be_bytes);
    let size = match kind_of(bytes, kind)? {
        // (After its signature: the IHDR chunk's length and name.)
        "png" => (be32(16), be32(20)),
        // (The standard header has its height first; BioWare's own, its
        // width.)
        "dds" if bytes.starts_with(b"DDS ") => (le32(16), le32(12)),
        "dds" => (le32(0), le32(4)),
        _ => (le16(12), le16(14)),
    };
    let (Some(width), Some(height)) = size else { return Ok((0, 0)) };
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("the picture is too large".into());
    }
    Ok((width, height))
}

/// A picture's pixels, rows from the top.
fn decode(bytes: &[u8], kind: Option<&str>) -> Result<(u32, u32, Vec<u8>), String> {
    size_of(bytes, kind)?;
    let kind = kind_of(bytes, kind)?;
    if kind != "png" {
        let rgba = match kind {
            "dds" => mg_image::read(ResType::DDS, bytes).map(|t| t.to_rgba().top_down()),
            // (By the origin its header declares, as a NUI window's.)
            _ => mg_image::tga::read_picture(bytes),
        };
        let rgba = rgba.map_err(|e| e.to_string())?;
        if u64::from(rgba.width) * u64::from(rgba.height) > MAX_PIXELS {
            return Err("the picture is too large".into());
        }
        return Ok((rgba.width, rgba.height, rgba.data));
    }
    let rgba = mg_image::png::read_picture(bytes, MAX_PIXELS).map_err(|e| match e {
        mg_image::ImageError::BadSize(..) => "the picture is too large".to_string(),
        e => e.to_string(),
    })?;
    Ok((rgba.width, rgba.height, rgba.data))
}

/// `mg.image(bytes, kind?)`: a PNG, TGA or DDS picture decoded.
pub(crate) fn image(
    lua: &Lua,
    held: &Rc<crate::runtime::Budget>,
    (bytes, kind): (mlua::LuaString, Option<String>),
) -> mlua::Result<Table> {
    let (width, height) =
        size_of(&bytes.as_bytes(), kind.as_deref()).or_else(|e| fail(format!("mg.image: {e}")))?;
    // (Its pixels are the host's for as long as the plugin has it.)
    let charge = held.hold(width as usize * height as usize * 4)?;
    let (width, height, data) =
        decode(&bytes.as_bytes(), kind.as_deref()).or_else(|e| fail(format!("mg.image: {e}")))?;
    let t = lua.create_table()?;
    t.set("width", width)?;
    t.set("height", height)?;
    t.set(
        "pixel",
        lua.create_function(move |_, (_, x, y): (This, f64, f64)| {
            let _held = &charge;
            // (Said so that what is not a number is outside it too.)
            let inside = x >= 0.0 && y >= 0.0 && x < f64::from(width) && y < f64::from(height);
            if !inside {
                return fail(format!("the picture has no pixel ({x}, {y})"));
            }
            let i = (y as usize * width as usize + x as usize) * 4;
            Ok((data[i], data[i + 1], data[i + 2], data[i + 3]))
        })?,
    )?;
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_s_files_stay_inside_it() {
        let dir = std::env::temp_dir().join(format!("mg-plugin-inside-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("in/sub")).unwrap();
        std::fs::write(dir.join("in/a.txt"), "a").unwrap();
        std::fs::write(dir.join("in/sub/b.txt"), "b").unwrap();
        std::fs::write(dir.join("secret.txt"), "s").unwrap();
        let root = dir.join("in");
        assert_eq!(listing(&root, 2, &|| false).unwrap(), ["a.txt", "sub/b.txt"]);
        // One file more than are listed: an error, not a list of some.
        let e = listing(&root, 1, &|| false).unwrap_err();
        assert_eq!(e, "the folder in has more than 1 files: choose a folder within it");
        // And a job called off stops walking.
        assert_eq!(listing(&root, 2, &|| true).unwrap_err(), "canceled");
        assert!(inside(&root, "a.txt").is_ok());
        assert!(inside(&root, "sub/b.txt").is_ok());
        for out in ["../secret.txt", "/etc/passwd", "sub/../../secret.txt", "", "sub", "none.txt"] {
            assert!(inside(&root, out).is_err(), "{out}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.join("secret.txt"), root.join("link.txt")).unwrap();
            assert!(inside(&root, "link.txt").is_err(), "a link out of the folder");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hak_names_lead_nowhere_else() {
        assert_eq!(hak_name("My_Tiles.hak").unwrap(), "my_tiles");
        for bad in ["", "../x", "a/b", "a.b", "with space", &"x".repeat(33)] {
            assert!(hak_name(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn pictures_decode_from_the_top() {
        // A 2×2 PNG: red, green over blue, white.
        let mut png_bytes = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut png_bytes, 2, 2);
            enc.set_color(png::ColorType::Rgb);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().unwrap();
            w.write_image_data(&[255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]).unwrap();
        }
        let (w, h, data) = decode(&png_bytes, None).unwrap();
        assert_eq!((w, h), (2, 2));
        assert_eq!(&data[..8], [255, 0, 0, 255, 0, 255, 0, 255]);
        assert_eq!(&data[8..12], [0, 0, 255, 255]);
        // The same as a TGA (stored bottom first) reads the same.
        let rgba = mg_image::Rgba { width: 2, height: 2, data: data.clone() };
        let tga = mg_image::write_tga(&rgba.top_down());
        let (_, _, again) = decode(&tga, None).unwrap();
        assert_eq!(again, data);
        assert!(decode(b"not a picture", None).is_err());
        assert!(decode(&png_bytes, Some("bmp")).is_err());
    }

    /// A header that declares more pixels than a plugin's picture may have
    /// is refused by the header, before anything is decoded: a
    /// run-length TGA of a few megabytes would otherwise be a gigabyte of
    /// pixels before it was found too large.
    #[test]
    fn a_picture_too_large_is_refused_by_its_header() {
        // A run-length true-color TGA of 16,384 × 16,384 with one packet.
        let mut tga = vec![0u8; 18];
        tga[2] = 10;
        tga[12..14].copy_from_slice(&16_384u16.to_le_bytes());
        tga[14..16].copy_from_slice(&16_384u16.to_le_bytes());
        tga[16] = 32;
        tga.extend_from_slice(&[0xff, 1, 2, 3, 4]);
        assert_eq!(decode(&tga, None).unwrap_err(), "the picture is too large");
        assert_eq!(decode(&tga, Some("tga")).unwrap_err(), "the picture is too large");
        // Both headers of a DDS.
        let mut bioware = Vec::new();
        for n in [16_384u32, 16_384, 4, 0, 0] {
            bioware.extend_from_slice(&n.to_le_bytes());
        }
        assert_eq!(decode(&bioware, Some("dds")).unwrap_err(), "the picture is too large");
        let mut standard = b"DDS ".to_vec();
        for n in [124u32, 0, 16_384, 16_384] {
            standard.extend_from_slice(&n.to_le_bytes());
        }
        assert_eq!(decode(&standard, None).unwrap_err(), "the picture is too large");
        // One within the limit, cut short, is told as cut short.
        tga[12..14].copy_from_slice(&64u16.to_le_bytes());
        tga[14..16].copy_from_slice(&64u16.to_le_bytes());
        assert_ne!(decode(&tga, None).unwrap_err(), "the picture is too large");
        assert_eq!(size_of(&tga, None), Ok((64, 64)));
        assert_eq!(size_of(&tga[..10], None), Ok((0, 0)));
    }
}
