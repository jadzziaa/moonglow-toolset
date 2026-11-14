//! A module's NWScript tests (`mg test`): scripts named `test_*` that
//! report through the assertion include [`INCLUDE`], run in the game's
//! server by a driver put into a scratch copy of the module, and read back
//! from the server's log.

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;
use mg_schema::{StructExt, ifo};

use crate::{Module, ModuleError};

/// The assertion include, as modules include it.
pub const INCLUDE_NAME: &str = "mg_test";
/// Its text. (Versioned by `MG_TEST_FORMAT`: modules depend on the lines
/// it writes.)
pub const INCLUDE: &str = include_str!("mg_test.nss");
/// The format of the lines the include and the driver write.
pub const FORMAT: u32 = 2;
/// The driver script put into the scratch copy.
pub const DRIVER_NAME: &str = "mg_test_run";
/// What every line of a test run has in the log.
pub const MARK: &str = "MG_TEST ";

/// Whether `name` is what `filter` asks for: a part of the name, or with
/// `*`s a pattern for all of it. No filter: every name.
pub fn matches(filter: Option<&str>, name: &str) -> bool {
    let Some(filter) = filter.map(str::to_lowercase).filter(|f| !f.is_empty()) else {
        return true;
    };
    if !filter.contains('*') {
        return name.contains(&filter);
    }
    let parts: Vec<&str> = filter.split('*').collect();
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    // (Cut where the parts end, never by their lengths alone: a name's
    // character of two bytes may lie there.)
    let Some(mut rest) = name.strip_prefix(first).and_then(|rest| rest.strip_suffix(last)) else {
        return false;
    };
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

/// The module's tests: its scripts named `test_*` that can be run (an
/// include file named so is none), by name.
pub fn test_scripts(module: &Module, filter: Option<&str>) -> Vec<ResKey> {
    let mut found: Vec<ResKey> = (module.keys_of(ResType::NSS))
        .filter(|k| {
            let name = k.resref.to_lowercase().to_string();
            name.starts_with("test_") && matches(filter, &name)
        })
        .filter(|k| module.get(k).is_some_and(mg_script::outline::has_entry_point))
        .copied()
        .collect();
    found.sort_by_key(|k| k.resref.to_lowercase().to_string());
    found
}

/// The driver: each test as an event of its own (so one that runs out of
/// instructions takes no other with it), then the module's own
/// OnModuleLoad script, `original`. The tests begin once that is through;
/// the run is done `settle` seconds after the last was started. It names
/// itself to the include (`MG_TEST_DRIVER`), which then names the test of
/// an assertion made outside the driver's run of it (format 2).
pub fn driver(tests: &[ResKey], original: Option<&str>, settle: f32) -> String {
    let mut s = String::from(
        "// Written by `mg test`: runs the module's tests.\n\
         void MgTestRun(string sScript)\n{\n    \
         WriteTimestampedLogEntry(\"MG_TEST RUN \" + sScript);\n    \
         ExecuteScript(sScript, OBJECT_SELF);\n    \
         WriteTimestampedLogEntry(\"MG_TEST END \" + sScript);\n}\n\n\
         void MgTestDone()\n{\n    WriteTimestampedLogEntry(\"MG_TEST DONE\");\n}\n\n\
         void main()\n{\n",
    );
    s += &format!("    SetLocalString(GetModule(), \"MG_TEST_DRIVER\", \"{DRIVER_NAME}\");\n");
    s += &format!("    WriteTimestampedLogEntry(\"MG_TEST BEGIN {FORMAT} {}\");\n", tests.len());
    let mut at = 0.5_f32;
    for t in tests {
        let name = t.resref.to_lowercase();
        s += &format!("    DelayCommand({at:.2}, MgTestRun(\"{name}\"));\n");
        at += 0.05;
    }
    s += &format!("    DelayCommand({:.2}, MgTestDone());\n", at + settle.max(0.0));
    if let Some(original) = original.filter(|o| !o.is_empty()) {
        s += &format!("    ExecuteScript(\"{original}\", OBJECT_SELF);\n");
    }
    s += "}\n";
    s
}

/// Puts the driver into `module` (a scratch copy: never the builder's own)
/// as its OnModuleLoad script, with the assertion include where the module
/// has none of its own. Each test is made to say when its `main()` has
/// returned ([`returning`]). Returns whether the module's own include was
/// kept, and the tests that will say so.
pub fn inject(
    module: &mut Module,
    tests: &[ResKey],
    settle: f32,
) -> Result<(bool, Vec<String>), ModuleError> {
    let include = ResKey::parse(INCLUDE_NAME, ResType::NSS).expect("a resource name");
    let own = module.contains(&include);
    if !own {
        module.set(include, INCLUDE.as_bytes().to_vec());
    }
    let mut returns = Vec::new();
    for key in tests {
        let name = key.resref.to_lowercase().to_string();
        let source = module.get(key).map(|s| String::from_utf8_lossy(s).into_owned());
        if let Some(wrapped) = source.and_then(|s| returning(&s, &name)) {
            module.set(*key, wrapped.into_bytes());
            returns.push(name);
        }
    }
    let mut info = module.info()?;
    let original = info.root.read(&ifo::MOD_ON_MOD_LOAD).to_string();
    let original = Some(original.as_str()).filter(|o| !o.eq_ignore_ascii_case(DRIVER_NAME));
    let driver_key = ResKey::parse(DRIVER_NAME, ResType::NSS).expect("a resource name");
    module.set(driver_key, driver(tests, original, settle).into_bytes());
    let name: ResRef = DRIVER_NAME.parse().expect("a resource name");
    info.root.write(&ifo::MOD_ON_MOD_LOAD, name);
    module.set_info(&info)?;
    Ok((own, returns))
}

/// A test's source made to say when its `main()` has returned: `main` is
/// given another name (in its place, so that no line moves), and a `main`
/// after it calls that and writes `MG_TEST RETURNED name`. A script that
/// stops inside (a function the server has not got, a division by zero)
/// never writes it, though the script that ran it goes on. `None` where
/// no `void main()` is found to rename.
pub fn returning(source: &str, name: &str) -> Option<String> {
    let pattern = regex_lite::Regex::new(r"\bvoid\s+main\s*\(\s*(void)?\s*\)").expect("a pattern");
    // (Not one after `//` on its line: a test may well have another's
    // in a comment above its own.)
    let found = pattern.find_iter(source).find(|m| {
        let line = &source[source[..m.start()].rfind('\n').map_or(0, |i| i + 1)..m.start()];
        !line.contains("//")
    })?;
    let mut out = String::with_capacity(source.len() + 160);
    out += &source[..found.start()];
    out += "void MgTestMain()";
    out += &source[found.end()..];
    out += &format!(
        "\nvoid main()\n{{\n    MgTestMain();\n    WriteTimestampedLogEntry(\"{MARK}RETURNED \
         {name}\");\n}}\n"
    );
    Some(out)
}

/// One assertion of a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    /// Made after its test's `main()` had returned (in a `DelayCommand`,
    /// an `AssignCommand`), and named by its test (format 2).
    pub delayed: bool,
}

