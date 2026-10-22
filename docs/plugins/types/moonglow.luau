--!strict
-- The types of Moonglow's plugin API, version 0.4, for editors that check
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
    -- Its fix, made with ctx.fix (API 0.4).
    fix: Fix?,
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
    set: (self: TwoDa, row: number, column: string, value: (string | number)?) -> (),
    add_row: (self: TwoDa, cells: { [string]: string | number }?) -> number,
    add_column: (self: TwoDa, name: string) -> (),
    write: (self: TwoDa) -> string,
}

export type TlkLine = { text: string, sound: string?, sound_length: number? }

export type Tlk = {
    lines: number,
    get: (self: Tlk, line: number) -> TlkLine?,
    set: (self: Tlk, line: number, value: (string | TlkLine)?) -> (),
    add: (self: Tlk, value: string | TlkLine) -> number,
    write: (self: Tlk) -> string,
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

-- A finding's fix (ctx.fix, in a check; API 0.4): ctx.edit's methods,
-- which record its edits and make none. The user makes them.
export type Fix = {
    label: string,
    set: (self: Fix, name: string, field: string, value: any, type: string?) -> (),
    remove: (self: Fix, name: string, field: string) -> (),
    insert: (self: Fix, name: string, list: string, item: Struct, index: number?) -> number,
    remove_item: (self: Fix, name: string, item: string) -> (),
    write: (self: Fix, name: string, text: string) -> (),
    write_bytes: (self: Fix, name: string, bytes: string) -> (),
    write_gff: (self: Fix, name: string, document: Struct) -> (),
    delete: (self: Fix, name: string) -> (),
}

export type Log = {
    info: (self: Log, text: string) -> (),
    warn: (self: Log, text: string) -> (),
    error: (self: Log, text: string) -> (),
}

-- A file of the user's that they chose: its name and its bytes.
export type File = {
    name: string,
    bytes: string,
}

-- A folder of the user's that they chose. Files are named by their path
-- in it, with "/" ("sub/file.txt"); nothing outside it is reached.
export type Folder = {
    name: string,
    files: (self: Folder) -> { string },
    bytes: (self: Folder, name: string) -> string,
    text: (self: Folder, name: string) -> string,
    to_hak: (self: Folder, hak: string, file: string, name: string?) -> (),
    write: (self: Folder, name: string, bytes: string) -> (),
}

export type Ui = {
    message: (self: Ui, text: string) -> (),
    confirm: (self: Ui, text: string) -> boolean,
    form: (self: Ui, form: Form) -> { [string]: any }?,
    open_file: (self: Ui, options: { title: string?, extensions: { string }? }?) -> File?,
    open_folder: (self: Ui, options: { title: string? }?) -> Folder?,
}

-- Resources for a hak of the user's hak folder (they are asked once for
-- each hak), and the module's list of haks.
export type Hak = {
    write: (self: Hak, hak: string, name: string, bytes: string) -> (),
    attach: (self: Hak, hak: string) -> (),
}

-- A picture decoded. Pixels count from 0, from the top left.
export type Image = {
    width: number,
    height: number,
    pixel: (self: Image, x: number, y: number) -> (number, number, number, number),
}

-- An area's terrain. Cells and corners count from the south-west, from 0:
-- an area `width` cells across has corners 0 to `width`.
export type Area = {
    resref: string,
    tileset: string,
    width: number,
    height: number,
    step: number,
    terrains: { string },
    crossers: { string },
    groups: { { name: string, rows: number, columns: number } },
    corner: (self: Area, x: number, y: number) -> (string, number),
    tile: (self: Area, x: number, y: number) -> { id: number, orientation: number, height: number },
    paint: (self: Area, x: number, y: number, terrain: string) -> boolean,
    raise: (self: Area, x: number, y: number, steps: number?) -> boolean,
    set_height: (self: Area, x: number, y: number, height: number) -> boolean,
    place_group: (self: Area, group: string, x: number, y: number, turns: number?) -> boolean,
    cross: (self: Area, crosser: string, path: { { number } }) -> boolean,
    erase: (self: Area, x: number, y: number) -> boolean,
    set_tile: (self: Area, x: number, y: number, id: number, orientation: number?, height: number?) -> (),
}

export type Terrain = {
    tilesets: (self: Terrain) -> { { resref: string, name: string } },
    new_area: (self: Terrain, spec: { name: string, tileset: string, width: number, height: number }) -> string,
    open: (self: Terrain, area: string) -> Area,
}

