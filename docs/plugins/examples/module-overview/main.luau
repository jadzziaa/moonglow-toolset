local mg = require("@moonglow")

-- Whether a tag is lower case anywhere (and so to upper-case).
local function lower(tag)
    return tag ~= nil and tag ~= string.upper(tag)
end

mg.panel("overview", function(ctx)
    local areas = ctx.module:resources("are")
    local shown = {
        { type = "heading", text = "The module" },
        { type = "value", label = "Areas", text = tostring(#areas) },
        { type = "value", label = "Creature blueprints", text = tostring(#ctx.module:resources("utc")) },
        { type = "value", label = "Scripts", text = tostring(#ctx.module:resources("nss")) },
    }
    -- Its areas, each opened by a click.
    local items = {}
    for _, name in areas do
        local are = ctx.module:gff(name)
        local title = if are.Name and are.Name[0] then are.Name[0] else name
        table.insert(items, { text = title, hint = `Open {name}`, open = name })
    end
    table.insert(shown, { type = "list", items = items })
    table.insert(shown, { type = "separator" })
    table.insert(shown, { type = "heading", text = "Selected" })
    if #ctx.selection == 0 then
        table.insert(shown, { type = "text", text = "Nothing is selected.", weak = true })
        return shown
    end
    local count = 0
    for _, s in ctx.selection do
        table.insert(shown, { type = "value", label = s.kind, text = s.tag or "(no tag)" })
        if s.module and lower(s.tag) then
            count += 1
        end
    end
    if count > 0 then
        table.insert(shown, { type = "text", text = `{count} of them not upper case`, color = "warning" })
        table.insert(shown, {
            type = "button",
            text = "Upper-Case Their Tags",
            hint = "Upper-Case Selected Tags, on what is selected",
            command = "upper-tags",
        })
    end
    return shown
end)

mg.command("upper-tags", function(ctx)
    local changed = 0
    for _, s in ctx.selection do
        if s.module and lower(s.tag) then
            ctx.edit:set(s.resource, s.path .. "/Tag", string.upper(s.tag))
            changed += 1
        end
    end
    ctx.log:info(`{changed} tags upper-cased`)
    return { label = "Upper-case tags" }
end)
