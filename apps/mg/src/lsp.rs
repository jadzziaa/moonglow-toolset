//! `mg lsp`: an NWScript language server (the Language Server Protocol over
//! standard input and output) for editors such as VS Code, Neovim, Emacs or
//! Helix, with Moonglow's script analysis and the game's own compiler:
//! errors as you type, go to definition, references, rename, hover,
//! completion and the document's outline.
//!
//! Scripts are found in the open documents, then the workspace folder's
//! `.nss` files (any depth: a nasher project's `src` works), then the game's
//! resources (its scripts and `nwscript.nss`). A game script opened by Go to
//! Definition is written to a cache folder, read-only.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use mg_core::ResType;
use mg_resman::{ResKey, ResMan};
use mg_script::Compiler;
use mg_script::analysis::{DeclKind, Declaration, Index, NWSCRIPT};
use serde_json::{Value, json};

/// A message from the client.
fn read_message(input: &mut impl BufRead) -> Result<Option<Value>> {
    let mut length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            length = Some(v.trim().parse::<usize>()?);
        }
    }
    let Some(n) = length else { return Ok(None) };
    let mut body = vec![0; n];
    input.read_exact(&mut body)?;
    Ok(Some(serde_json::from_slice(&body)?))
}

fn write_message(out: &mut impl Write, msg: &Value) -> Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(out, "Content-Length: {}\r\n\r\n", body.len())?;
    out.write_all(&body)?;
    out.flush()?;
    Ok(())
}

/// A `file://` URI's path.
fn uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let mut bytes = Vec::new();
    let mut it = rest.bytes();
    while let Some(b) = it.next() {
        if b == b'%' {
            let hex: Vec<u8> = it.by_ref().take(2).collect();
            let v = u8::from_str_radix(std::str::from_utf8(&hex).ok()?, 16).ok()?;
            bytes.push(v);
        } else {
            bytes.push(b);
        }
    }
    let s = String::from_utf8(bytes).ok()?;
    // file:///C:/x on Windows.
    let s = if cfg!(windows) { s.trim_start_matches('/').to_string() } else { s };
    Some(PathBuf::from(s))
}

/// A path's `file://` URI.
fn path_uri(path: &Path) -> String {
    let s = path.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !s.starts_with('/') {
        out.push('/');
    }
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// A script's name from its path: the file name without `.nss`.
fn script_name(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let stem = name.strip_suffix(".nss").or_else(|| name.strip_suffix(".NSS"))?;
    Some(stem.to_ascii_lowercase())
}

/// A byte offset's LSP position (line, UTF-16 column).
fn position(text: &str, offset: usize) -> Value {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..offset].matches('\n').count();
    let character: usize = text[line_start..offset].chars().map(char::len_utf16).sum();
    json!({ "line": line, "character": character })
}

/// An LSP position's byte offset.
fn offset(text: &str, pos: &Value) -> usize {
    let line = pos["line"].as_u64().unwrap_or(0) as usize;
    let character = pos["character"].as_u64().unwrap_or(0) as usize;
    let start = if line == 0 {
        0
    } else {
        text.match_indices('\n').nth(line - 1).map_or(text.len(), |(i, _)| i + 1)
    };
    let mut units = 0;
    for (i, c) in text[start..].char_indices() {
        if units >= character || c == '\n' {
            return start + i;
        }
        units += c.len_utf16();
    }
    text.len()
}

struct Server {
    root: Option<PathBuf>,
    /// The workspace's scripts by name.
    files: HashMap<String, PathBuf>,
    /// Open documents by name: their URI and text.
    open: HashMap<String, (String, String)>,
    index: Index,
    game: Option<ResMan>,
    /// Where game scripts are written for Go to Definition.
    cache: PathBuf,
}