-- Something selected or open in the window (API 0.4): what it is, and
-- where (the resource and path that ctx.edit takes: a field of it is
-- `path .. "/Tag"`).
export type Selected = {
    -- "creature", "placeable"... (placed, or a blueprint), "area"; for
    -- another editor open, its resource's type ("nss", "dlg").
    kind: string,
    resource: string,
    -- Where in the resource: "/Placeable List[3]"; "" for its root.
    path: string,
    -- A blueprint (of a palette, or the module tree), not a placed object.
    blueprint: boolean,
    -- In the module (false: one of the game's blueprints).
    module: boolean,
    -- The area's resref, for a placed object and an area.
    area: string?,
    -- A placed object's place in its list, from 0.
    index: number?,
    tag: string?,
    -- A blueprint's or an area's own; a placed object's blueprint.
    resref: string?,
}

-- The plugin's settings (API 0.4), as its manifest declares them
-- ([setting]): each read by its id (ctx.settings.code), and gone through
-- with `for id, value in ctx.settings`. A value is text (a string, a
-- choice, a path), a whole number (an int) or true or false (a bool).
export type Settings = {
    -- Sets one, for Moonglow to keep where its scope says when the job is
    -- done (nil: back to its default). Not in a check.
    set: (self: Settings, id: string, value: (string | number | boolean)?) -> (),
    [string]: string | number | boolean,
}

export type Context = {
    -- What a command was run on (the objects selected in the area view,
    -- the blueprints in a palette, what was right-clicked); empty in a
    -- check and an event (API 0.4).
    selection: { Selected },
    -- The area or editor in front, if any (API 0.4).
    open: Selected?,
    module: Reader,
    game: Game,
    edit: Edit,
    hak: Hak,
    terrain: Terrain,
    log: Log,
    ui: Ui,
    progress: (done: number, total: number, note: string?) -> (),
    plugin: { id: string, name: string, version: string },
    -- The plugin's settings (API 0.4).
    settings: Settings,
    -- In an event's handler (mg.on): what happened.
    event: { name: string, resources: { string } }?,
    -- In a check's handler: a fix to give a finding (API 0.4).
    fix: ((label: string) -> Fix)?,
}

local mg = {}

mg.api = "0.4"

-- A command's handler may return what Undo calls the change:
-- { label = "..." }, or the text alone.
function mg.command(id: string, handler: (ctx: Context) -> ...any) end
-- A check's handler returns its findings (none: an empty list, or nothing).
function mg.check(id: string, handler: (ctx: Context) -> { Finding }?) end

-- What the plugin does by itself on an event its manifest declares
-- ([event], on = "module-opened" or "module-saved"): ctx.event says what
-- happened.
function mg.on(id: string, handler: (ctx: Context) -> ...any) end

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

-- A PNG, TGA or DDS picture decoded (`kind`: "png", "tga" or "dds", when
-- the bytes don't say).
function mg.image(bytes: string, kind: string?): Image
    return { width = 0, height = 0, pixel = function() return 0, 0, 0, 0 end }
end

-- Where a struct read with `gff` is, for the edits: "/ClassList[0]" ("" for
-- the root).
function mg.path(struct: Struct): string? return nil end

-- A table's description (the text of its `name.2da.toml`): what each
-- column holds ("text", "int", "float", "list", "flags", "strref", or
-- "row:" and a table), a value's name, and what is wrong in a table.
export type Described = {
    rows: string?,
    columns: { [string]: string },
    notes: { string },
    name: (self: Described, column: string, value: string | number) -> string?,
    faults: (self: Described, table: TwoDa, lengths: { [string]: number }?) -> { { row: number, column: string, value: string?, message: string } },
}

-- 2DAs made, read (from a file's bytes) and written; `kinds` says which
-- columns of a table of the game's hold what ("strref", "model", or
-- "row:" and the table whose row it names, as "row:feat"); `describe`
-- reads a table's description, `scripts` the sources of the scripts
-- whose constants it names.
mg.twoda = {
    parse = function(bytes: string): TwoDa return nil :: any end,
    new = function(columns: { string }): TwoDa return nil :: any end,
    kinds = function(name: string): { [string]: string } return {} end,
    describe = function(text: string, scripts: { [string]: string }?): Described return nil :: any end,
}

-- Talk tables made, read and written.
mg.tlk = {
    parse = function(bytes: string): Tlk return nil :: any end,
    new = function(): Tlk return nil :: any end,
}

return mg
