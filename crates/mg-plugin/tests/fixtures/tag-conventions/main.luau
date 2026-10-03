local mg = require("@moonglow")
local rules = require("rules")

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

-- Every kind of edit: a level more for each guard's first class, a class
-- added and the old one removed, a name, a note in a script.
mg.command("promote", function(ctx)
    for _, name in ctx.module:resources("utc") do
        local guard = ctx.module:gff(name)
        local class = guard.ClassList[1]
        ctx.edit:set(name, mg.path(class) .. "/ClassLevel", class.ClassLevel + 1)
        -- What was written is what is read.
        assert(ctx.module:gff(name).ClassList[1].ClassLevel == class.ClassLevel + 1)
        local at = ctx.edit:insert(name, "/ClassList", {
            __struct_id = 2,
            Class = mg.int(7),
            ClassLevel = mg.short(1),
        })
        assert(at == 1)
        ctx.edit:remove_item(name, "/ClassList[0]")
        ctx.edit:set(name, "FirstName", "Captain")
        ctx.edit:set(name, "Wis", 14)
        ctx.edit:set(name, "Rank", 3, "byte")
        ctx.edit:set(name, "Motto", mg.locstring("Hold the gate"))
        ctx.edit:remove(name, "Comment")
    end
    ctx.edit:write("promoted.nss", "// promoted\nvoid main() { }\n")
    ctx.edit:delete("old.nss")
end)

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
