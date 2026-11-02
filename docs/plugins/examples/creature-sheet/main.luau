local mg = require("@moonglow")
local csv = require("csv")

-- A creature blueprint as a row: its resref, tag, name and level.
local function row(name, utc)
    local level = 0
    for _, class in utc.ClassList or {} do
        level += class.ClassLevel
    end
    local first = utc.FirstName and utc.FirstName[0] or ""
    return { string.gsub(name, "%.utc$", ""), utc.Tag or "", first, tostring(level) }
end

local HEADER = { "resref", "tag", "name", "level" }

mg.exporter("creatures-csv", function(ctx, out)
    local rows = { HEADER }
    local names = ctx.module:resources("utc")
    for i, name in names do
        ctx.progress(i, #names, name)
        table.insert(rows, row(name, ctx.module:gff(name)))
    end
    out:write(csv.write(rows))
    return `{#names} creatures written to {out.name}`
end)

mg.exporter("selected-csv", function(ctx, out)
    local rows = { HEADER }
    for _, s in ctx.selection do
        -- (A blueprint of a standard palette is the game's.)
        local utc = if s.module then ctx.module:gff(s.resource) else ctx.game:gff(s.resource)
        table.insert(rows, row(s.resource, utc))
    end
    out:write(csv.write(rows))
    return `{#rows - 1} creatures written to {out.name}`
end)

mg.importer("creatures-csv", function(ctx, file)
    local rows = csv.read(file.bytes)
    local header = table.remove(rows, 1) or {}
    local at = {}
    for i, column in header do
        at[column] = i
    end
    if not (at.resref and at.tag and at.name) then
        error(`{file.name} has no resref, tag and name columns`)
    end
    local changed = 0
    for _, cells in rows do
        local name = cells[at.resref] .. ".utc"
        if ctx.module:has(name) then
            ctx.edit:set(name, "Tag", cells[at.tag])
            ctx.edit:set(name, "FirstName", cells[at.name])
            changed += 1
        else
            ctx.log:warn(`{name} is not in the module: its row is passed over`)
        end
    end
    ctx.log:info(`{changed} creatures read from {file.name}`)
    return { label = `Import {changed} creatures` }
end)