impl Server {
    fn scan(&mut self) {
        self.files.clear();
        let Some(root) = self.root.clone() else { return };
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            let Ok(rd) = std::fs::read_dir(&dir) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                let hidden = e.file_name().to_string_lossy().starts_with('.');
                if p.is_dir() && !hidden && e.file_name() != "target" {
                    stack.push(p);
                } else if let Some(name) = script_name(&p) {
                    self.files.entry(name).or_insert(p);
                }
            }
        }
    }

    /// A script's text by name.
    fn text(&self, name: &str) -> Option<String> {
        if let Some((_, t)) = self.open.get(name) {
            return Some(t.clone());
        }
        if let Some(p) = self.files.get(name) {
            return std::fs::read(p).ok().map(|b| decode(&b));
        }
        let k = ResKey::parse(name, ResType::NSS)?;
        self.game.as_ref()?.get(&k).ok().map(|d| decode(&d))
    }

    /// A script's URI: open or in the workspace, else a cached copy of the
    /// game's.
    fn uri(&self, name: &str) -> Option<String> {
        if let Some((u, _)) = self.open.get(name) {
            return Some(u.clone());
        }
        if let Some(p) = self.files.get(name) {
            return Some(path_uri(p));
        }
        let text = self.text(name)?;
        let path = self.cache.join(format!("{name}.nss"));
        std::fs::create_dir_all(&self.cache).ok()?;
        if std::fs::read_to_string(&path).ok().as_deref() != Some(text.as_str()) {
            let _ = std::fs::write(&path, &text);
        }
        Some(path_uri(&path))
    }

    fn with_index<T>(
        &mut self,
        f: impl FnOnce(&mut Index, &mut dyn mg_script::analysis::Sources) -> T,
    ) -> T {
        let mut index = std::mem::take(&mut self.index);
        let this = &*self;
        let mut src = |name: &str| this.text(name).map(|t| Arc::from(t.as_str()));
        let out = f(&mut index, &mut src);
        self.index = index;
        out
    }

    fn location(&self, file: &str, span: &std::ops::Range<usize>) -> Option<Value> {
        let text = self.text(file)?;
        Some(json!({
            "uri": self.uri(file)?,
            "range": { "start": position(&text, span.start), "end": position(&text, span.end) },
        }))
    }

    /// The name and byte offset of a request's position.
    fn at(&self, params: &Value) -> Option<(String, usize)> {
        let uri = params["textDocument"]["uri"].as_str()?;
        let name = script_name(&uri_path(uri)?)?;
        let text = self.text(&name)?;
        Some((name.clone(), offset(&text, &params["position"])))
    }

    fn declaration(&mut self, params: &Value) -> Option<Declaration> {
        let (name, at) = self.at(params)?;
        self.with_index(|idx, src| idx.declaration_at(&name, at, src))
    }

    /// Errors in a script: the compiler's first, as a diagnostic.
    fn diagnostics(&self, name: &str) -> Vec<Value> {
        let Some(text) = self.text(name) else { return Vec::new() };
        let mut c = Compiler::new(|n: &str, t: ResType| {
            if t == ResType::NSS
                && let Some(text) = self.text(&n.to_ascii_lowercase())
            {
                return Some(encode(&text));
            }
            let k = ResKey::parse(n, t)?;
            self.game.as_ref()?.get(&k).ok().map(|d| d.into_owned())
        });
        c.set_require_entry_point(false);
        let Err(e) = c.compile(name) else { return Vec::new() };
        let line = e
            .location()
            .filter(|(f, _)| f.trim_end_matches(".nss").eq_ignore_ascii_case(name))
            .map_or(0, |(_, l)| l.saturating_sub(1));
        let end = text.lines().nth(line).map_or(0, |l| l.encode_utf16().count());
        vec![json!({
            "range": { "start": { "line": line, "character": 0 }, "end": { "line": line, "character": end } },
            "severity": 1,
            "source": "nwscript",
            "message": e.message,
        })]
    }

    fn publish(&self, name: &str, out: &mut impl Write) -> Result<()> {
        let Some((uri, _)) = self.open.get(name) else { return Ok(()) };
        write_message(
            out,
            &json!({
                "jsonrpc": "2.0",
                "method": "textDocument/publishDiagnostics",
                "params": { "uri": uri, "diagnostics": self.diagnostics(name) },
            }),
        )
    }

    fn handle(
        &mut self,
        method: &str,
        params: &Value,
        out: &mut impl Write,
    ) -> Result<Option<Value>> {
        Ok(Some(match method {
            "initialize" => {
                self.root = params["rootUri"]
                    .as_str()
                    .or_else(|| params["workspaceFolders"][0]["uri"].as_str())
                    .and_then(uri_path)
                    .or_else(|| params["rootPath"].as_str().map(PathBuf::from));
                self.scan();
                json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 1, "save": true },
                        "definitionProvider": true,
                        "referencesProvider": true,
                        "renameProvider": true,
                        "hoverProvider": true,
                        "documentSymbolProvider": true,
                        "completionProvider": { "triggerCharacters": [] },
                    },
                    "serverInfo": { "name": "mg lsp", "version": env!("CARGO_PKG_VERSION") },
                })
            }
            "shutdown" => Value::Null,
            "textDocument/didOpen" | "textDocument/didChange" | "textDocument/didSave" => {
                let uri = params["textDocument"]["uri"].as_str().unwrap_or_default().to_string();
                let Some(name) = uri_path(&uri).as_deref().and_then(script_name) else {
                    return Ok(None);
                };
                let text = params["textDocument"]["text"]
                    .as_str()
                    .or_else(|| params["contentChanges"].as_array()?.last()?["text"].as_str())
                    .map(str::to_string)
                    .or_else(|| self.open.get(&name).map(|(_, t)| t.clone()))
                    .unwrap_or_default();
                self.index.set(&name, Arc::from(text.as_str()));
                self.open.insert(name.clone(), (uri, text));
                self.publish(&name, out)?;
                return Ok(None);
            }
            "textDocument/didClose" => {
                if let Some(name) = params["textDocument"]["uri"]
                    .as_str()
                    .and_then(uri_path)
                    .as_deref()
                    .and_then(script_name)
                {
                    self.open.remove(&name);
                    self.index.forget(&name);
                }
                return Ok(None);
            }
            "workspace/didChangeWatchedFiles" => {
                self.scan();
                self.index = Index::new();
                return Ok(None);
            }
            "textDocument/definition" => match self.declaration(params) {
                Some(d) => self.location(&d.at.file, &d.at.span).unwrap_or(Value::Null),
                None => Value::Null,
            },
            "textDocument/references" => {
                let Some(d) = self.declaration(params) else { return Ok(Some(Value::Null)) };
                let files: Vec<String> =
                    self.files.keys().chain(self.open.keys()).cloned().collect();
                let refs = self.with_index(|idx, src| idx.references(&d, &files, src));
                let with_decl = params["context"]["includeDeclaration"].as_bool().unwrap_or(true);
                Value::Array(
                    refs.iter()
                        .filter(|r| with_decl || r.file != d.at.file || r.span != d.at.span)
                        .filter_map(|r| self.location(&r.file, &r.span))
                        .collect(),
                )
            }
            "textDocument/rename" => {
                let Some(d) = self.declaration(params) else { return Ok(Some(Value::Null)) };
                let new = params["newName"].as_str().unwrap_or_default().to_string();
                let files: Vec<String> =
                    self.files.keys().chain(self.open.keys()).cloned().collect();
                match self.with_index(|idx, src| idx.rename(&d, &new, &files, src)) {
                    Ok(places) => {
                        let mut changes: serde_json::Map<String, Value> = serde_json::Map::new();
                        for p in places {
                            let (Some(uri), Some(text)) = (self.uri(&p.file), self.text(&p.file))
                            else {
                                continue;
                            };
                            let edit = json!({
                                "range": { "start": position(&text, p.span.start), "end": position(&text, p.span.end) },
                                "newText": new,
                            });
                            changes
                                .entry(uri)
                                .or_insert_with(|| json!([]))
                                .as_array_mut()
                                .expect("array")
                                .push(edit);
                        }
                        json!({ "changes": changes })
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!(e));
                    }
                }
            }
            "textDocument/hover" => match self.declaration(params) {
                Some(d) => {
                    let doc =
                        if d.doc.is_empty() { String::new() } else { format!("\n\n{}", d.doc) };
                    json!({ "contents": { "kind": "markdown", "value": format!("```nwscript\n{}\n```{doc}", d.signature) } })
                }
                None => Value::Null,
            },
            "textDocument/documentSymbol" => {
                let Some(name) = params["textDocument"]["uri"]
                    .as_str()
                    .and_then(uri_path)
                    .as_deref()
                    .and_then(script_name)
                else {
                    return Ok(Some(Value::Null));
                };
                let symbols = self.with_index(|idx, src| idx.symbols(&name, src));
                Value::Array(
                    symbols
                        .iter()
                        .filter_map(|d| {
                            Some(json!({
                                "name": d.name,
                                "kind": kind(d.kind),
                                "location": self.location(&d.at.file, &d.at.span)?,
                                "containerName": d.signature,
                            }))
                        })
                        .collect(),
                )
            }
            "textDocument/completion" => {
                let Some((name, at)) = self.at(params) else { return Ok(Some(Value::Null)) };
                let items = self.with_index(|idx, src| idx.completions(&name, at, src));
                Value::Array(
                    items
                        .into_iter()
                        .map(|d| {
                            json!({
                                "label": d.name,
                                "kind": completion_kind(d.kind),
                                "detail": d.signature,
                                "documentation": d.doc,
                                "sortText": if d.at.file == NWSCRIPT { format!("1{}", d.name) } else { format!("0{}", d.name) },
                            })
                        })
                        .collect(),
                )
            }
            _ => return Ok(None),
        }))
    }
}

