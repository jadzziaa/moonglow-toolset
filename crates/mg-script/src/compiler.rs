//! The official NWScript compiler, built in (see `nwscript/README.md`).
//!
//! The compiler's C API calls back for every source it needs (the language
//! spec `nwscript.nss`, the script, its includes) and for every file it
//! writes, with no context pointer. A thread-local therefore points at the
//! compiler instance that is currently running; instances are `!Send`, so
//! each stays on its thread and the routing is always right.

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::marker::PhantomData;

use mg_core::ResType;
use thiserror::Error;

#[repr(C)]
struct CScriptCompiler {
    _private: [u8; 0],
}

#[repr(C)]
struct NativeCompileResult {
    code: i32,
    str_: *const c_char,
}

type WriteCb = extern "C" fn(*const c_char, u16, *const u8, usize, bool) -> i32;
type LoadCb = extern "C" fn(*const c_char, u16) -> bool;

// The declarations match `nwscript/compilerapi.h` (ABI 1).
#[allow(unsafe_code)]
unsafe extern "C" {
    fn scriptCompApiGetABIVersion() -> i32;
    fn scriptCompApiNewCompiler(
        src: i32,
        bin: i32,
        dbg: i32,
        write: WriteCb,
        load: LoadCb,
    ) -> *mut CScriptCompiler;
    fn scriptCompApiInitCompiler(
        c: *mut CScriptCompiler,
        lang: *const c_char,
        write_debug: bool,
        max_include_depth: i32,
        graphviz_out: *const c_char,
        output_alias: *const c_char,
    );
    fn scriptCompApiCompileFile(
        c: *mut CScriptCompiler,
        name: *const c_char,
    ) -> NativeCompileResult;
    fn scriptCompApiDeliverFile(c: *mut CScriptCompiler, data: *const c_char, size: usize);
    fn scriptCompApiSetOptimizationFlags(c: *mut CScriptCompiler, flags: u32);
    fn scriptCompApiSetGenerateDebuggerOutput(c: *mut CScriptCompiler, state: bool);
    fn scriptCompApiSetRequireEntryPoint(c: *mut CScriptCompiler, state: bool);
    fn scriptCompApiDestroyCompiler(c: *mut CScriptCompiler);
}

/// Optimisation flags (`CSCRIPTCOMPILER_OPTIMIZE_*`).
pub mod optimize {
    pub const NOTHING: u32 = 0;
    /// Remove dead functions: what the game and Aurora use.
    pub const SAFE: u32 = 0x1;
    pub const AGGRESSIVE: u32 = 0x1 | 0x4;
}

/// A compile failure: the talk-table string of the error (negative, as the
/// compiler reports it) and the compiler's message, e.g.
/// `t_bad.nss(1): ERROR: UNDEFINED IDENTIFIER (UndefinedFn)`. The compiler
/// stops at the first error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{message}")]
pub struct CompileError {
    pub code: i32,
    pub message: String,
}

impl CompileError {
    /// Where the error is: the script (without `.nss`; an include when the
    /// error is in one) and the 1-based line, from the message.
    pub fn location(&self) -> Option<(String, usize)> {
        let (file, rest) = self.message.split_once(".nss(")?;
        let (line, _) = rest.split_once(')')?;
        let file = file.rsplit([' ', ':', '/', '\\']).next().unwrap_or(file);
        Some((file.to_string(), line.trim().parse().ok()?))
    }
}

/// A compiled script.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Compiled {
    /// NCS bytecode; empty when compiling without an entry point
    /// ([`Compiler::set_require_entry_point`]).
    pub ncs: Vec<u8>,
    /// NDB debug information, when enabled.
    pub ndb: Option<Vec<u8>>,
}

/// Supplies source files: `(name without extension, type)` → bytes. The
/// compiler asks for `nwscript` (the language spec) first, then scripts and
/// their includes (type `NSS`).
pub type Resolver<'a> = dyn FnMut(&str, ResType) -> Option<Vec<u8>> + 'a;

