
        local mg = require("@moonglow")
        mg.on("retag", function(ctx)
            local e = ctx.event
            for _, o in e.objects do
                ctx.edit:set(o.resource, o.path .. "/Tag", `{o.tag}_{e.by}`)
            end
            ctx.log:info(`{#e.objects} in {e.area}`)
            return "Retag the paste"
        end)
        