/// One test script's run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Test {
    pub script: String,
    pub cases: Vec<Case>,
    /// Why it did not run to its end (out of instructions, a script that
    /// isn't there), with what the server logged meanwhile.
    pub error: Option<String>,
}

impl Test {
    pub fn failed(&self) -> usize {
        self.cases.iter().filter(|c| !c.ok).count()
    }
}

/// What a run's log said.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub tests: Vec<Test>,
    /// The format the module's lines are in (`BEGIN`), if it said.
    pub format: Option<u32>,
    /// The run said it was done.
    pub done: bool,
    /// The tests that say when their `main()` has returned
    /// ([`returning`]): one of them that ends without saying so stopped
    /// inside.
    pub returns: Vec<String>,
    /// The test running has said its `main()` returned.
    returned: bool,
    /// The test running (begun, not ended) and what was logged since.
    running: Option<(usize, Vec<String>)>,
    /// What the engine said of scripts that failed, by script, until the
    /// test's end is known.
    said: Vec<(String, String)>,
}

/// Cases reported while no test was running that name no test of the run
/// (a module's own driver, an include of format 1, a script other than a
/// test asserting) go under this name.
pub const OUTSIDE: &str = "(outside a test)";

/// A case's name that names its test first, `[test_a] name`: the test's
/// script and the rest.
fn named_test(name: &str) -> Option<(&str, &str)> {
    let (script, rest) = name.strip_prefix('[')?.split_once("] ")?;
    let resref = (1..=16).contains(&script.len())
        && script.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    resref.then_some((script, rest))
}

