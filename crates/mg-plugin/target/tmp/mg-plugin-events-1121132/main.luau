
        local mg = require("@moonglow")
        mg.on("emit", function(ctx)
            ctx.log:info(`{ctx.event.name}: {table.concat(ctx.event.resources, " ")}`)
            local folder = ctx.ui:open_folder({ title = "Development" })
            folder:write("saved.txt", table.concat(ctx.event.resources, "\n"))
            ctx.edit:write("note.nss", "// saved\n")
            return "Emit"
        end)
        mg.check("writes", function(ctx)
            ctx.ui:open_folder():write("x.txt", "x")
        end)
        