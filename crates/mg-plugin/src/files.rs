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
use mg_resman::ResKey;
use mlua::{Lua, Table, Value as LuaValue};

use crate::runtime::{CODEPAGE, Shared, This, fail};
use crate::{Answer, HakData, HakWrite, MAX_FILE, Question};

/// The names of what these functions hand out, as the reference writes
/// them.
pub(crate) const NAMES: [&str; 10] = [
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

/// The most files of a folder that are listed.
const MAX_LISTED: usize = 20_000;
/// The most pixels a picture may have (a small file can unpack to a
/// great many).
const MAX_PIXELS: u64 = 1 << 26;

/// A file's bytes, if it is not too large for a plugin.
fn read(path: &Path) -> mlua::Result<Vec<u8>> {
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
/// with `/`, sorted. Links are not followed out of it.
fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut todo = vec![PathBuf::new()];
    while let Some(sub) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(dir.join(&sub)) else { continue };
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            let path = sub.join(e.file_name());
            if kind.is_dir() {
                todo.push(path);
            } else if kind.is_file() && out.len() < MAX_LISTED {
                let parts: Vec<String> = path
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect();
                out.push(parts.join("/"));
            }
        }
    }
    out.sort();
    out
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
            let file = lua.create_table()?;
            file.set("name", name_of(&path))?;
            file.set("bytes", lua.create_string(read(&path)?)?)?;
            Ok(LuaValue::Table(file))
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
            if !dir.is_dir() {
                return fail(format!("{} is not a folder", dir.display()));
            }
            let folder = lua.create_table()?;
            folder.set("name", name_of(&dir))?;
            let d = dir.clone();
            folder.set("files", lua.create_function(move |_, _: This| Ok(listing(&d)))?)?;
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
            folder.set(
                "text",
                lua.create_function(move |lua, (_, name): (This, String)| {
                    let bytes = read(&inside(&dir, &name)?)?;
                    // UTF-8 if it is, else as the module's codepage has it.
                    match String::from_utf8(bytes) {
                        Ok(text) => lua.create_string(text.trim_start_matches('\u{feff}')),
                        Err(e) => lua.create_string(CODEPAGE.decode(e.as_bytes()).as_bytes()),
                    }
                })?,
            )?;
            Ok(LuaValue::Table(folder))
        })?,
    )?;
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
        s.haks.borrow_mut().push(HakWrite { name: hak.clone(), files: Vec::new() });
    }
    let mut haks = s.haks.borrow_mut();
    let files = &mut haks.iter_mut().find(|h| h.name == hak).expect("added").files;
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
            let key = ResKey::parse("module", ResType::IFO).expect("a resref");
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

/// A picture's pixels, rows from the top.
fn decode(bytes: &[u8], kind: Option<&str>) -> Result<(u32, u32, Vec<u8>), String> {
    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n";
    let kind = match kind.map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "png",
        Some("tga") => "tga",
        Some("dds") => "dds",
        Some(other) => return Err(format!("{other:?} is not a kind of picture (png, tga, dds)")),
        None if bytes.starts_with(PNG) => "png",
        None if bytes.starts_with(b"DDS ") => "dds",
        None => "tga",
    };
    if kind != "png" {
        let restype = if kind == "dds" { ResType::DDS } else { ResType::TGA };
        let texture = mg_image::read(restype, bytes).map_err(|e| e.to_string())?;
        if u64::from(texture.width) * u64::from(texture.height) > MAX_PIXELS {
            return Err("the picture is too large".into());
        }
        let rgba = texture.to_rgba().top_down();
        return Ok((rgba.width, rgba.height, rgba.data));
    }
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let (width, height) = (reader.info().width, reader.info().height);
    if u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err("the picture is too large".into());
    }
    let size = reader.output_buffer_size().ok_or("the picture is too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let data = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Grayscale => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => {
            data.as_chunks::<2>().0.iter().flat_map(|p| [p[0], p[0], p[0], p[1]]).collect()
        }
        png::ColorType::Rgb => {
            data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
        }
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Indexed => return Err("the picture's palette was not unpacked".into()),
    };
    Ok((width, height, rgba))
}

/// `mg.image(bytes, kind?)`: a PNG, TGA or DDS picture decoded.
pub(crate) fn image(
    lua: &Lua,
    (bytes, kind): (mlua::LuaString, Option<String>),
) -> mlua::Result<Table> {
    let (width, height, data) =
        decode(&bytes.as_bytes(), kind.as_deref()).or_else(|e| fail(format!("mg.image: {e}")))?;
    let t = lua.create_table()?;
    t.set("width", width)?;
    t.set("height", height)?;
    t.set(
        "pixel",
        lua.create_function(move |_, (_, x, y): (This, f64, f64)| {
            if x < 0.0 || y < 0.0 || x >= f64::from(width) || y >= f64::from(height) {
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
        assert_eq!(listing(&root), ["a.txt", "sub/b.txt"]);
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
}
