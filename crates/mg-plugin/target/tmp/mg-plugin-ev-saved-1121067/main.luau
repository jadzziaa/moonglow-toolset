
        local mg = require("@moonglow")
        mg.on("after", function(ctx)
            local e = ctx.event
            ctx.log:info(`{e.resource}: {e.old and e.old.Tag} -> {e.new and e.new.Tag}`)
            if e.warn then error("a saved resource is past warning") end
        end)
        mg.on("edits", function(ctx) ctx.edit:set("guard.utc", "Tag", "X") end)
        