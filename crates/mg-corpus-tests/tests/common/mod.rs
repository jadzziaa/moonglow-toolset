//! Helpers shared by the engine tests.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use mg_image::Rgba;

use mg_core::{ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

/// Builds `<dir>/modules/<name>.mod` from a shipped module with `script`
/// (NWScript source) as OnModuleLoad, the given haks attached and `extra`
/// resources added.
#[allow(dead_code)]
pub(crate) fn probe_module(
    root: &Path,
    dir: &Path,
    name: &str,
    script: &str,
    haks: &[&str],
    extra: &[(ResKey, Vec<u8>)],
) {
    let compiler = mg_testkit::nwn_tool("nwn_script_comp").expect("nwn_script_comp");
    std::fs::create_dir_all(dir.join("modules")).unwrap();
    let nss = dir.join(format!("{name}.nss"));
    let ncs = dir.join(format!("{name}.ncs"));
    std::fs::write(&nss, script).unwrap();
    let out = Command::new(&compiler)
        .args(["-o", ncs.to_str().unwrap(), nss.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));

    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let mut bytes = erf.data(e).unwrap().into_owned();
        if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(name.as_bytes().to_vec()));
            let list = haks
                .iter()
                .map(|h| {
                    let mut s = Struct::new(8);
                    s.set("Mod_Hak", Value::String(h.as_bytes().to_vec()));
                    s
                })
                .collect();
            ifo.root.set("Mod_HakList", Value::List(list));
            bytes = ifo.to_bytes().unwrap();
        }
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    for (k, data) in extra {
        w.add(k.resref, k.restype, data.clone()).unwrap();
    }
    w.add(ResRef::from_str(name).unwrap(), ResType::NCS, std::fs::read(&ncs).unwrap()).unwrap();
    std::fs::write(dir.join(format!("modules/{name}.mod")), w.to_bytes().unwrap()).unwrap();
}

/// Rewrites every GFF in the module with Moonglow's writer, as a module
/// whose every resource was edited would be saved.
#[allow(dead_code)] // not every test binary uses every helper
pub(crate) fn rewrite_all_gffs(m: &mut mg_module::Module) {
    let keys: Vec<ResKey> = m.keys().copied().filter(|k| k.restype.is_gff()).collect();
    for k in keys {
        let gff = m.gff(&k).unwrap().unwrap();
        m.set_gff(k, &gff).unwrap();
    }
}

/// Compiles NWScript source with the official compiler (`nwn_script_comp`).
#[allow(dead_code)]
pub(crate) fn compile(dir: &Path, name: &str, source: &str) -> Vec<u8> {
    let compiler = mg_testkit::nwn_tool("nwn_script_comp").expect("nwn_script_comp");
    let nss = dir.join(format!("{name}.nss"));
    let ncs = dir.join(format!("{name}.ncs"));
    std::fs::write(&nss, source).unwrap();
    let out = Command::new(&compiler)
        .args(["-o", ncs.to_str().unwrap(), nss.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));
    std::fs::read(&ncs).unwrap()
}

/// Walks every area and object during module load, before any heartbeat or
/// AI has run, yielding with `DelayCommand` only when the instruction budget
/// runs low (then resuming at the same area and object). Ends with the done
/// marker and padding that makes the server flush its buffered log.
#[allow(dead_code)]
pub(crate) const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }

void Finish()
{
    Log("MG_DONE");
    int i;
    for (i = 0; i < 2000; i++)
        Log("MG_PAD ................................................................");
}

void ProbeFrom(int nArea, int nObj)
{
    int a = 0;
    object oArea = GetFirstArea();
    while (GetIsObjectValid(oArea) && a < nArea) { oArea = GetNextArea(); a++; }
    while (GetIsObjectValid(oArea))
    {
        string sArea = GetResRef(oArea);
        if (nObj == 0)
            Log("MG_AREA " + sArea + "|" + GetTag(oArea) + "|" + GetName(oArea));
        int i = 0;
        object o = GetFirstObjectInArea(oArea);
        while (GetIsObjectValid(o))
        {
            if (i >= nObj)
            {
                if (GetScriptInstructionsRemaining() < 20000)
                {
                    DelayCommand(0.0, ProbeFrom(a, i));
                    return;
                }
                vector v = GetPosition(o);
                Log("MG_OBJ " + sArea + "|" + IntToString(GetObjectType(o)) + "|" + GetTag(o)
                    + "|" + GetResRef(o) + "|" + FloatToString(v.x, 0, 2) + "," + FloatToString(v.y, 0, 2)
                    + "," + FloatToString(v.z, 0, 2) + "|" + FloatToString(GetFacing(o), 0, 1));
            }
            i++;
            o = GetNextObjectInArea(oArea);
        }
        nObj = 0;
        a++;
        oArea = GetNextArea();
    }
    DelayCommand(0.0, Finish());
}