impl Report {
    fn test(&mut self, script: &str) -> usize {
        match self.tests.iter().position(|t| t.script == script) {
            Some(i) => i,
            None => {
                self.tests.push(Test { script: script.to_string(), ..Default::default() });
                self.tests.len() - 1
            }
        }
    }

    fn unfinished(&mut self) {
        if let Some((i, said)) = self.running.take() {
            let mut why = String::from("did not run to its end");
            if self.tests[i].cases.is_empty() {
                why += " (and made no assertion)";
            }
            for line in said.iter().take(8) {
                why += "\n";
                why += line;
            }
            let script = self.tests[i].script.clone();
            for (_, line) in self.said.extract_if(.., |(of, _)| *of == script) {
                why += "\n";
                why += &line;
            }
            self.tests[i].error = Some(why);
        }
    }

    /// Reads a line of the engine's own log (`nwengineLog.txt`), where a
    /// script that failed is reported: `Script error: NAME OID: …, ERROR:
    /// TOO MANY INSTRUCTIONS`.
    pub fn engine_line(&mut self, line: &str) {
        let Some(at) = line.find("Script error: ") else { return };
        let said = &line[at..];
        let script = said["Script error: ".len()..].split(' ').next().unwrap_or_default();
        match self.tests.iter_mut().find(|t| t.script == script) {
            Some(Test { error: Some(why), .. }) => {
                *why += "\n";
                *why += said;
            }
            _ => self.said.push((script.to_string(), said.to_string())),
        }
    }

    /// Reads a line of the server's log. Returns whether the run is done.
    pub fn line(&mut self, line: &str) -> bool {
        let Some(at) = line.find(MARK) else {
            // (What the engine says of a script that failed.)
            if let Some((_, said)) = &mut self.running
                && !line.trim().is_empty()
            {
                said.push(line.trim().to_string());
            }
            return self.done;
        };
        let said = line[at + MARK.len()..].trim();
        let (word, rest) = said.split_once(' ').unwrap_or((said, ""));
        match word {
            "BEGIN" => self.format = rest.split(' ').next().and_then(|n| n.parse().ok()),
            "RUN" => {
                self.unfinished();
                let i = self.test(rest);
                self.running = Some((i, Vec::new()));
                self.returned = false;
            }
            "RETURNED" => {
                if self.running.as_ref().is_some_and(|(i, _)| self.tests[*i].script == rest) {
                    self.returned = true;
                }
            }
            "END" => {
                if self.running.as_ref().is_some_and(|(i, _)| self.tests[*i].script == rest) {
                    // (The script that ran it went on, but its own main()
                    // never returned: it stopped inside.)
                    match self.returns.iter().any(|r| r == rest) && !self.returned {
                        true => self.unfinished(),
                        false => self.running = None,
                    }
                }
            }
            "PASS" | "FAIL" => {
                let (name, detail) = rest.split_once(" :: ").unwrap_or((rest, ""));
                // (Format 2: one made outside its test's run names it.)
                let named = (self.format >= Some(2)).then(|| named_test(name)).flatten();
                let of = named.and_then(|(script, rest)| {
                    let i = self.tests.iter().position(|t| t.script.eq_ignore_ascii_case(script));
                    i.map(|i| (i, rest))
                });
                let (i, name, delayed) = match (of, &self.running) {
                    (Some((i, rest)), _) => (i, rest, true),
                    (None, Some((i, _))) => (*i, name, false),
                    (None, None) => (self.test(OUTSIDE), name, false),
                };
                self.tests[i].cases.push(Case {
                    name: name.to_string(),
                    ok: word == "PASS",
                    detail: detail.to_string(),
                    delayed,
                });
            }
            "DONE" => {
                self.unfinished();
                self.done = true;
            }
            _ => {}
        }
        self.done
    }

