
        local mg = require("@moonglow")
        mg.on("lines", function(ctx)
            ctx.log:info(table.concat(ctx.event.changed, "|"))
        end)
        