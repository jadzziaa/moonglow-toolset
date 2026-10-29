//! `mg lsp` over the protocol: a workspace with an include and a script
//! that uses it; definition, references, rename, hover, outline and
//! completion, and (with the game) errors as you type.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::Stdio;

use serde_json::{Value, json};

struct Client {
    child: std::process::Child,
    out: BufReader<std::process::ChildStdout>,
    next: u64,
    notes: Vec<Value>,
}

impl Client {
    fn send(&mut self, msg: &Value) {
        let body = serde_json::to_vec(msg).unwrap();
        let stdin = self.child.stdin.as_mut().unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n", body.len()).unwrap();
        stdin.write_all(&body).unwrap();
        stdin.flush().unwrap();
    }

    fn read(&mut self) -> Value {
        let mut length = 0;
        loop {
            let mut line = String::new();
            self.out.read_line(&mut line).unwrap();
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.strip_prefix("Content-Length:") {
                length = v.trim().parse().unwrap();
            }
        }
        let mut body = vec![0; length];
        self.out.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let m = self.read();
            if m["id"] == id {
                return m;
            }
            self.notes.push(m);
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }
}

/// A path's URI, as the server writes them (`file:///C:/...` on Windows).
fn uri(p: &std::path::Path) -> String {
    let s = p.display().to_string().replace('\\', "/");
    if s.starts_with('/') { format!("file://{s}") } else { format!("file:///{s}") }
}

#[test]
fn the_language_server_answers() {
    let dir = mg_testkit::scratch_dir("mg-lsp");
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let inc = src.join("inc_util.nss");
    let main = src.join("main_a.nss");
    let util = "// Doubles a number.\nint Twice(int n) { return n * 2; }\n\
                struct Pt {\n    // * Across.\n    int nX;\n};\n";
    std::fs::write(&inc, util).unwrap();
    let text = "#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2);\n    struct Pt p; p.nX = 1;\n}\n";
    std::fs::write(&main, text).unwrap();
    let mut child = crate::mg()
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let out = BufReader::new(child.stdout.take().unwrap());
    let mut c = Client { child, out, next: 0, notes: Vec::new() };

    let init = c.request("initialize", json!({ "rootUri": uri(&dir), "capabilities": {} }));
    assert_eq!(init["result"]["capabilities"]["definitionProvider"], true);
    c.notify("initialized", json!({}));
    c.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&main), "languageId": "nwscript", "version": 1, "text": text } }),
    );
    let twice = json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 3, "character": 14 } });

    // Definition: in the include, which isn't open.
    let def = c.request("textDocument/definition", twice.clone());
    assert_eq!(def["result"]["uri"], uri(&inc), "{def}");
    assert_eq!(def["result"]["range"]["start"], json!({ "line": 1, "character": 4 }));

    // Hover: the signature and its comment.
    let hover = c.request("textDocument/hover", twice.clone());
    let value = hover["result"]["contents"]["value"].as_str().unwrap();
    assert!(value.contains("int Twice(int n)") && value.contains("Doubles a number."), "{value}");
    // A struct's member, with its comment (BioWare's `// *` without the
    // `*`).
    let member = json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 4, "character": 19 } });
    let hover = c.request("textDocument/hover", member);
    let value = hover["result"]["contents"]["value"].as_str().unwrap_or_default();
    assert!(value.contains("int nX (struct Pt)") && value.ends_with("\n\nAcross."), "{hover}");

    // References: the definition and the use.
    let mut params = twice.clone();
    params["context"] = json!({ "includeDeclaration": true });
    let refs = c.request("textDocument/references", params);
    assert_eq!(refs["result"].as_array().unwrap().len(), 2, "{refs}");

    // Rename: edits in both files.
    let mut params = twice.clone();
    params["newName"] = json!("Double");
    let rename = c.request("textDocument/rename", params);
    let changes = rename["result"]["changes"].as_object().unwrap();
    assert_eq!(changes.len(), 2, "{rename}");
    assert_eq!(changes[&uri(&main)][0]["newText"], "Double");

    // Outline and completion.
    let symbols =
        c.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri(&inc) } }));
    assert_eq!(symbols["result"][0]["name"], "Twice");
    let completion = c.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 3, "character": 21 } }),
    );
    let labels: Vec<&str> = completion["result"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|i| i["label"].as_str())
        .collect();
    assert!(labels.contains(&"x") && labels.contains(&"Twice"), "{labels:?}");

    // Errors as you type (with the game, whose nwscript.nss the compiler needs).
    if mg_testkit::nwn_root().is_some() {
        c.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri(&main), "version": 2 },
                "contentChanges": [{ "text": "#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2) +;\n}\n" }],
            }),
        );
        // A request after the change: its diagnostics arrive first.
        c.request("textDocument/hover", twice);
        let diag = c
            .notes
            .iter()
            .rev()
            .find(|n| n["method"] == "textDocument/publishDiagnostics")
            .expect("diagnostics");
        assert_eq!(diag["params"]["diagnostics"][0]["range"]["start"]["line"], 3, "{diag}");
    }
    c.request("shutdown", Value::Null);
    c.notify("exit", Value::Null);
    assert!(c.child.wait().unwrap().success());
}