fn kind(k: DeclKind) -> u32 {
    match k {
        DeclKind::Function => 12,
        DeclKind::Variable | DeclKind::Local | DeclKind::Parameter => 13,
        DeclKind::Constant => 14,
        DeclKind::Struct => 23,
    }
}

fn completion_kind(k: DeclKind) -> u32 {
    match k {
        DeclKind::Function => 3,
        DeclKind::Variable | DeclKind::Local | DeclKind::Parameter => 6,
        DeclKind::Constant => 21,
        DeclKind::Struct => 22,
    }
}

/// Script text in the game's codepage (Windows-1252).
fn decode(bytes: &[u8]) -> String {
    mg_core::Codepage::WINDOWS_1252.decode(bytes).into_owned()
}

fn encode(text: &str) -> Vec<u8> {
    mg_core::Codepage::WINDOWS_1252
        .encode(text)
        .map_or_else(|| text.as_bytes().to_vec(), |b| b.into_owned())
}

/// Serves the protocol on standard input and output until `exit`.
pub(crate) fn serve(game: Option<ResMan>) -> Result<()> {
    let cache = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("moonglow/scripts");
    let mut server = Server {
        root: None,
        files: HashMap::new(),
        open: HashMap::new(),
        index: Index::new(),
        game,
        cache,
    };
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    while let Some(msg) = read_message(&mut input)? {
        let method = msg["method"].as_str().unwrap_or_default().to_string();
        if method == "exit" {
            break;
        }
        let id = msg.get("id").cloned();
        let result = server.handle(&method, &msg["params"], &mut out);
        let Some(id) = id else { continue };
        let reply = match result {
            Ok(r) => json!({ "jsonrpc": "2.0", "id": id, "result": r.unwrap_or(Value::Null) }),
            Err(e) => {
                json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32803, "message": e.to_string() } })
            }
        };
        write_message(&mut out, &reply)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_and_uris() {
        let t = "ab\nxé𝄞y\n";
        assert_eq!(position(t, 0), json!({"line": 0, "character": 0}));
        let y = t.find('y').unwrap();
        assert_eq!(position(t, y), json!({"line": 1, "character": 4}));
        assert_eq!(offset(t, &json!({"line": 1, "character": 4})), y);
        assert_eq!(offset(t, &json!({"line": 1, "character": 99})), t.len() - 1);
        let p = Path::new("/tmp/my mod/src/a_b.nss");
        assert_eq!(path_uri(p), "file:///tmp/my%20mod/src/a_b.nss");
        assert_eq!(uri_path(&path_uri(p)).unwrap(), p);
        assert_eq!(script_name(p).as_deref(), Some("a_b"));
    }
}
