
        local mg = require("@moonglow")
        mg.on("land", function(ctx)
            local e = ctx.event
            ctx.log:info(`{e.resource} {e.kind} by {e.by}: {table.concat(e.made, " ")}`)
            ctx.edit:insert((string.gsub(e.resource, "%.are$", ".git")), "/VarTable",
                { __struct_id = 0, Name = mg.string("tdod_land"), Type = mg.dword(3),
                  Value = mg.string("krezk") })
        end)
        