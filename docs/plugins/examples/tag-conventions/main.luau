local mg = require("@moonglow")
local rules = require("rules")

-- Plugins > Fix Creature Tags: every creature blueprint's tag as the rules
-- want it. All the edits are one step of Undo.
mg.command("fix-tags", function(ctx)
    local changed = 0
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if tag ~= rules.wanted(tag) then
            ctx.edit:set(name, "Tag", rules.wanted(tag))
            changed += 1
        end
    end
    ctx.log:info(`{changed} creature tags changed`)
    return { label = "Upper-case creature tags" }
end)

-- Build > Verify Module: a finding for each tag the command would change.
mg.check("tag-case", function(ctx)
    local findings = {}
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if tag ~= rules.wanted(tag) then
            table.insert(findings, {
                resource = name,
                at = "Tag",
                message = `tag "{tag}" is not upper case`,
            })
        end
    end
    return findings
end)
