//! Functions declared with one return type and defined (or declared
//! again) with another, in what a compile read: the script and the
//! includes the compiler asked for. Also an include's entry point that
//! Aurora would compile in place of the script's ([`included_entry`]).
//!
//! The game's own toolset (Aurora) compiles them, keeping the return type
//! met first at the calls, and so does Moonglow (`nwscript/README.md`, the
//! return-type-mismatch patch); newer builds of the compiler refuse them.
//! The base game's `x2_inc_cutscene` has one: `CutCreateObjectCopy` is
//! declared `object` and defined `void`, so every script that includes it
//! would fail. Each one found is a [`CompileWarning`].

use std::collections::{HashMap, HashSet};

use crate::compiler::CompileWarning;
use crate::outline::outline;

/// A function's prototype or definition, as the warning needs it.
#[derive(Debug, Clone)]
struct Signature {
    name: String,
    returns: String,
    params: Vec<String>,
    defined: bool,
    line: usize,
    at: usize,
}

/// What a source declares, in order, and where it includes what.
#[derive(Debug, Default)]
struct Facts {
    functions: Vec<Signature>,
    /// Where (byte) and what (lower case, without `.nss`).
    includes: Vec<(usize, String)>,
}

fn facts(src: &[u8]) -> Facts {
    let text = String::from_utf8_lossy(src);
    let o = outline(src);
    let line = |at: usize| mg_core::text::line_of(text.as_bytes(), at);
    let functions = (o.functions.into_iter())
        .map(|f| Signature {
            line: line(f.name_span.start),
            at: f.span.start,
            returns: f.return_type,
            params: f.params.into_iter().map(|p| p.ty).collect(),
            defined: f.body.is_some(),
            name: f.name,
        })
        .collect();
    let includes = (o.includes.into_iter()).map(|i| (i.span.start, script_name(&i.name))).collect();
    Facts { functions, includes }
}

/// An include's name as the compiler asks for it: lower case, without
/// `.nss`.
fn script_name(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    name.strip_suffix(".nss").map(str::to_string).unwrap_or(name)
}

/// What each source compiled declares, kept from one compile to the next
/// (an include read by many scripts is looked at once while it stays the
/// same).
#[derive(Debug, Default)]
pub(crate) struct Cache {
    files: HashMap<String, (u64, std::rc::Rc<Facts>)>,
}

impl Cache {
    fn facts(&mut self, name: &str, src: &[u8]) -> std::rc::Rc<Facts> {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        src.hash(&mut h);
        let hash = h.finish();
        match self.files.get(name) {
            Some((same, f)) if *same == hash => f.clone(),
            _ => {
                let f = std::rc::Rc::new(facts(src));
                self.files.insert(name.to_string(), (hash, f.clone()));
                f
            }
        }
    }
}

/// The functions of `script` and what it includes (`sources`: each by
/// its name, lower case, as the compiler read it), in the order the
/// compiler meets them: an include's where its `#include` is, once.
fn in_order(
    script: &str,
    sources: &HashMap<String, &[u8]>,
    cache: &mut Cache,
) -> Vec<(String, Signature)> {
    fn walk(
        name: &str,
        sources: &HashMap<String, &[u8]>,
        cache: &mut Cache,
        seen: &mut HashSet<String>,
        out: &mut Vec<(String, Signature)>,
    ) {
        let Some(src) = sources.get(name) else { return };
        let facts = cache.facts(name, src);
        let mut includes = facts.includes.iter().peekable();
        for f in &facts.functions {
            while let Some((_, inc)) = includes.next_if(|(at, _)| *at < f.at) {
                if seen.insert(inc.clone()) {
                    walk(inc, sources, cache, seen, out);
                }
            }
            out.push((name.to_string(), f.clone()));
        }
        for (_, inc) in includes {
            if seen.insert(inc.clone()) {
                walk(inc, sources, cache, seen, out);
            }
        }
    }
    let mut out = Vec::new();
    let name = script_name(script);
    let mut seen = HashSet::from([name.clone()]);
    walk(&name, sources, cache, &mut seen, &mut out);
    out
}

/// What a function of return type `ty` gives where it returns nothing.
fn empty_value(ty: &str) -> String {
    match ty {
        "int" => "0".into(),
        "float" => "0.0".into(),
        "string" => "\"\"".into(),
        "object" => "OBJECT_INVALID".into(),
        "vector" => "[0.0, 0.0, 0.0]".into(),
        ty => format!("an empty {ty}"),
    }
}

