local mg = require("@moonglow")

-- The variable The Domains of Dread's scripts tint a placeable by.
local VARIABLE = "tdod_tint"

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

mg.command("pick-tint", function(ctx)
    local chosen = {}
    for _, s in ctx.selection do
        if s.kind == "placeable" and s.area then
            table.insert(chosen, s)
        end
    end
    if #chosen == 0 then
        ctx.ui:message("Right-click placed placeables first.")
        return
    end
    -- The color field: its picker the Properties windows' own; while it is
    -- open, the placeables are drawn with the variable set to it.
    local answers = ctx.ui:form({
        title = "Tint",
        fields = {
            {
                id = "tint",
                label = "Color",
                type = "color",
                default = variable(ctx, chosen[1], VARIABLE) or "#ffffff",
                variable = VARIABLE,
            },
        },
    })
    if not answers then
        return
    end
    for _, s in chosen do
        -- One of that name is replaced: taken out, then the new one put in.
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
            Value = mg.string(answers.tint),
        })
    end
    return { label = `Tint {answers.tint}` }
end)