    /// The run is over without having said so (a timeout, a server that
    /// ended): a test still running did not finish, and those in
    /// `expected` that never began did not run.
    pub fn close(&mut self, expected: &[ResKey]) {
        self.unfinished();
        for t in expected {
            let name = t.resref.to_lowercase().to_string();
            if !self.tests.iter().any(|x| x.script == name) {
                let i = self.test(&name);
                self.tests[i].error = Some("did not run".into());
            }
        }
    }

    pub fn passed(&self) -> usize {
        self.tests.iter().map(|t| t.cases.len() - t.failed()).sum()
    }

    pub fn failed(&self) -> usize {
        self.tests.iter().map(Test::failed).sum()
    }

    pub fn errors(&self) -> usize {
        self.tests.iter().filter(|t| t.error.is_some()).count()
    }

    /// Every test ran to its end, none failed, and the run said it was
    /// done.
    pub fn ok(&self) -> bool {
        self.done && self.failed() == 0 && self.errors() == 0
    }

    pub fn to_json(&self) -> serde_json::Value {
        use serde_json::json;
        let tests: Vec<_> = (self.tests.iter())
            .map(|t| {
                let cases: Vec<_> = (t.cases.iter())
                    .map(|c| {
                        json!({
                            "name": c.name, "ok": c.ok, "detail": c.detail, "delayed": c.delayed,
                        })
                    })
                    .collect();
                json!({
                    "script": t.script, "passed": t.cases.len() - t.failed(),
                    "failed": t.failed(), "error": t.error, "cases": cases,
                })
            })
            .collect();
        json!({
            "ok": self.ok(), "done": self.done, "format": self.format,
            "passed": self.passed(), "failed": self.failed(), "errors": self.errors(),
            "tests": tests,
        })
    }

    /// The run as a JUnit report (the XML that build servers read): a
    /// suite a test script, a case an assertion.
    pub fn junit(&self, name: &str, seconds: f64) -> String {
        fn esc(s: &str) -> String {
            let mut out = String::new();
            for c in s.chars() {
                match c {
                    '&' => out += "&amp;",
                    '<' => out += "&lt;",
                    '>' => out += "&gt;",
                    '"' => out += "&quot;",
                    '\n' => out += "&#10;",
                    c if (c as u32) < 0x20 && c != '\t' => {}
                    c => out.push(c),
                }
            }
            out
        }
        let cases = self.passed() + self.failed() + self.errors();
        let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        x += &format!(
            "<testsuites name=\"{}\" tests=\"{cases}\" failures=\"{}\" errors=\"{}\" \
             time=\"{seconds:.3}\">\n",
            esc(name),
            self.failed(),
            self.errors() + usize::from(!self.done),
        );
        for t in &self.tests {
            x += &format!(
                "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"{}\">\n",
                esc(&t.script),
                t.cases.len() + usize::from(t.error.is_some()),
                t.failed(),
                usize::from(t.error.is_some()),
            );
            for c in &t.cases {
                let open = format!(
                    "    <testcase classname=\"{}\" name=\"{}\"",
                    esc(&t.script),
                    esc(&c.name)
                );
                match c.ok {
                    true => x += &format!("{open}/>\n"),
                    false => {
                        x += &format!(
                            "{open}>\n      <failure message=\"{}\"/>\n    </testcase>\n",
                            esc(&c.detail)
                        );
                    }
                }
            }
            if let Some(why) = &t.error {
                x += &format!(
                    "    <testcase classname=\"{0}\" name=\"{0}\">\n      <error \
                     message=\"{1}\"/>\n    </testcase>\n",
                    esc(&t.script),
                    esc(why)
                );
            }
            x += "  </testsuite>\n";
        }
        if !self.done {
            x += "  <testsuite name=\"mg test\" tests=\"1\" failures=\"0\" errors=\"1\">\n    \
                  <testcase classname=\"mg test\" name=\"the run\">\n      <error \
                  message=\"the tests did not say they were done\"/>\n    </testcase>\n  \
                  </testsuite>\n";
        }
        x += "</testsuites>\n";
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str) -> ResKey {
        ResKey::parse(name, ResType::NSS).unwrap()
    }

