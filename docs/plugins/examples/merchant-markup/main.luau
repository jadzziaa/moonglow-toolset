local mg = require("@moonglow")

mg.command("set-markup", function(ctx)
    local merchants = ctx.module:resources("utm")
    if #merchants == 0 then
        ctx.ui:message("The module has no merchant blueprints.")
        return
    end
    local answers = ctx.ui:form({
        title = `Markup of {#merchants} merchants`,
        fields = {
            { id = "sell", label = "Sells at (%)", type = "number", default = 150, min = 1, max = 1000 },
            { id = "buy", label = "Buys at (%)", type = "number", default = 50, min = 0, max = 100 },
            { id = "only", label = "Only tags starting", type = "text", default = "" },
        },
    })
    if not answers then
        return
    end
    local changed = 0
    for i, name in merchants do
        ctx.progress(i, #merchants, name)
        local merchant = ctx.module:gff(name)
        if string.sub(merchant.Tag, 1, #answers.only) == answers.only then
            ctx.edit:set(name, "MarkUp", answers.sell)
            ctx.edit:set(name, "MarkDown", answers.buy)
            changed += 1
        end
    end
    ctx.log:info(`{changed} of {#merchants} merchants set to sell at {answers.sell}% and buy at {answers.buy}%`)
    return { label = "Set merchant markup" }
end)