/// The return-type mismatches in what compiling `script` read.
pub(crate) fn mismatches(
    script: &str,
    sources: &HashMap<String, &[u8]>,
    cache: &mut Cache,
) -> Vec<CompileWarning> {
    let mut first: HashMap<String, (String, Signature)> = HashMap::new();
    let mut out = Vec::new();
    let same = |a: &str, b: &str| a.split_whitespace().eq(b.split_whitespace());
    for (file, f) in in_order(script, sources, cache) {
        let Some((kept_file, kept)) = first.get(&f.name) else {
            first.insert(f.name.clone(), (file, f));
            continue;
        };
        // (Parameters that differ are the compiler's error.)
        let params_same = kept.params.len() == f.params.len()
            && kept.params.iter().zip(&f.params).all(|(a, b)| same(a, b));
        if !params_same || same(&kept.returns, &f.returns) {
            continue;
        }
        let how = |s: &Signature| if s.defined { "defined" } else { "declared" };
        let at = |file: &str, s: &Signature| format!("{file}.nss({})", s.line);
        let (name, kept_ty, ty) = (&f.name, &kept.returns, &f.returns);
        let what = format!(
            "{name} is {} {kept_ty} at {} and {} {ty} at {}",
            how(kept),
            at(kept_file, kept),
            how(&f),
            at(&file, &f)
        );
        let meaning = if !f.defined {
            format!("calls to it return {kept_ty}, as it was {} first", how(kept))
        } else if ty == "void" {
            format!(
                "what a call to it returns is always {}, as it returns nothing",
                empty_value(kept_ty)
            )
        } else if kept_ty == "void" {
            "its value goes where its callers keep none: in the game a call to it \
             overwrites the caller's last variable with it"
                .to_string()
        } else {
            format!(
                "its {ty} goes where its callers keep a {kept_ty}: in the game the script \
                 stops, saying nothing, where that value is used"
            )
        };
        out.push(CompileWarning {
            file: file.clone(),
            line: f.line,
            message: format!(
                "{}: WARNING: {what}. Aurora compiles it so (newer compilers refuse it): {meaning}",
                at(&file, &f)
            ),
        });
    }
    out
}

/// The sources of `script` and what it includes, read through `read`
/// (each by its name, lower case, without `.nss`), for a script the
/// compiler passed over.
pub(crate) fn gather(
    script: &str,
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
    cache: &mut Cache,
) -> HashMap<String, Vec<u8>> {
    let mut out = HashMap::new();
    let mut todo = vec![script_name(script)];
    while let Some(name) = todo.pop() {
        if out.contains_key(&name) {
            continue;
        }
        let Some(src) = read(&name) else { continue };
        todo.extend(cache.facts(&name, &src).includes.iter().map(|(_, i)| i.clone()));
        out.insert(name, src);
    }
    out
}