    #[test]
    fn a_filter_is_a_part_or_a_pattern() {
        assert!(matches(None, "test_a") && matches(Some(""), "test_a"));
        assert!(matches(Some("Rules"), "test_rules_feats"));
        assert!(!matches(Some("spells"), "test_rules_feats"));
        assert!(matches(Some("test_*_feats"), "test_rules_feats"));
        assert!(matches(Some("*rules*"), "test_rules_feats"));
        assert!(!matches(Some("test_*_feat"), "test_rules_feats"));
        assert!(!matches(Some("a*a"), "a"));
        // A script's name with a byte above 0x7F (two bytes of its text)
        // where a part's length would cut it.
        assert!(!matches(Some("test_*x"), "test_\u{e9}"));
        assert!(matches(Some("test_*x"), "test_\u{e9}x"));
        assert!(matches(Some("*\u{e9}*"), "test_\u{e9}x"));
    }

    #[test]
    fn the_tests_are_the_scripts_named_so_that_can_run() {
        let mut m = Module::new();
        m.set(key("test_b"), b"void main() {}".to_vec());
        m.set(key("test_a"), b"#include \"mg_test\"\nvoid main() {}".to_vec());
        m.set(key("test_inc"), b"int Helper() { return 1; }".to_vec());
        m.set(key("other"), b"void main() {}".to_vec());
        let names = |f| -> Vec<String> {
            test_scripts(&m, f).iter().map(|k| k.resref.to_string()).collect()
        };
        assert_eq!(names(None), ["test_a", "test_b"]);
        assert_eq!(names(Some("_b")), ["test_b"]);
    }

    #[test]
    fn the_driver_runs_each_test_and_the_module_s_own_script() {
        let d = driver(&[key("test_a"), key("test_b")], Some("x2_mod_def_load"), 2.0);
        assert!(d.contains("MG_TEST BEGIN 2 2"));
        assert!(d.contains("SetLocalString(GetModule(), \"MG_TEST_DRIVER\", \"mg_test_run\");"));
        assert!(d.contains("DelayCommand(0.50, MgTestRun(\"test_a\"));"));
        assert!(d.contains("DelayCommand(0.55, MgTestRun(\"test_b\"));"));
        assert!(d.contains("DelayCommand(2.60, MgTestDone());"));
        assert!(d.contains("ExecuteScript(\"x2_mod_def_load\", OBJECT_SELF);"));
        assert!(!driver(&[], None, 0.0).contains("ExecuteScript(\"\""));
    }

