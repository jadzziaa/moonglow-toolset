//! Which compiler Direct3D 12 compiles shaders with: Microsoft's DXC, the
//! `dxcompiler.dll` Moonglow ships beside its programs, or else FXC
//! (Windows' own `d3dcompiler_47.dll`).
//!
//! wgpu's Direct3D 12 backend writes each pipeline stage in HLSL and
//! compiles it. FXC can't compile a texture read with derivatives in a
//! loop of a count it doesn't know (X3511: the game's normal-mapped lit
//! programs, `fslit_nm` and `fslit_sm_nm`, and builders' copies of them);
//! DXC can. wgpu's own default (`Dx12Compiler::Auto`) looks for
//! `dxcompiler.dll` by name, wherever Windows finds a DLL (`PATH`
//! included, so a stray copy of another version could be taken), so
//! Moonglow names the one beside the program instead, and FXC where there
//! is none ([`choose`]). Each wgpu instance Moonglow makes takes
//! [`backend_options`] (the window's, the headless device's, the tests'),
//! and the debug log's `graphics:` line says which ([`compiler`]).
//!
//! A `dxcompiler.dll` that is there but can't be loaded would make wgpu
//! drop Direct3D 12 altogether, so it is loaded first, and FXC is taken
//! where it fails. `WGPU_DX12_COMPILER` (`fxc`, `dxc`, `auto`) still
//! decides, as for any wgpu program. With DXC, wgpu offers no Direct3D 12
//! adapter whose driver lacks shader model 6 (the Direct3D 12 drivers of
//! 2017 on have it); `WGPU_DX12_COMPILER=fxc` takes FXC for such a one.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The file name of DXC's library, beside `moonglow.exe` and `mg.exe`.
pub const DXC_DLL: &str = "dxcompiler.dll";

/// The environment variable wgpu reads the compiler from.
pub const ENVIRONMENT: &str = "WGPU_DX12_COMPILER";

/// The compiler Direct3D 12 compiles shaders with, as chosen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShaderCompiler {
    /// DXC: the `dxcompiler.dll` at this path, beside the program.
    Dxc(PathBuf),
    /// FXC, and why not DXC.
    Fxc(String),
    /// As `WGPU_DX12_COMPILER` says (its value).
    Environment(String),
    /// Not on Windows: no Direct3D 12.
    NoDirect3D12,
}

impl ShaderCompiler {
    /// Whether shaders are compiled by DXC (named so where the
    /// environment chose).
    pub fn is_dxc(&self) -> bool {
        match self {
            ShaderCompiler::Dxc(_) => true,
            ShaderCompiler::Environment(v) => {
                matches!(v.to_lowercase().as_str(), "dxc" | "dynamicdxc" | "staticdxc")
            }
            _ => false,
        }
    }
}

impl fmt::Display for ShaderCompiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShaderCompiler::Dxc(path) => {
                write!(f, "Direct3D 12 compiles shaders with DXC ({})", path.display())
            }
            ShaderCompiler::Fxc(why) => write!(f, "Direct3D 12 compiles shaders with FXC ({why})"),
            ShaderCompiler::Environment(v) => {
                write!(f, "Direct3D 12 compiles shaders as {ENVIRONMENT}={v} says")
            }
            ShaderCompiler::NoDirect3D12 => write!(f, "no Direct3D 12 here"),
        }
    }
}

/// The compiler for Direct3D 12, and what it is: on Windows (`windows`),
/// `WGPU_DX12_COMPILER`'s (`environment`) where it names one wgpu knows;
/// else DXC where `dxcompiler.dll` lies beside the program (`exe`) and
/// `loads` (it is loaded, as wgpu will load it); else FXC. Elsewhere,
/// wgpu's default is left (it has no Direct3D 12 there).
pub fn choose(
    windows: bool,
    exe: Option<&Path>,
    environment: Option<&str>,
    loads: impl FnOnce(&Path) -> Result<(), String>,
) -> (wgpu::Dx12Compiler, ShaderCompiler) {
    if !windows {
        return (wgpu::Dx12Compiler::default(), ShaderCompiler::NoDirect3D12);
    }
    if let Some(value) = environment
        && let Ok(compiler) = value.parse::<wgpu::Dx12Compiler>()
    {
        return (compiler, ShaderCompiler::Environment(value.to_string()));
    }
    let fxc = |why: String| (wgpu::Dx12Compiler::Fxc, ShaderCompiler::Fxc(why));
    let Some(dir) = exe.and_then(Path::parent) else {
        return fxc("the program's folder is not known".into());
    };
    let dll = dir.join(DXC_DLL);
    let program =
        exe.and_then(Path::file_name).map_or("the program".into(), |n| n.to_string_lossy());
    if !dll.is_file() {
        return fxc(format!("no {DXC_DLL} beside {program}"));
    }
    match loads(&dll) {
        Ok(()) => (
            wgpu::Dx12Compiler::DynamicDxc { dxc_path: dll.to_string_lossy().into_owned() },
            ShaderCompiler::Dxc(dll),
        ),
        Err(e) => fxc(format!("{} could not be loaded: {e}", dll.display())),
    }
}

