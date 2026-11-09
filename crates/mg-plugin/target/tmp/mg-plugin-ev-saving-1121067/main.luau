
local mg = require("@moonglow")
mg.on("names", function(ctx)
    local e = ctx.event
    ctx.log:info(`{e.name} {e.resource} {e.kind} of {#e.resources}`)
    if e.kind == "area" then
        if e.old and e.old.Tag ~= e.new.Tag then
            e:refuse(`the area's tag is kept: it was {e.old.Tag}, now {e.new.Tag}`)
        end
        if e.new.Tag ~= string.upper(e.new.Tag) then
            local fix = ctx.fix("Upper-case it")
            fix:set(e.resource, "Tag", string.upper(e.new.Tag))
            e:warn(`tag {e.new.Tag} is not upper case`, fix)
        end
        ctx.log:info(table.concat(e.changed or {}, "; "))
    end
end)
mg.on("edits", function(ctx)
    ctx.edit:set(ctx.event.resource, "Tag", "X")
end)
mg.on("sets", function(ctx)
    ctx.settings:set("n", 2)
end)