    #[test]
    fn a_log_is_read_into_tests_and_cases() {
        let log = "\
[Sat Oct 10 15:23:02] Loading Module: m
[Sat Oct 10 15:23:02] MG_TEST BEGIN 1 3
[Sat Oct 10 15:23:03] MG_TEST RUN test_a
[Sat Oct 10 15:23:03] MG_TEST PASS adds
[Sat Oct 10 15:23:03] MG_TEST FAIL feats: has cleave :: got 0, expected 1
[Sat Oct 10 15:23:03] MG_TEST END test_a
[Sat Oct 10 15:23:03] MG_TEST RUN test_tmi
[Sat Oct 10 15:23:03] Script test_tmi, OID: 0, Tag: , ERROR: TOO MANY INSTRUCTIONS
[Sat Oct 10 15:23:03] MG_TEST RUN test_b
[Sat Oct 10 15:23:03] MG_TEST PASS fine
[Sat Oct 10 15:23:03] MG_TEST END test_b
[Sat Oct 10 15:23:04] MG_TEST PASS later
";
        let mut r = Report::default();
        for line in log.lines() {
            assert!(!r.line(line));
        }
        assert!(!r.ok() && !r.done);
        assert!(r.line("[x] MG_TEST DONE"));
        assert_eq!((r.format, r.passed(), r.failed(), r.errors()), (Some(1), 3, 1, 1));
        assert_eq!(r.tests[0].cases[1].name, "feats: has cleave");
        assert_eq!(r.tests[0].cases[1].detail, "got 0, expected 1");
        let tmi = r.tests[1].error.as_ref().unwrap();
        assert!(tmi.contains("did not run to its end") && tmi.contains("TOO MANY INSTRUCTIONS"));
        assert_eq!(r.tests[3].script, OUTSIDE);
        // The engine's own log says why, before or after the test's end
        // is known.
        let engine = "E [15:31:43] ScriptVM (x.cpp:264:ReportError): Script error: test_tmi OID: \
                      00000000, Tag: , ERROR: TOO MANY INSTRUCTIONS";
        r.engine_line(engine);
        assert!(r.tests[1].error.as_ref().unwrap().ends_with("ERROR: TOO MANY INSTRUCTIONS"));
        let mut early = Report::default();
        early.line("MG_TEST RUN test_tmi");
        early.engine_line(engine);
        early.line("MG_TEST DONE");
        assert!(early.tests[0].error.as_ref().unwrap().contains("Script error: test_tmi OID"));
        assert_eq!(r.to_json()["tests"][0]["failed"], 1);
        let x = r.junit("m", 1.5);
        assert!(x.contains("<testsuites name=\"m\" tests=\"5\" failures=\"1\" errors=\"1\""));
        assert!(x.contains("<failure message=\"got 0, expected 1\"/>"));
        assert!(x.contains("TOO MANY INSTRUCTIONS"));
    }

    /// Format 2: an assertion made after its test (in a `DelayCommand`)
    /// names its test, and goes under it, whatever ran meanwhile. One
    /// naming no test of the run is kept as it was, outside a test; in
    /// format 1 nothing is read as a test's name.
    #[test]
    fn a_delayed_assertion_goes_under_its_own_test() {
        let mut r = Report::default();
        for line in [
            "MG_TEST BEGIN 2 2",
            "MG_TEST RUN test_a",
            "MG_TEST PASS now",
            "MG_TEST END test_a",
            "MG_TEST RUN test_b",
            "MG_TEST PASS b: [test_a] not a name of its test",
            "MG_TEST FAIL [test_a] math: later :: got 1, expected 2",
            "MG_TEST END test_b",
            "MG_TEST PASS [TEST_B] after both",
            "MG_TEST PASS [nw_heartbeat] someone else's",
            "MG_TEST PASS [not a resref] plain",
        ] {
            r.line(line);
        }
        assert!(r.line("MG_TEST DONE"));
        let cases = |i: usize| -> Vec<(&str, bool, bool)> {
            r.tests[i].cases.iter().map(|c| (c.name.as_str(), c.ok, c.delayed)).collect()
        };
        assert_eq!(cases(0), [("now", true, false), ("math: later", false, true)]);
        assert_eq!(r.tests[0].cases[1].detail, "got 1, expected 2");
        assert_eq!(
            cases(1),
            [("b: [test_a] not a name of its test", true, false), ("after both", true, true)]
        );
        assert_eq!(r.tests[2].script, OUTSIDE);
        assert_eq!(
            cases(2),
            [("[nw_heartbeat] someone else's", true, false), ("[not a resref] plain", true, false)]
        );
        assert_eq!(r.to_json()["tests"][0]["cases"][1]["delayed"], true);
        assert!(r.junit("m", 0.0).contains("classname=\"test_a\" name=\"math: later\""));

        let mut old = Report::default();
        for line in ["MG_TEST BEGIN 1 1", "MG_TEST RUN test_a", "MG_TEST END test_a"] {
            old.line(line);
        }
        old.line("MG_TEST PASS [test_a] later");
        assert_eq!(
            (old.tests[1].script.as_str(), old.tests[1].cases[0].name.as_str()),
            (OUTSIDE, "[test_a] later")
        );
    }