/// This process's choice ([`choose`]), made once.
fn chosen() -> &'static (wgpu::Dx12Compiler, ShaderCompiler) {
    static CHOSEN: OnceLock<(wgpu::Dx12Compiler, ShaderCompiler)> = OnceLock::new();
    CHOSEN.get_or_init(|| {
        let exe = std::env::current_exe().ok();
        let environment = std::env::var(ENVIRONMENT).ok();
        choose(cfg!(windows), exe.as_deref(), environment.as_deref(), load)
    })
}

/// The compiler Direct3D 12 compiles shaders with in this process.
pub fn compiler() -> &'static ShaderCompiler {
    &chosen().1
}

/// The backend options every wgpu instance Moonglow makes takes: wgpu's
/// from the environment (`WGPU_*`), with the Direct3D 12 compiler chosen
/// ([`compiler`]).
pub fn backend_options() -> wgpu::BackendOptions {
    let mut options = wgpu::BackendOptions::from_env_or_default();
    options.dx12.shader_compiler = chosen().0.clone();
    options
}

/// Loads DXC's library as wgpu will, and finds the function wgpu calls
/// in it; kept loaded (wgpu loads it again, for the process's life).
#[cfg(windows)]
#[allow(unsafe_code)]
fn load(dll: &Path) -> Result<(), String> {
    // SAFETY: loading a DLL runs its initialization; this is the
    // dxcompiler.dll shipped beside Moonglow, which wgpu loads and runs
    // in any case once it is named.
    let library = unsafe { libloading::Library::new(dll) }.map_err(|e| e.to_string())?;
    // SAFETY: the symbol is only looked up, never called through this
    // type.
    unsafe { library.get::<unsafe extern "system" fn()>(b"DxcCreateInstance\0") }
        .map_err(|e| format!("no DxcCreateInstance in it ({e})"))?;
    std::mem::forget(library);
    Ok(())
}

#[cfg(not(windows))]
fn load(_: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(_: &Path) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn dxc_is_taken_where_it_lies_beside_the_program() {
        let dir = std::env::temp_dir().join(format!("mg-dx12-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("moonglow.exe");
        let dll = dir.join(DXC_DLL);
        let _ = std::fs::remove_file(&dll);
        // None beside it: FXC, said why.
        let (c, said) = choose(true, Some(&exe), None, loaded);
        assert!(matches!(c, wgpu::Dx12Compiler::Fxc));
        assert_eq!(said, ShaderCompiler::Fxc("no dxcompiler.dll beside moonglow.exe".into()));
        assert!(!said.is_dxc());
        assert_eq!(
            said.to_string(),
            "Direct3D 12 compiles shaders with FXC (no dxcompiler.dll beside moonglow.exe)"
        );
        // There: DXC, by its whole path (not wherever Windows would find one).
        std::fs::write(&dll, b"").unwrap();
        let (c, said) = choose(true, Some(&exe), None, loaded);
        match c {
            wgpu::Dx12Compiler::DynamicDxc { dxc_path } => {
                assert_eq!(Path::new(&dxc_path), dll)
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(said, ShaderCompiler::Dxc(dll.clone()));
        assert!(said.is_dxc());
        assert!(said.to_string().starts_with("Direct3D 12 compiles shaders with DXC ("));
        // There but not loading: FXC (DXC named would cost Direct3D 12).
        let (c, said) = choose(true, Some(&exe), None, |_| Err("not a DLL".into()));
        assert!(matches!(c, wgpu::Dx12Compiler::Fxc));
        assert!(
            matches!(&said, ShaderCompiler::Fxc(w) if w.ends_with("could not be loaded: not a DLL"))
        );
        // WGPU_DX12_COMPILER decides; one wgpu doesn't know is passed over.
        let (c, said) = choose(true, Some(&exe), Some("fxc"), loaded);
        assert!(matches!(c, wgpu::Dx12Compiler::Fxc));
        assert_eq!(said, ShaderCompiler::Environment("fxc".into()));
        assert!(!said.is_dxc());
        assert_eq!(said.to_string(), "Direct3D 12 compiles shaders as WGPU_DX12_COMPILER=fxc says");
        assert!(choose(true, Some(&exe), Some("DXC"), loaded).1.is_dxc());
        let (_, said) = choose(true, Some(&exe), Some("speedy"), loaded);
        assert_eq!(said, ShaderCompiler::Dxc(dll.clone()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn elsewhere_there_is_no_direct3d_12() {
        let (c, said) = choose(false, None, Some("fxc"), |_| panic!("not loaded"));
        assert!(matches!(c, wgpu::Dx12Compiler::Auto));
        assert_eq!(said, ShaderCompiler::NoDirect3D12);
        let (_, said) = choose(true, None, None, |_| panic!("not loaded"));
        assert_eq!(said, ShaderCompiler::Fxc("the program's folder is not known".into()));
    }
}
