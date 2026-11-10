//! What API 0.6 added a plugin can be: an exporter, an importer, a wizard
//! and a panel, each run as a job as a command is (`docs/plugin-proposal.md`,
//! "API 0.6").
//!
//! - An exporter reads the module and writes only where the user chose
//!   (`out`), the files handed back as a folder's to write.
//! - An importer reads what the user chose, a file or a folder, and edits.
//! - A wizard's pages are asked one at a time ([`Question::Page`]); they
//!   only read, and its `finish` edits.
//! - A panel's handler returns what the panel shows, as data ([`Shown`]),
//!   which the window draws; it only reads, within [`PANEL_BUDGET`]. Its
//!   fields (API 0.9) are a form's, whose values the window keeps and
//!   hands to the commands it runs (`ctx.values`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mlua::{Function, LuaSerdeExt, Table, Value as LuaValue};

use crate::runtime::{
    Shared, This, about, context, fail, failed, form_field, form_fields, handler, shared, start,
};
use crate::{
    Answer, ExporterDecl, FieldKind, FolderWrite, FormField, Host, ImporterDecl, Input, Level,
    MAX_FILE, Outcome, Page, PanelDecl, Plugin, PluginError, Question, Selected, WizardDecl,
    is_file_name,
};

/// How long a panel's handler may run before it is stopped: it runs
/// beside the window, which shows what it last returned meanwhile.
pub const PANEL_BUDGET: std::time::Duration = std::time::Duration::from_millis(250);

/// The most elements (and a list's items, each counted) a panel shows;
/// those past it are cut, and the panel says so.
pub const MAX_ELEMENTS: usize = 2000;

/// The most characters of a panel's text; the rest is cut. And of a
/// choice's option, which is refused past it (its text is its value).
pub const MAX_TEXT: usize = 1000;

/// The most options a choice (a form's field, a panel's) has; one of more
/// is refused.
pub const MAX_CHOICES: usize = MAX_ELEMENTS;

/// What a handler returned to call its change by (`{ label = … }`, or the
/// text alone).
fn label_of(made: LuaValue) -> mlua::Result<Option<String>> {
    Ok(match made {
        LuaValue::Table(t) => t.get::<Option<String>>("label")?,
        LuaValue::String(s) => Some(s.to_str()?.to_string()),
        _ => None,
    })
}

/// A title as a step of Undo or the log names it: without its `…`.
fn plain(title: &str) -> &str {
    title.trim_end_matches('…').trim_end_matches("...").trim()
}

/// Whether the job wrote or edited anything: what one that only reads may
/// not do.
fn changed(sh: &Shared) -> bool {
    !sh.edits.borrow().is_empty() || !sh.haks.borrow().is_empty() || !sh.folders.borrow().is_empty()
}

/// The outcome of a job that edits: its edits and what it wrote outside
/// the module.
fn outcome(sh: &Shared, label: String) -> Outcome {
    Outcome {
        label,
        edits: sh.edits.take(),
        haks: sh.haks.take(),
        folders: sh.folders.take(),
        settings: sh.settings_set.take(),
        said: Vec::new(),
    }
}

/// Files by name, and their bytes.
type Files = Vec<(String, Vec<u8>)>;