struct Active {
    compiler: *mut CScriptCompiler,
    resolver: *mut Resolver<'static>,
    output: Compiled,
}

thread_local! {
    static ACTIVE: RefCell<Option<Active>> = const { RefCell::new(None) };
}

#[allow(unsafe_code)]
extern "C" fn load(name: *const c_char, restype: u16) -> bool {
    // A panic must not unwind into C++.
    std::panic::catch_unwind(|| {
        ACTIVE.with(|a| {
            let a = a.borrow();
            let Some(active) = a.as_ref() else { return false };
            // SAFETY: the compiler passes a NUL-terminated name.
            let name = unsafe { CStr::from_ptr(name) }.to_string_lossy();
            // SAFETY: `resolver` points at the resolver borrowed for the
            // duration of the compiler call that invoked this callback.
            let resolver = unsafe { &mut *active.resolver };
            match resolver(&name, ResType(restype)) {
                Some(data) => {
                    // SAFETY: the compiler copies the data before returning;
                    // this is the documented way to answer the callback.
                    unsafe {
                        scriptCompApiDeliverFile(active.compiler, data.as_ptr().cast(), data.len())
                    };
                    true
                }
                None => false,
            }
        })
    })
    .unwrap_or(false)
}

#[allow(unsafe_code)]
extern "C" fn write(
    _name: *const c_char,
    restype: u16,
    data: *const u8,
    size: usize,
    _binary: bool,
) -> i32 {
    std::panic::catch_unwind(|| {
        ACTIVE.with(|a| {
            let mut a = a.borrow_mut();
            let Some(active) = a.as_mut() else { return 1 };
            let bytes = if size == 0 {
                Vec::new()
            } else {
                // SAFETY: the compiler passes `size` readable bytes.
                unsafe { std::slice::from_raw_parts(data, size) }.to_vec()
            };
            match ResType(restype) {
                ResType::NCS => active.output.ncs = bytes,
                ResType::NDB => active.output.ndb = Some(bytes),
                _ => return 1,
            }
            0
        })
    })
    .unwrap_or(1)
}

/// An instance of the official compiler. Not thread-safe (and not `Send`):
/// use one per thread.
pub struct Compiler<'r> {
    raw: *mut CScriptCompiler,
    resolver: Box<Resolver<'r>>,
    _not_send: PhantomData<*const ()>,
}

impl std::fmt::Debug for Compiler<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Compiler")
    }
}

