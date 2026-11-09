
        local mg = require("@moonglow")
        mg.on("spin", function(ctx)
            pcall(function() while true do end end)
            ctx.event:warn("never")
        end)
        