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

-- Build > Verify Module: a finding for each tag the command would change,
-- with a fix that changes that one. The fix's edit is recorded, not made:
-- Verify shows a button beside the finding, and the tag changes only when
-- it is clicked (or with `mg verify --fix example.tag-conventions/tag-case`).
mg.check("tag-case", function(ctx)
    local findings = {}
    for _, name in ctx.module:resources("utc") do
        local tag = ctx.module:gff(name).Tag
        if tag ~= rules.wanted(tag) then
            local fix = ctx.fix("Upper-case this tag")
            fix:set(name, "Tag", rules.wanted(tag))
            table.insert(findings, {
                resource = name,
                at = "Tag",
                message = `tag "{tag}" is not upper case`,
                fix = fix,
            })
        end
    end
    return findings
end)
