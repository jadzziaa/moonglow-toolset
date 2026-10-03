--!strict
-- The types of Moonglow's plugin API, version 0.1, for editors that check
-- Luau (luau-lsp): point the alias "moonglow" at this file in a `.luaurc`
-- in or above your plugin's folder, and `require("@moonglow")` is typed
-- and completed:
--
--     { "aliases": { "moonglow": "./types/moonglow" } }
--
-- Moonglow itself never reads this file (it serves "@moonglow" itself):
-- the functions here do nothing.

-- A value with its type, as `mg gff` writes fields: { type = "int", value = 3 }.
export type Value = { type: string, value: any }

-- A struct read with `gff`: its fields by label. Numbers and text are
-- plain; a list is its items from 1; a localized string is its texts by
-- language number, with `strref`.
export type Struct = { [string]: any }

export type Finding = {
    resource: string,
    at: string?,
    message: string,
    severity: ("warning" | "error")?,
}

export type FormField = {
    id: string,
    label: string?,
    type: ("text" | "number" | "check" | "choice")?,
    default: any?,
    min: number?,
    max: number?,
    choices: { string }?,
}

export type Form = { title: string?, fields: { FormField } }

export type Reader = {
    resources: (self: Reader, kind: string?) -> { string },
    has: (self: Reader, name: string) -> boolean,
    gff: (self: Reader, name: string) -> Struct,
    raw: (self: Reader, name: string, path: string?) -> Struct,
    text: (self: Reader, name: string) -> string,
    bytes: (self: Reader, name: string) -> string,
}

-- One of the game's 2DA tables. Rows count from 0, as in the file; a cell
-- that is empty (****) is nil.
export type TwoDa = {
    rows: number,
    columns: { string },
    get: (self: TwoDa, row: number, column: string) -> string?,
}

export type Game = Reader & {
    table: (self: Game, name: string) -> TwoDa,
    string: (self: Game, strref: number) -> string?,
}

export type Edit = {
    set: (self: Edit, name: string, field: string, value: any, type: string?) -> (),
    remove: (self: Edit, name: string, field: string) -> (),
    insert: (self: Edit, name: string, list: string, item: Struct, index: number?) -> number,
    remove_item: (self: Edit, name: string, item: string) -> (),
    write: (self: Edit, name: string, text: string) -> (),
    write_bytes: (self: Edit, name: string, bytes: string) -> (),
    write_gff: (self: Edit, name: string, document: Struct) -> (),
    delete: (self: Edit, name: string) -> (),
}

export type Log = {
    info: (self: Log, text: string) -> (),
    warn: (self: Log, text: string) -> (),
    error: (self: Log, text: string) -> (),
}

export type Ui = {
    message: (self: Ui, text: string) -> (),
    confirm: (self: Ui, text: string) -> boolean,
    form: (self: Ui, form: Form) -> { [string]: any }?,
}

export type Context = {
    module: Reader,
    game: Game,
    edit: Edit,
    log: Log,
    ui: Ui,
    progress: (done: number, total: number, note: string?) -> (),
    plugin: { id: string, name: string, version: string },
}

local mg = {}

mg.api = "0.1"

-- A command's handler may return what Undo calls the change:
-- { label = "..." }, or the text alone.
function mg.command(id: string, handler: (ctx: Context) -> ...any) end
-- A check's handler returns its findings (none: an empty list, or nothing).
function mg.check(id: string, handler: (ctx: Context) -> { Finding }?) end

local function typed(type: string, value: any): Value
    return { type = type, value = value }
end

function mg.byte(value: number): Value return typed("byte", value) end
function mg.char(value: number): Value return typed("char", value) end
function mg.word(value: number): Value return typed("word", value) end
function mg.short(value: number): Value return typed("short", value) end
function mg.dword(value: number): Value return typed("dword", value) end
function mg.int(value: number): Value return typed("int", value) end
function mg.dword64(value: number | string): Value return typed("dword64", value) end
function mg.int64(value: number | string): Value return typed("int64", value) end
function mg.float(value: number): Value return typed("float", value) end
function mg.double(value: number): Value return typed("double", value) end
function mg.string(value: string): Value return typed("cexostring", value) end
function mg.resref(value: string): Value return typed("resref", value) end
-- Text (its English), or its texts by language number, with `strref`.
function mg.locstring(value: string | { [any]: any }): Value return typed("cexolocstring", value) end

-- Where a struct read with `gff` is, for the edits: "/ClassList[0]" ("" for
-- the root).
function mg.path(struct: Struct): string? return nil end

return mg