    /// A test that stops inside its main() (the script that ran it goes
    /// on, and says END) is an error, not a test that passed with nothing
    /// asserted.
    #[test]
    fn a_test_that_stops_inside_is_an_error() {
        let source = "#include \"mg_test\"\n// void main() of old\nvoid  main ( )\n{\n    Assert(1, \"x\");\n}\n";
        let wrapped = returning(source, "test_a").unwrap();
        assert!(wrapped.contains("// void main() of old\nvoid MgTestMain()\n{"), "{wrapped}");
        assert!(wrapped.ends_with(
            "void main()\n{\n    MgTestMain();\n    WriteTimestampedLogEntry(\"MG_TEST RETURNED test_a\");\n}\n"
        ));
        assert_eq!(wrapped.lines().nth(4), source.lines().nth(4), "no line moves");
        assert_eq!(returning("int StartingConditional() { return 1; }", "x"), None);

        let mut r =
            Report { returns: vec!["test_env".into(), "test_ok".into()], ..Default::default() };
        for line in [
            "MG_TEST BEGIN 1 3",
            "MG_TEST RUN test_env",
            "MG_TEST END test_env",
            "MG_TEST RUN test_ok",
            "MG_TEST PASS fine",
            "MG_TEST RETURNED test_ok",
            "MG_TEST END test_ok",
            // (One with a driver of its own way: not held to it.)
            "MG_TEST RUN test_old",
            "MG_TEST END test_old",
        ] {
            r.line(line);
        }
        assert!(r.line("MG_TEST DONE"));
        let errors: Vec<_> = r.tests.iter().map(|t| t.error.as_deref()).collect();
        assert_eq!(errors, [Some("did not run to its end (and made no assertion)"), None, None]);
        assert!(!r.ok() && r.errors() == 1 && r.passed() == 1);
    }

    #[test]
    fn a_run_cut_short_names_what_did_not_run() {
        let mut r = Report::default();
        r.line("MG_TEST RUN test_a");
        r.close(&[key("test_a"), key("test_b")]);
        assert!(!r.ok());
        assert!(r.tests[0].error.as_ref().unwrap().contains("did not run to its end"));
        assert_eq!(r.tests[1].error.as_deref(), Some("did not run"));
        assert!(r.junit("m", 0.0).contains("did not say they were done"));
    }

    #[test]
    fn the_driver_goes_into_the_module_s_load_script() {
        let mut m = Module::new();
        let mut info = mg_gff::Gff::new(*b"IFO ");
        info.root.write(&ifo::MOD_ON_MOD_LOAD, "my_load".parse::<ResRef>().unwrap());
        m.set_info(&info).unwrap();
        m.set(key("test_a"), b"void main() {}".to_vec());
        assert!(!inject(&mut m, &[key("test_a")], 0.0).unwrap().0);
        assert_eq!(m.info().unwrap().root.read(&ifo::MOD_ON_MOD_LOAD).to_string(), DRIVER_NAME);
        let d = String::from_utf8(m.get(&key(DRIVER_NAME)).unwrap().to_vec()).unwrap();
        assert!(d.contains("ExecuteScript(\"my_load\""));
        assert_eq!(m.get(&key(INCLUDE_NAME)).unwrap(), INCLUDE.as_bytes());
        // Again (a copy that has it): the driver does not run itself, and
        // a module's own include is kept.
        m.set(key(INCLUDE_NAME), b"// ours".to_vec());
        assert!(inject(&mut m, &[], 0.0).unwrap().0);
        let d = String::from_utf8(m.get(&key(DRIVER_NAME)).unwrap().to_vec()).unwrap();
        assert!(!d.contains("ExecuteScript(\"mg_test_run\""));
        assert_eq!(m.get(&key(INCLUDE_NAME)).unwrap(), b"// ours");
    }
}
