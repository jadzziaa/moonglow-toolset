local mg = require("@moonglow")

mg.command("list-classes", function(ctx)
    local classes = ctx.game:table("classes")
    -- A class's name: its row's Name is a talk-table string's number.
    local function class_name(row: number): string
        local strref = tonumber(classes:get(row, "Name"))
        return (strref and ctx.game:string(strref)) or classes:get(row, "Label") or `class {row}`
    end
    local creatures = ctx.module:resources("utc")
    for _, name in creatures do
        local creature = ctx.module:gff(name)
        local parts = {}
        for _, class in creature.ClassList or {} do
            table.insert(parts, `{class_name(class.Class)} {class.ClassLevel}`)
        end
        ctx.log:info(`{name} ({creature.Tag}): {table.concat(parts, ", ")}`)
    end
    ctx.log:info(`{#creatures} creature blueprints`)
    -- (It changes nothing: there is no step to undo.)
end)