pub(crate) fn run_export(
    plugin: &Plugin,
    decl: &ExporterDecl,
    to: &Path,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let name = to.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let refused =
        |message: String| PluginError::Script { plugin: plugin.manifest.name.clone(), message };
    if !decl.folder && !is_file_name(&name) {
        return Err(refused(format!("{:?} is not a file's name to export to", to.display())));
    }
    let sh = shared(plugin, input, &host);
    let files: Rc<RefCell<Files>> = Rc::default();
    let run =
        || -> mlua::Result<Option<String>> {
            let lua = start(Some(&plugin.manifest.entry), &sh)?;
            let f = handler(&lua, "mg.exporters", "exporter", &decl.id)?;
            let ctx = context(&lua, &sh, about(plugin))?;
            let out = lua.create_table()?;
            out.set("name", name.as_str())?;
            let kept = files.clone();
            let held = sh.budget_held.clone();
            if decl.folder {
                out.set(
                "write",
                lua.create_function(move |_, (_, file, bytes): (This, String, mlua::LuaString)| {
                    if !is_file_name(&file) {
                        return fail(format!(
                            "out:write({file:?}, …): a file's name alone (no folders, nothing \
                             hidden)"
                        ));
                    }
                    if bytes.as_bytes().len() as u64 > MAX_FILE {
                        return fail(format!("{file} is too large to write"));
                    }
                    held.keep(bytes.as_bytes().len())?;
                    let bytes = bytes.as_bytes().to_vec();
                    let mut all = kept.borrow_mut();
                    match all.iter_mut().find(|(n, _)| *n == file) {
                        Some(old) => {
                            held.give_back(old.1.len());
                            old.1 = bytes;
                        }
                        None => all.push((file, bytes)),
                    }
                    Ok(())
                })?,
            )?;
            } else {
                let file = name.clone();
                out.set(
                    "write",
                    lua.create_function(move |_, (_, bytes): (This, mlua::LuaString)| {
                        if bytes.as_bytes().len() as u64 > MAX_FILE {
                            return fail(format!("{file} is too large to write"));
                        }
                        held.keep(bytes.as_bytes().len())?;
                        let had = kept.replace(vec![(file.clone(), bytes.as_bytes().to_vec())]);
                        held.give_back(had.iter().map(|(_, b)| b.len()).sum());
                        Ok(())
                    })?,
                )?;
            }
            label_of(f.call::<LuaValue>((ctx, out))?)
        };
    let said = run().map_err(|e| failed(plugin, &sh, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    if changed(&sh) {
        return Err(refused(format!(
            "the exporter {:?} changes the module or writes elsewhere: an exporter only reads, \
             and writes where the user chose (out:write)",
            decl.id
        )));
    }
    if let Some(text) = said {
        host.log(Level::Info, &text);
    }
    let dir = match decl.folder {
        true => to.to_path_buf(),
        false => (to.parent().filter(|p| !p.as_os_str().is_empty()))
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf),
    };
    let files = files.take();
    let mut done = outcome(&sh, plain(&decl.title).to_string());
    if !files.is_empty() {
        done.folders.push(FolderWrite { dir, files });
    }
    Ok(done)
}

pub(crate) fn run_import(
    plugin: &Plugin,
    decl: &ImporterDecl,
    from: &Path,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let refused =
        |message: String| PluginError::Script { plugin: plugin.manifest.name.clone(), message };
    match decl.folder {
        true if !from.is_dir() => {
            return Err(refused(format!("{} is not a folder", from.display())));
        }
        false if !from.is_file() => {
            return Err(refused(format!("{} is not a file", from.display())));
        }
        _ => {}
    }
    let extension = from.extension().map(|e| e.to_string_lossy().to_ascii_lowercase());
    if !decl.folder
        && !decl.extensions.is_empty()
        && !extension.as_ref().is_some_and(|e| decl.extensions.contains(e))
    {
        let kinds: Vec<String> = decl.extensions.iter().map(|e| format!(".{e}")).collect();
        return Err(refused(format!(
            "{}: {} reads {}",
            from.display(),
            plain(&decl.title),
            kinds.join(", ")
        )));
    }
    let sh = shared(plugin, input, &host);
    let run = || -> mlua::Result<Option<String>> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let f = handler(&lua, "mg.importers", "importer", &decl.id)?;
        let ctx = context(&lua, &sh, about(plugin))?;
        let source = match decl.folder {
            true => crate::files::folder_table(&lua, &sh, from.to_path_buf())?,
            false => crate::files::file_table(&lua, from)?,
        };
        label_of(f.call::<LuaValue>((ctx, source))?)
    };
    let label = run().map_err(|e| failed(plugin, &sh, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    let label = label.unwrap_or_else(|| format!("Import {}", plain(&decl.title)));
    Ok(outcome(&sh, label))
}

/// A form field's default set to a value given before (going back, or
/// asked again).
fn default_to(kind: &mut FieldKind, v: &serde_json::Value) {
    match kind {
        FieldKind::Text { default } => {
            if let Some(t) = v.as_str() {
                *default = t.to_string();
            }
        }
        FieldKind::Number { default, .. } => {
            if let Some(n) = v.as_f64() {
                *default = n;
            }
        }
        FieldKind::Check { default } => {
            if let Some(b) = v.as_bool() {
                *default = b;
            }
        }
        FieldKind::Choice { choices, default } => {
            if let Some(i) = v.as_str().and_then(|t| choices.iter().position(|c| c.text == t)) {
                *default = i;
            }
        }
        FieldKind::Color { default, .. } => {
            if let Some(c) = v.as_str().and_then(crate::color_of) {
                *default = c;
            }
        }
    }
}

pub(crate) fn run_wizard(
    plugin: &Plugin,
    decl: &WizardDecl,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Outcome, PluginError> {
    let sh = shared(plugin, input, &host);
    let canceled = std::cell::Cell::new(false);
    let reads_only = |what: &str| -> mlua::Result<()> {
        if changed(&sh) {
            return fail(format!(
                "{what} changes the module or writes outside it: a wizard's pages only read, \
                 and it makes its edits in finish"
            ));
        }
        Ok(())
    };
    let run = || -> mlua::Result<Option<String>> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let wizards: Table = lua.named_registry_value("mg.wizards")?;
        let Some(spec) = wizards.get::<Option<Table>>(decl.id.as_str())? else {
            return fail(format!(
                "the manifest declares the wizard {:?}, but the code registers none \
                 (mg.wizard({:?}, …))",
                decl.id, decl.id
            ));
        };
        let ctx = context(&lua, &sh, about(plugin))?;
        let pages: Vec<LuaValue> = (spec.get::<Table>("pages")?.sequence_values::<LuaValue>())
            .collect::<mlua::Result<_>>()?;
        if pages.is_empty() {
            return fail(format!("the wizard {:?} has no pages", decl.id));
        }
        let finish: Function = spec.get("finish")?;
        let answers = lua.create_table()?;
        // (Each field's page: a field's id is its own across the pages.)
        let mut owner: BTreeMap<String, String> = BTreeMap::new();
        // The pages shown, to go back through.
        let mut shown: Vec<usize> = Vec::new();
        let mut i = 0;
        'pages: while i < pages.len() {
            let page = match &pages[i] {
                LuaValue::Table(t) => Some(t.clone()),
                LuaValue::Function(f) => {
                    let page: Option<Table> = f.call((ctx.clone(), answers.clone()))?;
                    reads_only(&format!("the wizard's page {}", i + 1))?;
                    page
                }
                _ => {
                    return fail(format!(
                        "the wizard's page {}: a table, or a function that makes one",
                        i + 1
                    ));
                }
            };
            // (Skipped: its function made none.)
            let Some(page) = page else {
                i += 1;
                continue;
            };
            let id = page.get::<Option<String>>("id")?.unwrap_or_else(|| format!("{}", i + 1));
            let title = page.get::<Option<String>>("title")?.unwrap_or_default();
            let text = page.get::<Option<String>>("text")?.unwrap_or_default();
            let fields = match page.get::<Option<Table>>("fields")? {
                Some(f) => form_fields(f)?,
                None => Vec::new(),
            };
            for f in &fields {
                if let Some(other) = owner.get(&f.id)
                    && *other != id
                {
                    return fail(format!(
                        "the field {:?} is on the pages {other:?} and {id:?}: a wizard's \
                         answers are by field id, each its own",
                        f.id
                    ));
                }
                owner.insert(f.id.clone(), id.clone());
            }
            let check: Option<Function> = page.get("check")?;
            let mut error = None;
            loop {
                // What was given before is what the page starts with.
                let mut fields = fields.clone();
                for f in &mut fields {
                    let v: LuaValue = answers.get(f.id.as_str())?;
                    if !v.is_nil() {
                        default_to(&mut f.kind, &lua.from_value::<serde_json::Value>(v)?);
                    }
                }
                let asked = Question::Page(Page {
                    wizard: decl.title.clone(),
                    title: title.clone(),
                    text: text.clone(),
                    fields,
                    step: i + 1,
                    steps: pages.len(),
                    back: !shown.is_empty(),
                    last: i + 1 == pages.len(),
                    error: error.take(),
                });
                let (values, back) = match sh.host.ask(&asked) {
                    Some(Answer::Page { values, back }) => (values, back),
                    Some(Answer::Values(values)) => (values, false),
                    _ => {
                        canceled.set(true);
                        return fail("canceled");
                    }
                };
                for (k, v) in values {
                    answers.set(k, lua.to_value(&v)?)?;
                }
                if back {
                    i = shown.pop().unwrap_or(0);
                    continue 'pages;
                }
                if let Some(check) = &check {
                    let said: Option<String> = check.call((ctx.clone(), answers.clone()))?;
                    reads_only(&format!("the check of the wizard's page {id:?}"))?;
                    if let Some(said) = said.filter(|s| !s.trim().is_empty()) {
                        // (Nobody to change it: that is the end.)
                        if !sh.host.interactive() {
                            let page = if title.is_empty() { &id } else { &title };
                            return fail(format!("{page}: {said}"));
                        }
                        error = Some(said);
                        continue;
                    }
                }
                shown.push(i);
                i += 1;
                break;
            }
        }
        label_of(finish.call::<LuaValue>((ctx, answers))?)
    };
    let label = run().map_err(|e| match canceled.get() {
        true => PluginError::Canceled(plugin.manifest.name.clone()),
        false => failed(plugin, &sh, e),
    })?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    let label = label.unwrap_or_else(|| plain(&decl.title).to_string());
    Ok(outcome(&sh, label))
}

/// How a panel's text is shown.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Tone {
    #[default]
    Plain,
    /// Of less weight (`weak = true`).
    Weak,
    Warning,
    Error,
}