/// An include's own entry point, which Aurora (8193.37) compiles as the
/// script in place of the script's: a `main` in an include of an `int
/// StartingConditional()` script, or a `main` or `StartingConditional`
/// in an include of a script with none. Moonglow's compiler takes the
/// script's own, or makes nothing of a script with none (the owner's
/// ruling of 2026-11-26: kept, and warned of). In the game Aurora's
/// condition runs the include's `main` and always fails.
pub(crate) fn included_entry(
    script: &str,
    sources: &HashMap<String, &[u8]>,
    cache: &mut Cache,
) -> Vec<CompileWarning> {
    let name = script_name(script);
    let entry =
        |f: &Signature| f.defined && matches!(f.name.as_str(), "main" | "StartingConditional");
    let all = in_order(script, sources, cache);
    let own = all.iter().find(|(file, f)| *file == name && entry(f)).map(|(_, f)| f.name.clone());
    let theirs = all.iter().find(|(file, f)| {
        *file != name
            && entry(f)
            && match &own {
                Some(own) => own == "StartingConditional" && f.name == "main",
                None => true,
            }
    });
    let Some((file, f)) = theirs else { return Vec::new() };
    let at = format!("{file}.nss({})", f.line);
    let what = match own {
        Some(_) => format!(
            "{file} has a main of its own, which Aurora compiles as {name} in place of its \
             StartingConditional. Moonglow compiles {name}'s own: in the game Aurora's runs \
             {file}'s main, and as a condition always fails"
        ),
        None => format!(
            "{name} has no main or StartingConditional of its own, so Moonglow compiles \
             nothing for it. Aurora compiles {file}'s {} as {name}",
            f.name
        ),
    };
    vec![CompileWarning {
        file: file.clone(),
        line: f.line,
        message: format!("{at}: WARNING: {what}"),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_of(script: &str, files: &[(&str, &str)]) -> Vec<String> {
        let sources: HashMap<String, &[u8]> =
            files.iter().map(|(n, t)| (n.to_string(), t.as_bytes())).collect();
        (included_entry(script, &sources, &mut Cache::default()).into_iter())
            .map(|w| w.message)
            .collect()
    }

    /// The three ways of writing where Aurora's Build compiles an
    /// include's entry point (`aurora_scripts::codegen_compiles_as_aurora_builds_it`),
    /// through an include of an include too; none where the script's own
    /// is a `main`, or no include has one, or the include's `main` is
    /// commented out.
    #[test]
    fn an_includes_entry_point_that_aurora_compiles() {
        let lib = "void Helper() {}

void main() { Helper(); }
";
        let cond = "#include \"lib\"\nint StartingConditional() { return TRUE; }\n";
        let said = entry_of("cond", &[("cond", cond), ("lib", lib)]);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].starts_with(
                "lib.nss(3): WARNING: lib has a main of its own, which Aurora compiles as cond \
                 in place of its StartingConditional."
            ),
            "{}",
            said[0]
        );
        let said = entry_of(
            "only",
            &[("only", "#include \"mid\"\n"), ("mid", "#include \"lib\"\n"), ("lib", lib)],
        );
        assert!(said[0].starts_with("lib.nss(3): WARNING: only has no main"), "{said:?}");
        assert!(said[0].ends_with("Aurora compiles lib's main as only"), "{said:?}");
        let sc = "int StartingConditional() { return TRUE; }\n";
        let said = entry_of("uses", &[("uses", "#include \"sc\"\n"), ("sc", sc)]);
        assert!(said[0].ends_with("Aurora compiles sc's StartingConditional as uses"), "{said:?}");

        let main = "#include \"sc\"\nvoid main() {}\n";
        assert!(entry_of("m", &[("m", main), ("sc", sc)]).is_empty());
        assert!(entry_of("c", &[("c", "#include \"inc\"\n"), ("inc", "void A() {}\n")]).is_empty());
        let commented = "void A() {}\n// void main() {}\n/* void main() {} */\n";
        assert!(
            entry_of("c", &[("c", cond.replace("lib", "inc").as_str()), ("inc", commented)])
                .is_empty()
        );
    }

    fn run(script: &str, files: &[(&str, &str)]) -> Vec<String> {
        let sources: HashMap<String, &[u8]> =
            files.iter().map(|(n, t)| (n.to_string(), t.as_bytes())).collect();
        mismatches(script, &sources, &mut Cache::default()).into_iter().map(|w| w.message).collect()
    }

    /// `x2_inc_cutscene`'s: declared `object`, defined `void`, in an
    /// include; the warning names both lines and the file.
    #[test]
    fn a_function_declared_object_and_defined_void() {
        let inc = "// doc\nobject Copy(float f, object o);\n\nvoid Copy(float f, object o)\n{\n}\n";
        let said = run("uses", &[("uses", "#include \"inc\"\nvoid main() {}\n"), ("inc", inc)]);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].starts_with(
                "inc.nss(4): WARNING: Copy is declared object at inc.nss(2) and defined void at \
                 inc.nss(4)."
            ),
            "{}",
            said[0]
        );
        assert!(said[0].contains("always OBJECT_INVALID"), "{}", said[0]);
    }

    /// Across files, in the order the compiler meets them: an include's
    /// declarations where its `#include` is; a commented-out include is
    /// none.
    #[test]
    fn the_order_follows_the_includes() {
        let script = "int A();\n#include \"inc\"\n/* #include \"other\" */\nvoid main() {}\n";
        let said =
            run("main", &[("main", script), ("inc", "void A() {}\n"), ("other", "string A();\n")]);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(said[0].starts_with("inc.nss(1): WARNING: A is declared int at main.nss(1)"));

        // The definition first, then a prototype of another type.
        let said = run("main", &[("main", "int B() { return 1; }\nvoid B();\nvoid main() {}\n")]);
        assert_eq!(said.len(), 1);
        assert!(said[0].contains("calls to it return int, as it was defined first"), "{}", said[0]);
    }

    /// Where the definition returns a value its callers keep no room for,
    /// or keep room for another type in.
    #[test]
    fn a_definition_that_returns_another_type() {
        let said = run("m", &[("m", "void A();\nint A() { return 1; }\nvoid main() {}\n")]);
        assert!(said[0].contains("overwrites the caller's last variable"), "{}", said[0]);
        let said = run("m", &[("m", "string A();\nint A() { return 1; }\nvoid main() {}\n")]);
        assert!(said[0].contains("its int goes where its callers keep a string"), "{}", said[0]);
    }

    /// The same return type, however written; other parameters (the
    /// compiler's own error); a struct's name.
    #[test]
    fn nothing_where_they_agree() {
        assert!(
            run("m", &[("m", "int A();\nint  A() { return 1; }\nvoid main() {}\n")]).is_empty()
        );
        assert!(run("m", &[("m", "int A(int n);\nvoid A() {}\nvoid main() {}\n")]).is_empty());
        let s = "struct S { int n; };\nstruct T { int n; };\nstruct S A();\nstruct T A() { struct T t; return t; }\n";
        assert_eq!(run("m", &[("m", s)]).len(), 1);
    }
}
