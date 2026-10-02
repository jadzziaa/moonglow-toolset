//! NWScript: the official compiler, built in ([`compiler`]), and the
//! script tooling built on it.

pub mod analysis;
pub mod compiler;
pub mod lex;
pub mod outline;
pub mod spec;

pub use compiler::{CompileError, Compiled, Compiler};

#[cfg(test)]
mod tests {
    use mg_core::ResType;

    use super::*;

    const SPEC: &str =
        "#define ENGINE_NUM_STRUCTURES 0\nint TRUE = 1;\nvoid PrintString(string sString);\n";

    fn sources(name: &str, t: ResType) -> Option<Vec<u8>> {
        if t != ResType::NSS {
            return None;
        }
        let text = match name {
            "nwscript" => SPEC,
            "ok" => "#include \"inc\"\nvoid main() { PrintString(Greeting()); }\n",
            "inc" => "string Greeting() { return \"hi\"; }\n",
            "bad" => "void main() { Nope(); }\n",
            "cond" => "int StartingConditional() { return TRUE; }\n",
            _ => return None,
        };
        Some(text.as_bytes().to_vec())
    }

    #[test]
    fn compiles_with_includes_and_reports_errors() {
        let mut c = Compiler::new(sources);
        let out = c.compile("ok").unwrap();
        assert_eq!(&out.ncs[..8], b"NCS V1.0");
        assert!(out.ndb.is_none());
        assert!(c.compile("cond").is_ok());

        let err = c.compile("bad").unwrap_err();
        assert!(err.code < 0);
        assert!(err.message.contains("UNDEFINED IDENTIFIER"), "{}", err.message);
        let err = c.compile("missing").unwrap_err();
        assert!(!err.message.is_empty());

        c.set_debug_output(true);
        let out = c.compile("ok").unwrap();
        assert!(out.ndb.is_some_and(|d| d.starts_with(b"NDB")));
    }

    #[test]
    fn include_files_validate_without_code() {
        let mut c = Compiler::new(sources);
        assert!(c.compile("inc").is_err(), "no entry point");
        c.set_require_entry_point(false);
        assert!(c.compile("inc").unwrap().ncs.is_empty());
    }

    #[test]
    fn compilers_on_several_threads_do_not_mix() {
        let handles: Vec<_> = (0..4)
            .map(|i| {
                std::thread::spawn(move || {
                    let mut c = Compiler::new(move |name, t| {
                        if name == "t" && t == ResType::NSS {
                            Some(format!("void main() {{ PrintString(\"{i}\"); }}").into_bytes())
                        } else {
                            sources(name, t)
                        }
                    });
                    (0..20).map(|_| c.compile("t").unwrap().ncs).collect::<Vec<_>>()
                })
            })
            .collect();
        let results: Vec<Vec<Vec<u8>>> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        for (i, r) in results.iter().enumerate() {
            assert!(r.windows(2).all(|w| w[0] == w[1]));
            let needle = i.to_string().into_bytes();
            assert!(r[0].windows(needle.len()).any(|w| w == needle));
        }
    }
}