/// Something a panel shows.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Heading(String),
    Text {
        text: String,
        tone: Tone,
    },
    /// A value beside what it is (`Areas: 12`).
    Value {
        label: String,
        text: String,
    },
    Separator,
    /// A button that runs one of the plugin's commands, on `on` (none:
    /// on what is selected).
    Button {
        text: String,
        hint: String,
        command: String,
        on: Vec<Selected>,
    },
    List(Vec<Item>),
    /// A field, as a form's (API 0.9): its value what the handler gave
    /// (the field's `default`), which the window keeps, by the field's
    /// id, until the handler gives another; `command`, one of the
    /// plugin's commands, run when the user sets it, with the panel's
    /// values (`ctx.values`).
    Field {
        field: FormField,
        command: Option<String>,
    },
}

/// An item of a panel's list: a click opens `open`, or runs `command` on
/// `on`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub text: String,
    pub hint: String,
    pub open: Option<Selected>,
    pub command: Option<String>,
    pub on: Vec<Selected>,
}

/// What a panel shows: its elements, and how many it returned past
/// [`MAX_ELEMENTS`] (cut) and whether texts were cut short.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Shown {
    pub elements: Vec<Element>,
    pub cut: usize,
    pub shortened: bool,
}

impl Shown {
    /// Every text it shows (a heading's, a value's label and text, a
    /// button's, an item's, a field's label and the value it was given),
    /// for a plugin's test.
    pub fn texts(&self) -> Vec<String> {
        let mut out = Vec::new();
        for e in &self.elements {
            match e {
                Element::Heading(t) | Element::Text { text: t, .. } => out.push(t.clone()),
                Element::Value { label, text } => out.push(format!("{label}: {text}")),
                Element::Button { text, .. } => out.push(text.clone()),
                Element::List(items) => out.extend(items.iter().map(|i| i.text.clone())),
                Element::Field { field, .. } => out.push(format!(
                    "{}: {}",
                    field.label,
                    value_text(&field.kind.default_value())
                )),
                Element::Separator => {}
            }
        }
        out
    }
}

