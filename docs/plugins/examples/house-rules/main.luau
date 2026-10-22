local mg = require("@moonglow")

-- Before a save, for each area it would write: the area's tag. Players'
-- places and first visits are kept by it, so once an area has a tag the
-- team keeps it: a save that changes it is refused (or, with the setting
-- off, warned of), with a fix that puts it back. A tag that is not upper
-- case is warned of, with a fix. Nothing here edits: the user makes a fix,
-- or saves anyway.
mg.on("area-tags", function(ctx)
    local e = ctx.event
    if e.new == nil then
        return -- (an area deleted)
    end
    local tag = e.new.Tag or ""
    local was = if e.old then e.old.Tag or "" else nil
    if was and was ~= tag then
        local back = ctx.fix(`Put back {was}`)
        back:set(e.resource, "Tag", was)
        local why = `its tag was {was}, now {tag}: players' places are kept by it`
        if ctx.settings.keep_tags then
            e:refuse(why, back)
        else
            e:warn(why, back)
        end
        return
    end
    if tag ~= string.upper(tag) then
        local fix = ctx.fix(`Make it {string.upper(tag)}`)
        fix:set(e.resource, "Tag", string.upper(tag))
        e:warn(`tag {tag} is not upper case`, fix)
    end
end)

-- Waypoints pasted: a copy has its original's tag, and a script finds the
-- first of a tag. Each gets one of its own (the tag and a number), as part
-- of the paste: one Undo takes both back.
mg.on("pasted-waypoints", function(ctx)
    local taken: { [string]: number } = {}
    for _, git in ctx.module:resources("git") do
        for _, wp in ctx.module:gff(git).WaypointList or {} do
            local tag = wp.Tag or ""
            taken[tag] = (taken[tag] or 0) + 1
        end
    end
    local retagged = 0
    for _, o in ctx.event.objects do
        local tag = o.tag or ""
        if (taken[tag] or 0) > 1 then
            local n = 2
            while taken[`{tag}_{n}`] do
                n += 1
            end
            local new = `{tag}_{n}`
            taken[tag] -= 1
            taken[new] = 1
            ctx.edit:set(o.resource, o.path .. "/Tag", new)
            retagged += 1
        end
    end
    if retagged > 0 then
        ctx.log:info(`{retagged} pasted waypoints have tags of their own`)
    end
end)

-- A new area (the Area Wizard's, a copy's): the team's land, as a
-- variable of the area (kept in its GIT, where the game reads an area's
-- variables), as part of making it.
mg.on("new-area", function(ctx)
    local land = ctx.settings.land
    if land == "" then
        return
    end
    local git = string.gsub(ctx.event.resource, "%.are$", ".git")
    ctx.edit:insert(git, "/VarTable", {
        __struct_id = 0,
        Name = mg.string("land"),
        Type = mg.dword(3),
        Value = mg.string(land),
    })
    ctx.log:info(`{ctx.event.resource} is in {land}`)
end)
