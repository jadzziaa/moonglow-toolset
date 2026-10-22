local mg = require("@moonglow")

-- The local variable's type, as a VarTable has it, and its value typed.
local TYPES = { int = 1, float = 2, string = 3 }

local function typed(kind: string, text: string)
    if kind == "string" then
        return mg.string(text)
    end
    local n = tonumber(text)
    if n == nil then
        return nil
    end
    return if kind == "int" then mg.int(math.floor(n)) else mg.float(n)
end

-- Run from the right-click menu of a placed object or a blueprint, it gets
-- what was clicked; from the Plugins menu, what is selected. Each entry
-- says where it is: the resource and path that ctx.edit takes.
mg.command("set-local", function(ctx)
    local chosen = {}
    for _, s in ctx.selection do
        -- (A blueprint of the game's is not the module's to change.)
        if s.module and s.kind ~= "area" then
            table.insert(chosen, s)
        end
    end
    if #chosen == 0 then
        ctx.ui:message("Select objects, or blueprints of the module, first.")
        return
    end
    local answers = ctx.ui:form({
        title = `A local variable on {#chosen} {if #chosen == 1 then "object" else "objects"}`,
        fields = {
            { id = "name", label = "Name", default = "" },
            { id = "type", label = "Type", type = "choice", choices = { "int", "float", "string" }, default = "string" },
            { id = "value", label = "Value", default = "" },
        },
    })
    if not answers or answers.name == "" then
        return
    end
    local value = typed(answers.type, answers.value)
    if value == nil then
        ctx.ui:message(`{answers.value} is not a number`)
        return
    end
    for _, s in chosen do
        -- One of that name is replaced: taken out, then the new one put in.
        local object = ctx.module:raw(s.resource, s.path)
        local vars = if object.VarTable then object.VarTable.value else {}
        for i = #vars, 1, -1 do
            if vars[i].Name.value == answers.name then
                ctx.edit:remove_item(s.resource, `{s.path}/VarTable[{i - 1}]`)
            end
        end
        ctx.edit:insert(s.resource, `{s.path}/VarTable`, {
            __struct_id = 0,
            Name = mg.string(answers.name),
            Type = mg.dword(TYPES[answers.type]),
            Value = value,
        })
        ctx.log:info(`{s.tag or s.resref}: {answers.name} = {answers.value}`)
    end
    return { label = `Set {answers.name}` }
end)

-- The GIT's lists of placed objects, and what each holds.
local LISTS = {
    { "Creature List", "creatures" },
    { "Door List", "doors" },
    { "Encounter List", "encounters" },
    { "List", "items" },
    { "Placeable List", "placeables" },
    { "SoundList", "sounds" },
    { "StoreList", "stores" },
    { "TriggerList", "triggers" },
    { "WaypointList", "waypoints" },
}

-- Run on an area: the one right-clicked, or the one open in front.
mg.command("count-objects", function(ctx)
    local area = ctx.selection[1] or ctx.open
    if area == nil or area.kind ~= "area" then
        ctx.ui:message("Right-click an area, or open one.")
        return
    end
    local git = ctx.module:gff(`{area.area}.git`)
    local counts = {}
    for _, list in LISTS do
        local n = if git[list[1]] then #git[list[1]] else 0
        if n > 0 then
            table.insert(counts, `{n} {list[2]}`)
        end
    end
    local held = if #counts == 0 then "nothing placed" else table.concat(counts, ", ")
    ctx.log:info(`{area.tag or area.area}: {held}`)
end)