/// A field's value as text: text as it is, a number (a whole one without
/// its `.0`), `true` or `false`.
pub fn value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => match n.as_f64() {
            Some(f) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
            _ => n.to_string(),
        },
        other => other.to_string(),
    }
}

/// An entry as a panel names one (`on`, `open`): `{ resource, path }` as
/// `ctx.selection`'s, or its text (`"tavern.are"`, `"tavern.git/Creature
/// List[0]"`).
fn entry(v: &LuaValue) -> mlua::Result<Selected> {
    let text = match v {
        LuaValue::String(s) => s.to_str()?.to_string(),
        LuaValue::Table(t) => {
            let resource: String = t.get("resource").or_else(|_| {
                fail("an entry is { resource = \"name.ext\", path = \"…\" }, as ctx.selection's")
            })?;
            let path: Option<String> = t.get("path")?;
            format!("{resource}{}", path.unwrap_or_default())
        }
        _ => return fail("an entry is a resource's name, or { resource, path }"),
    };
    Selected::parse(&text).or_else(fail)
}

/// `on`: one entry, or a list of them.
fn entries(v: LuaValue) -> mlua::Result<Vec<Selected>> {
    match &v {
        LuaValue::Nil => Ok(Vec::new()),
        LuaValue::Table(t) if t.contains_key("resource")? => Ok(vec![entry(&v)?]),
        LuaValue::Table(t) => t.sequence_values::<LuaValue>().map(|e| entry(&e?)).collect(),
        _ => Ok(vec![entry(&v)?]),
    }
}