impl<'r> Compiler<'r> {
    /// A compiler that reads sources through `resolver`, with the game's and
    /// Aurora's optimisation settings and no debug output.
    ///
    /// # Panics
    /// If the built-in compiler's ABI is not the one this wrapper was written
    /// for (a build problem, not a runtime condition).
    #[allow(unsafe_code)]
    pub fn new(resolver: impl FnMut(&str, ResType) -> Option<Vec<u8>> + 'r) -> Compiler<'r> {
        // SAFETY: a plain call into the built-in library.
        assert_eq!(unsafe { scriptCompApiGetABIVersion() }, 1, "compiler ABI mismatch");
        // SAFETY: the callbacks are valid for the life of the program.
        let raw = unsafe {
            scriptCompApiNewCompiler(
                ResType::NSS.0 as i32,
                ResType::NCS.0 as i32,
                ResType::NDB.0 as i32,
                write,
                load,
            )
        };
        let mut c = Compiler { raw, resolver: Box::new(resolver), _not_send: PhantomData };
        let lang = CString::new("nwscript").expect("no NUL");
        let alias = CString::new("scriptout").expect("no NUL");
        c.with_active(|raw| {
            // SAFETY: `raw` is live; init loads the language spec through the
            // active resolver.
            unsafe {
                scriptCompApiInitCompiler(
                    raw,
                    lang.as_ptr(),
                    false,
                    16,
                    std::ptr::null(),
                    alias.as_ptr(),
                )
            };
        });
        c.set_optimization(optimize::SAFE);
        c
    }

    /// Runs `f` with this compiler registered as the thread's active one.
    #[allow(unsafe_code)]
    fn with_active<T>(&mut self, f: impl FnOnce(*mut CScriptCompiler) -> T) -> (T, Compiled) {
        let resolver: *mut Resolver<'r> = &mut *self.resolver;
        // SAFETY: the pointer is only used by callbacks during `f`, while
        // `self.resolver` is borrowed; its lifetime is erased for the
        // thread-local, which is cleared again before returning.
        let resolver: *mut Resolver<'static> = unsafe { std::mem::transmute(resolver) };
        let previous = ACTIVE.with(|a| {
            a.borrow_mut().replace(Active {
                compiler: self.raw,
                resolver,
                output: Compiled::default(),
            })
        });
        let result = f(self.raw);
        let active = ACTIVE.with(|a| std::mem::replace(&mut *a.borrow_mut(), previous));
        (result, active.map(|a| a.output).unwrap_or_default())
    }

    /// Compiles `name` (a script resref, without `.nss`).
    #[allow(unsafe_code)]
    pub fn compile(&mut self, name: &str) -> Result<Compiled, CompileError> {
        let cname = CString::new(name).map_err(|_| CompileError {
            code: -1,
            message: format!("{name:?}: invalid script name"),
        })?;
        // SAFETY: `raw` is live and the name is NUL-terminated.
        let (result, output) =
            self.with_active(|raw| unsafe { scriptCompApiCompileFile(raw, cname.as_ptr()) });
        if result.code == 0 {
            return Ok(output);
        }
        let mut message = if result.str_.is_null() {
            String::new()
        } else {
            // SAFETY: a NUL-terminated static buffer owned by the compiler.
            unsafe { CStr::from_ptr(result.str_) }.to_string_lossy().trim().to_string()
        };
        if message.is_empty() {
            // Some failures (e.g. a script that cannot be loaded) come with a
            // code only; its text is talk-table string -code.
            message =
                format!("{name}.nss: compiler error {} (StrRef {})", result.code, -result.code);
        }
        Err(CompileError { code: result.code, message })
    }

    /// Sets the optimisation flags ([`optimize`]).
    #[allow(unsafe_code)]
    pub fn set_optimization(&mut self, flags: u32) {
        // SAFETY: `raw` is live.
        unsafe { scriptCompApiSetOptimizationFlags(self.raw, flags) };
    }

    /// Emit NDB debug information with the bytecode.
    #[allow(unsafe_code)]
    pub fn set_debug_output(&mut self, on: bool) {
        // SAFETY: `raw` is live.
        unsafe { scriptCompApiSetGenerateDebuggerOutput(self.raw, on) };
    }

    /// With `false`, scripts without `main`/`StartingConditional` (include
    /// files) are checked but produce no bytecode.
    #[allow(unsafe_code)]
    pub fn set_require_entry_point(&mut self, on: bool) {
        // SAFETY: `raw` is live.
        unsafe { scriptCompApiSetRequireEntryPoint(self.raw, on) };
    }
}

impl Drop for Compiler<'_> {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: created by `scriptCompApiNewCompiler`, destroyed once.
        unsafe { scriptCompApiDestroyCompiler(self.raw) };
    }
}

#[cfg(test)]
mod location_tests {
    use super::CompileError;

    #[test]
    fn error_locations() {
        let e = |m: &str| CompileError { code: -1, message: m.to_string() };
        assert_eq!(
            e("broken.nss(4): ERROR: VARIABLE DEFINED WITHOUT TYPE").location(),
            Some(("broken".into(), 4))
        );
        assert_eq!(
            e("usesinc.nss: brokeninc.nss(1): ERROR: UNKNOWN STATE").location(),
            Some(("brokeninc".into(), 1))
        );
        assert_eq!(e("\"x\": invalid script name").location(), None);
    }
}