void main()
{
    Log("MG_MODULE " + GetName(GetModule()) + "|" + GetTag(GetModule()));
    ProbeFrom(0, 0);
}
"#;

/// The probe's view of the world, sorted. Creatures keep only their area,
/// type, tag and blueprint: their AI may move them between probe chunks.
#[allow(dead_code)]
pub(crate) fn world(run: &mg_testkit::engine::ServerRun) -> Vec<String> {
    let mut lines: Vec<String> = ["MG_MODULE", "MG_AREA", "MG_OBJ"]
        .iter()
        .flat_map(|tag| run.values(tag).into_iter().map(move |v| format!("{tag} {v}")))
        .map(|l| match l.strip_prefix("MG_OBJ ") {
            Some(obj) if obj.split('|').nth(1) == Some("1") => {
                let identity: Vec<&str> = obj.split('|').take(4).collect();
                format!("MG_OBJ {} (creature)", identity.join("|"))
            }
            _ => l,
        })
        .collect();
    lines.sort();
    lines
}

/// The game client's settings for tests, for a controlled comparison
/// (merged into the client's defaults): no splash, movies or effects that
/// Moonglow doesn't draw, and Moonglow's texture filtering (8× anisotropic).
#[allow(dead_code)]
pub(crate) const SETTINGS: &str = r#"[graphics]
	[graphics.fbo]
		[graphics.fbo.hdr-bloom]
			enabled = false
		[graphics.fbo.high-contrast]
			enabled = false
		[graphics.fbo.sharpen]
			enabled = false
		[graphics.fbo.ssao]
			enabled = false
		[graphics.fbo.vibrance]
			enabled = false
	[graphics.general]
		shader-quality = "High Quality"
	[graphics.grass]
		mode = 0
	[graphics.hilite]
		enabled = false
	[graphics.intro]
		[graphics.intro.splash]
			enabled = false
	[graphics.keyholing]
		enabled = false
	[graphics.lod]
		enabled = false
	[graphics.movies]
		enabled = false
		[graphics.movies.intro]
			enabled = false
	[graphics.shadows]
		[graphics.shadows.creatures]
			mode = 0
		[graphics.shadows.environment]
			enabled = false
	[graphics.skyboxes]
		enabled = false
	[graphics.tile-borders]
		enabled = false
	[graphics.video]
		[graphics.video.anisotropic-filtering]
			mode = 8
"#;

#[allow(dead_code)]
pub(crate) fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[allow(dead_code)]
/// Runs the client until the scene is ready and screenshots its window.
/// One client at a time: the screenshot finds the window by its title.
pub(crate) fn client_screenshot(dir: &Path, module: &str) -> Option<Rgba> {
    static ONE_CLIENT: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _only = ONE_CLIENT.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    // In single player the game's server logs to the client's log.
    let log = dir.join("user/logs/nwclientLog1.txt");
    let _ = std::fs::remove_file(&log);
    let mut child = Command::new(repo().join("tools/nwclient/run-client.sh"))
        .arg(dir)
        .args(["+TestNewModule", module])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let start = Instant::now();
    let mut ready = false;
    while start.elapsed() < Duration::from_secs(180) {
        if std::fs::read_to_string(&log).is_ok_and(|l| l.contains("MG_READY")) {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    std::thread::sleep(Duration::from_secs(3));
    let shot = dir.join("client.png");
    let ok = ready
        && Command::new("python3")
            .arg(repo().join("tools/aurora/xdrive.py"))
            .args(["winshot", "Neverwinter Nights: Enhanced Edition"])
            .arg(&shot)
            .status()
            .is_ok_and(|s| s.success());
    let _ = child.kill();
    let _ = child.wait();
    ok.then(|| read_png(&shot))
}

#[allow(dead_code)]
pub(crate) fn read_png(path: &Path) -> Rgba {
    let decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let data = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgb => {
            data.as_chunks::<3>().0.iter().flat_map(|p| [p[0], p[1], p[2], 255]).collect()
        }
        _ => data.to_vec(),
    };
    Rgba { width: info.width, height: info.height, data: rgba }
}

#[allow(dead_code)]
pub(crate) fn save_png(img: &Rgba, path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), img.width, img.height);
    enc.set_color(png::ColorType::Rgba);
    enc.write_header().unwrap().write_image_data(&img.data).unwrap();
}
