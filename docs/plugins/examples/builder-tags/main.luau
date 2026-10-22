local mg = require("@moonglow")

-- What a tag the builder makes begins with: the team's prefix, then their
-- code if they have one (`KRZ_ab_`). Both are settings: the prefix the
-- team's (moonglow.toml), the code the builder's own (.moonglow/user.toml).
local function wanted(ctx): string
    local s = ctx.settings
    if s.code ~= "" then
        return `{s.prefix}{s.code}_`
    end
    return s.prefix
end

-- The blueprints the check looks at, as the team's setting says.
local function checked(ctx): { string }
    local kinds = ctx.settings.kinds
    if kinds == "creatures" then
        return { "utc" }
    elseif kinds == "placeables" then
        return { "utp" }
    end
    return { "utc", "utp" }
end

-- A tag with the prefix in front, cut to the longest the team allows.
local function prefixed(ctx, tag: string): string
    return string.sub(wanted(ctx) .. tag, 1, ctx.settings.max_length)
end

-- Build > Verify Module: each tag without the team's prefix, with a fix
-- that puts it (and the builder's code) in front. Nothing is checked while
-- the team has set no prefix.
mg.check("tag-prefix", function(ctx)
    local s = ctx.settings
    local findings = {}
    if s.prefix == "" then
        return findings
    end
    for _, kind in checked(ctx) do
        for _, name in ctx.module:resources(kind) do
            local tag = ctx.module:gff(name).Tag or ""
            if string.sub(tag, 1, #s.prefix) ~= s.prefix then
                local fix = ctx.fix(`Begin it with {wanted(ctx)}`)
                fix:set(name, "Tag", prefixed(ctx, tag))
                table.insert(findings, {
                    resource = name,
                    at = "Tag",
                    message = `tag "{tag}" does not begin with {s.prefix}`,
                    severity = if s.strict then "error" else "warning",
                    fix = fix,
                })
            end
        end
    end
    return findings
end)

-- Plugins > Set My Builder Code…: asks for the code and keeps it, in the
-- builder's own part of the project (the setting's scope says where).
mg.command("remember-code", function(ctx)
    local answer = ctx.ui:form({
        title = "Your builder code",
        fields = { { id = "code", label = "Code", default = ctx.settings.code } },
    })
    if answer == nil then
        return
    end
    ctx.settings:set("code", answer.code)
    ctx.log:info(`Your builder code is {answer.code}`)
end)

-- Right-click on creatures or placeables (or their blueprints) > Prefix
-- Tags: the prefix and the builder's code in front of each tag that lacks
-- them (a tag with the team's prefix alone gets the code after it).
mg.command("prefix-tags", function(ctx)
    local team = ctx.settings.prefix
    local begins = wanted(ctx)
    local changed = 0
    for _, selected in ctx.selection do
        local tag = selected.tag or ""
        if string.sub(tag, 1, #begins) ~= begins then
            if team ~= "" and string.sub(tag, 1, #team) == team then
                tag = string.sub(tag, #team + 1)
            end
            ctx.edit:set(selected.resource, selected.path .. "/Tag", prefixed(ctx, tag))
            changed += 1
        end
    end
    ctx.log:info(`{changed} tags begin with {begins}`)
    return { label = `Prefix tags with {begins}` }
end)