/// What a panel's handler returned, read: its elements, held to the
/// limits and to the plugin's commands.
fn elements(list: &Table, manifest: &crate::Manifest) -> mlua::Result<Shown> {
    let mut shown = Shown::default();
    let mut count = 0;
    let text = |v: Option<String>, shown: &mut Shown| -> String {
        let t = v.unwrap_or_default();
        match t.char_indices().nth(MAX_TEXT) {
            Some((at, _)) => {
                shown.shortened = true;
                format!("{}…", &t[..at])
            }
            None => t,
        }
    };
    let command = |c: Option<String>| -> mlua::Result<Option<String>> {
        match c {
            Some(c) if !manifest.commands.iter().any(|d| d.id == c) => fail(format!(
                "a panel's button (or field) runs one of the plugin's commands: the manifest \
                 declares no command {c:?}"
            )),
            c => Ok(c),
        }
    };
    // (Each field's id, its value's key: its own in the panel.)
    let mut ids: Vec<String> = Vec::new();
    for value in list.sequence_values::<LuaValue>() {
        let value = value?;
        if count >= MAX_ELEMENTS {
            shown.cut += 1;
            continue;
        }
        count += 1;
        let t = match value {
            LuaValue::String(s) => {
                let t = text(Some(s.to_str()?.to_string()), &mut shown);
                shown.elements.push(Element::Text { text: t, tone: Tone::Plain });
                continue;
            }
            LuaValue::Table(t) => t,
            _ => return fail("a panel's element is a table ({ type = \"text\", … }) or text"),
        };
        let kind = t.get::<Option<String>>("type")?.unwrap_or_else(|| "text".into());
        let element = match kind.as_str() {
            "heading" => Element::Heading(text(t.get("text")?, &mut shown)),
            "text" => {
                let tone = match (t.get::<Option<String>>("color")?.as_deref(), t.get("weak")?) {
                    (Some("warning"), _) => Tone::Warning,
                    (Some("error"), _) => Tone::Error,
                    (Some(other), _) => {
                        return fail(format!(
                            "a text's color is \"warning\" or \"error\", not {other:?}"
                        ));
                    }
                    (None, Some(true)) => Tone::Weak,
                    (None, _) => Tone::Plain,
                };
                Element::Text { text: text(t.get("text")?, &mut shown), tone }
            }
            "value" => Element::Value {
                label: text(t.get("label")?, &mut shown),
                text: text(t.get("text")?, &mut shown),
            },
            "separator" => Element::Separator,
            "field" => {
                if t.contains_key("default")? {
                    return fail(
                        "a panel's field gives what it shows as its value (value = …), not \
                         default: the field takes it each time the panel gives another",
                    );
                }
                let Some(kind) = t.get::<Option<String>>("kind")? else {
                    return fail(
                        "a panel's field says what it takes: { type = \"field\", kind = \
                         \"text\" (or \"number\", \"check\", \"choice\", \"color\"), … }",
                    );
                };
                let mut field = form_field(&t, &kind, "value")?;
                if ids.contains(&field.id) {
                    return fail(format!("a panel has two fields {:?}", field.id));
                }
                ids.push(field.id.clone());
                field.label = text(Some(field.label), &mut shown);
                if let FieldKind::Text { default } = &mut field.kind {
                    *default = text(Some(std::mem::take(default)), &mut shown);
                }
                Element::Field { field, command: command(t.get("command")?)? }
            }
            "button" => {
                let Some(c) = command(t.get("command")?)? else {
                    return fail("a panel's button runs a command: { command = \"…\" }");
                };
                Element::Button {
                    text: text(t.get("text")?, &mut shown),
                    hint: text(t.get("hint")?, &mut shown),
                    command: c,
                    on: entries(t.get("on")?)?,
                }
            }
            "list" => {
                let mut items = Vec::new();
                let given: Vec<LuaValue> = match t.get::<Option<Table>>("items")? {
                    Some(g) => g.sequence_values::<LuaValue>().collect::<mlua::Result<_>>()?,
                    None => Vec::new(),
                };
                for item in given {
                    if count >= MAX_ELEMENTS {
                        shown.cut += 1;
                        continue;
                    }
                    count += 1;
                    items.push(match item {
                        LuaValue::String(s) => Item {
                            text: text(Some(s.to_str()?.to_string()), &mut shown),
                            hint: String::new(),
                            open: None,
                            command: None,
                            on: Vec::new(),
                        },
                        LuaValue::Table(i) => {
                            let open: LuaValue = i.get("open")?;
                            Item {
                                text: text(i.get("text")?, &mut shown),
                                hint: text(i.get("hint")?, &mut shown),
                                open: (!open.is_nil()).then(|| entry(&open)).transpose()?,
                                command: command(i.get("command")?)?,
                                on: entries(i.get("on")?)?,
                            }
                        }
                        _ => return fail("a list's item is text, or { text, hint, open, … }"),
                    });
                }
                Element::List(items)
            }
            other => {
                return fail(format!(
                    "a panel's element is a heading, text, value, separator, button, list or \
                     field, not {other:?}"
                ));
            }
        };
        shown.elements.push(element);
    }
    Ok(shown)
}

