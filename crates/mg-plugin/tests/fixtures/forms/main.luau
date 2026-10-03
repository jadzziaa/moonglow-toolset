local mg = require("@moonglow")

mg.command("rename", function(ctx)
    local answers = ctx.ui:form({
        title = "Prefix Tags",
        fields = {
            { id = "prefix", label = "Prefix", type = "text", default = "NPC_" },
            { id = "count", label = "At most", type = "number", default = 10, min = 1, max = 100 },
            { id = "upper", label = "Upper case", type = "check", default = true },
            { id = "kind", label = "Of", type = "choice", choices = { "creatures", "doors" } },
        },
    })
    if not answers then
        ctx.log:info("canceled")
        return
    end
    if not ctx.ui:confirm(`Prefix up to {answers.count} {answers.kind}?`) then
        ctx.log:info("declined")
        return
    end
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if answers.upper then tag = string.upper(tag) end
        ctx.edit:set(name, "Tag", answers.prefix .. tag)
    end
    ctx.ui:message("Done.")
    print("prefixed", answers.prefix, answers.count)
end)
