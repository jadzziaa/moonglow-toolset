//! `mg lsp` over the protocol: a workspace with an include and a script
//! that uses it; definition, references, rename, hover, outline and
//! completion, and (with the game) errors as you type.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};

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
    std::fs::write(&inc, "// Doubles a number.\nint Twice(int n) { return n * 2; }\n").unwrap();
    let text = "#include \"inc_util\"\nvoid main()\n{\n    int x = Twice(2);\n}\n";
    std::fs::write(&main, text).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_mg"))
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