/// Hover over a function of an include saved in Windows-1250 (as Polish
/// builders keep their scripts), from a server started with `args` and
/// the user directory `user`: its comment as the server read it.
fn polish_hover(name: &str, user: Option<&std::path::Path>, args: &[&str]) -> String {
    let comment = "// Zażółć gęślą jaźń. Źródło.\n";
    let bytes = mg_core::Codepage::WINDOWS_1250.encode(comment).unwrap().into_owned();
    let mut command = crate::mg();
    match user {
        Some(u) => command.args(["--user-dir", u.to_str().unwrap()]),
        None => command.arg("--no-user-dir"),
    };
    hover_of(name, &bytes, command, args)
}

/// Hover over a function of an include whose comment is `comment` (its
/// bytes), from `command lsp args`: the comment as the server read it.
fn hover_of(
    name: &str,
    comment: &[u8],
    mut command: std::process::Command,
    args: &[&str],
) -> String {
    let dir = mg_testkit::scratch_dir(name);
    let src = dir.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let inc = src.join("inc_pl.nss");
    let main = src.join("main_pl.nss");
    let mut bytes = comment.to_vec();
    bytes.extend_from_slice(b"int Twice(int n) { return n * 2; }\n");
    std::fs::write(&inc, bytes).unwrap();
    let text = "#include \"inc_pl\"\nvoid main()\n{\n    int x = Twice(2);\n}\n";
    std::fs::write(&main, text).unwrap();
    let mut child = command
        .arg("lsp")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let out = BufReader::new(child.stdout.take().unwrap());
    let mut c = Client { child, out, next: 0, notes: Vec::new() };
    c.request("initialize", json!({ "rootUri": uri(&dir), "capabilities": {} }));
    c.notify("initialized", json!({}));
    c.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&main), "languageId": "nwscript", "version": 1, "text": text } }),
    );
    let at = json!({ "textDocument": { "uri": uri(&main) }, "position": { "line": 3, "character": 14 } });
    let hover = c.request("textDocument/hover", at);
    c.request("shutdown", Value::Null);
    c.notify("exit", Value::Null);
    assert!(c.child.wait().unwrap().success());
    hover["result"]["contents"]["value"].as_str().unwrap_or_default().to_string()
}

/// A script's text is read in the game's codepage (GitHub issue 27): the
/// one the user directory's settings.tml gives the game (its language's,
/// Polish here), or the one named; Windows-1252 without either.
#[test]
fn the_language_server_reads_scripts_in_the_game_s_codepage() {
    let polish = "Zażółć gęślą jaźń. Źródło.";
    let user = mg_testkit::scratch_dir("mg-lsp-pl-user");
    let settings = "[game]\n\t[game.language]\n\t\tcodepage = \"\"\n\t\toverride = \"pl\"\n";
    std::fs::write(user.join("settings.tml"), settings).unwrap();
    let by_settings = polish_hover("mg-lsp-pl-settings", Some(&user), &[]);
    assert!(by_settings.contains(polish), "{by_settings}");
    let named = polish_hover("mg-lsp-pl-named", None, &["--codepage", "cp1250"]);
    assert!(named.contains(polish), "{named}");
    let western = polish_hover("mg-lsp-pl-western", None, &[]);
    assert!(!western.contains(polish) && western.contains("Twice"), "{western}");
}

/// Given the module the scripts are of, the language server reads them
/// in the table of its `encoding.2da` (in its hak): byte 0xF0 is "ğ" there.
#[test]
fn the_language_server_reads_scripts_by_a_module_s_table() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-lsp-table");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let mut rows = String::from("2DA V2.0\n\n  Codepoint\n");
    for row in 0..240 {
        rows.push_str(&format!("{row} ****\n"));
    }
    rows.push_str("240 0x11f\n");
    let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
    let table = mg_resman::ResKey::from_filename("encoding.2da").unwrap();
    hak.add(table.resref, table.restype, rows.into_bytes()).unwrap();
    std::fs::write(user.join("hak/mg_turkish.hak"), hak.to_bytes().unwrap()).unwrap();
    let mut m = mg_module::Module::new();
    let mut info = mg_gff::Gff::new(*b"IFO ");
    let mut listed = mg_gff::Struct::new(8);
    listed.set("Mod_Hak", mg_gff::Value::String(b"mg_turkish".to_vec()));
    info.root.set("Mod_HakList", mg_gff::Value::List(vec![listed]));
    m.set_info(&info).unwrap();
    let path = dir.join("turkish.mod");
    m.save_as(&mg_module::ModuleLocation::Archive(path.clone())).unwrap();
    let comment = b"// Da\xf0 Evi.\n";
    let by_table = hover_of(
        "mg-lsp-table-module",
        comment,
        crate::mg_on(&root, Some(&user)),
        &["--module", path.to_str().unwrap()],
    );
    assert!(by_table.contains("Dağ Evi"), "{by_table}");
    let without = hover_of("mg-lsp-table-none", comment, crate::mg_on(&root, Some(&user)), &[]);
    assert!(without.contains("Dað Evi"), "{without}");
}
