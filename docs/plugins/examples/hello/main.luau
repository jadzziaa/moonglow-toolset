local mg = require("@moonglow")

mg.command("count", function(ctx)
    local creatures = ctx.module:resources("utc")
    ctx.log:info(`The module has {#creatures} creature blueprints`)
end)

mg.command("stamp", function(ctx)
    local creatures = ctx.module:resources("utc")
    if not ctx.ui:confirm(`Write a note into {#creatures} creatures' comments?`) then
        return
    end
    for i, name in creatures do
        ctx.progress(i, #creatures, name)
        local creature = ctx.module:gff(name)
        ctx.edit:set(name, "Comment", `{creature.Tag}: seen by Hello`)
    end
    return { label = "Stamp creature comments" }
end)
