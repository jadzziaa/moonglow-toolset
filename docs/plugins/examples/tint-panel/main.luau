local mg = require("@moonglow")

-- The variable The Domains of Dread's scripts tint a placeable by.
local VARIABLE = "tdod_tint"

-- Named tints, each shown with its swatch in the panel's choice.
local PRESETS = {
    { text = "Crimson", color = "#a01010" },
    { text = "Forest", color = "#2f6b2a" },
    { text = "Night blue", color = "#2a4d8f" },
    { text = "Bone", color = "#e3dac9" },
}

-- The placed placeables selected.
local function placeables(ctx)
    local chosen = {}
    for _, s in ctx.selection do
        if s.kind == "placeable" and s.area then
            table.insert(chosen, s)
        end
    end
    return chosen
end

-- A placed object's string variable of that name, or nil.
local function variable(ctx, s, name: string): string?
    local object = ctx.module:raw(s.resource, s.path)
    for _, v in (if object.VarTable then object.VarTable.value else {}) do
        if v.Name.value == name then
            return v.Value.value
        end
    end
    return nil
end

-- The named tint of a color, or "Custom".
local function preset_of(tint: string): string
    for _, p in PRESETS do
        if p.color == string.lower(tint) then
            return p.text
        end
    end
    return "Custom"
end

-- The panel: the first one's tint, in a color field that previews on the
-- placeables while its picker is open and runs set-tint when it closes;
-- and the named tints, each with its swatch, running set-preset.
mg.panel("tint", function(ctx)
    local chosen = placeables(ctx)
    if #chosen == 0 then
        return { { type = "text", text = "Select placed placeables in an area.", weak = true } }
    end
    local tint = variable(ctx, chosen[1], VARIABLE) or "#ffffff"
    local choices: { any } = { "Custom" }
    for _, p in PRESETS do
        table.insert(choices, p)
    end
    return {
        { type = "value", label = "Placeables", text = tostring(#chosen) },
        {
            type = "field",
            kind = "color",
            id = "tint",
            label = "Tint",
            value = tint,
            variable = VARIABLE,
            command = "set-tint",
        },
        {
            type = "field",
            kind = "choice",
            id = "preset",
            label = "Named",
            choices = choices,
            value = preset_of(tint),
            command = "set-preset",
        },
    }
end)

-- Each placeable's variable set to the tint: one of that name replaced
-- (taken out, then the new one put in).
local function tint(ctx, chosen, color: string)
    for _, s in chosen do
        local object = ctx.module:raw(s.resource, s.path)
        local vars = if object.VarTable then object.VarTable.value else {}
        for i = #vars, 1, -1 do
            if vars[i].Name.value == VARIABLE then
                ctx.edit:remove_item(s.resource, `{s.path}/VarTable[{i - 1}]`)
            end
        end
        ctx.edit:insert(s.resource, `{s.path}/VarTable`, {
            __struct_id = 0,
            Name = mg.string(VARIABLE),
            Type = mg.dword(3),
            Value = mg.string(color),
        })
    end
    return { label = `Tint {color}` }
end

-- Run from the panel, ctx.values has its fields' values; run from
-- anywhere else, it is empty.
mg.command("set-tint", function(ctx)
    local chosen = placeables(ctx)
    if not ctx.values.tint then
        ctx.ui:message("Pick the tint in its panel: Plugins › Panels › Tint.")
        return
    end
    if #chosen == 0 then
        return
    end
    return tint(ctx, chosen, ctx.values.tint)
end)

mg.command("set-preset", function(ctx)
    local chosen = placeables(ctx)
    for _, p in PRESETS do
        if p.text == ctx.values.preset and #chosen > 0 then
            return tint(ctx, chosen, p.color)
        end
    end
    return nil
end)