pub(crate) fn run_panel(
    plugin: &Plugin,
    decl: &PanelDecl,
    input: Input,
    host: Rc<dyn Host>,
) -> Result<Shown, PluginError> {
    let input = Input { budget: input.budget.or(Some(PANEL_BUDGET)), ..input };
    let sh = shared(plugin, input, &host);
    sh.no_set.set(Some("a panel only reads: ctx.settings:set is for commands"));
    let run = || -> mlua::Result<Shown> {
        let lua = start(Some(&plugin.manifest.entry), &sh)?;
        let f = handler(&lua, "mg.panels", "panel", &decl.id)?;
        let ctx = context(&lua, &sh, about(plugin))?;
        match f.call::<LuaValue>(ctx)? {
            LuaValue::Nil => Ok(Shown::default()),
            LuaValue::Table(t) => elements(&t, &plugin.manifest),
            _ => fail("a panel returns a list of what it shows"),
        }
    };
    let shown = run().map_err(|e| failed(plugin, &sh, e))?;
    if host.cancelled() {
        return Err(PluginError::Canceled(plugin.manifest.name.clone()));
    }
    if changed(&sh) {
        return Err(PluginError::Script {
            plugin: plugin.manifest.name.clone(),
            message: format!(
                "the panel {:?} changes the module or writes outside it: a panel only reads \
                 (its buttons run commands, which edit)",
                decl.id
            ),
        });
    }
    Ok(shown)
}

/// The names of what an exporter's handler gets, as the reference writes
/// them.
pub(crate) const NAMES: [&str; 2] = ["out.name", "out:write"];

#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    #[test]
    fn a_title_loses_its_ellipsis() {
        assert_eq!(plain("Creatures as CSV…"), "Creatures as CSV");
        assert_eq!(plain("Shop Wizard..."), "Shop Wizard");
    }

    #[test]
    fn an_entry_is_named_as_a_selection_is() {
        let lua = Lua::new();
        let t = lua.create_table().unwrap();
        t.set("resource", "tavern.git").unwrap();
        t.set("path", "/Creature List[2]").unwrap();
        let s = entry(&LuaValue::Table(t)).unwrap();
        assert_eq!(s.kind(), "creature");
        let s = entry(&LuaValue::String(lua.create_string("guard.utc").unwrap())).unwrap();
        assert_eq!(s, Selected::parse("guard.utc").unwrap());
        assert!(entry(&LuaValue::Boolean(true)).is_err());
    }
}
