local mg = require("@moonglow")

-- What a sandbox must not have, and what it must not let in.
mg.command("reach", function(ctx)
    local found = {}
    for _, name in { "io", "package", "loadfile", "dofile", "debug" } do
        if getfenv()[name] ~= nil and name ~= "debug" then
            table.insert(found, name)
        end
    end
    if os.execute or os.remove or os.rename or os.getenv or os.exit then
        table.insert(found, "os")
    end
    for _, path in { "../tag-conventions/rules", "/etc/passwd", "..", "a\\b", "c:x" } do
        if pcall(require, path) then
            table.insert(found, "require " .. path)
        end
    end
    -- The globals are fixed: a plugin cannot replace what others use.
    if pcall(function() string.upper = function() return "x" end end) then
        table.insert(found, "string.upper replaced")
    end
    return { label = table.concat(found, ", ") }
end)

-- Never ends by itself, and swallows every error it is given.
mg.command("spin", function(ctx)
    while true do
        pcall(function()
            while true do
                xpcall(function() while true do end end, function() end)
            end
        end)
        coroutine.resume(coroutine.create(function() while true do end end))
    end
end)

mg.command("hoard", function(ctx)
    local kept = {}
    while true do
        table.insert(kept, string.rep("x", 16 * 1024 * 1024) .. tostring(#kept))
    end
end)

-- Each of these fails where it is made, and changes nothing.
mg.command("bad-edits", function(ctx)
    local faults = {}
    local function try(what, f)
        local ok, err = pcall(f)
        table.insert(faults, what .. ": " .. (ok and "allowed" or tostring(err)))
    end
    try("no resource", function() ctx.edit:set("nobody.utc", "Tag", "X") end)
    try("no name", function() ctx.edit:set("guard", "Tag", "X") end)
    try("no path", function() ctx.edit:set("guard.utc", "/ClassList[9]/ClassLevel", 1) end)
    try("too big", function() ctx.edit:set("guard.utc", "/ClassList[0]/ClassLevel", 70000) end)
    try("not a number", function() ctx.edit:set("guard.utc", "/ClassList[0]/ClassLevel", "five") end)
    try("no type", function() ctx.edit:set("guard.utc", "Invented", 1) end)
    try("a list", function() ctx.edit:set("guard.utc", "ClassList", {}) end)
    try("no item", function() ctx.edit:remove_item("guard.utc", "/ClassList[4]") end)
    try("no field", function() ctx.edit:remove("guard.utc", "Nothing") end)
    try("not there", function() ctx.edit:delete("nobody.nss") end)
    for _, fault in faults do
        ctx.log:warn(fault)
    end
end)

mg.command("fails", function(ctx)
    ctx.edit:set("guard.utc", "Tag", "HALF_DONE")
    error("something went wrong")
end)

mg.check("edits", function(ctx)
    ctx.edit:set("guard.utc", "Tag", "CHANGED_BY_A_CHECK")
    return {}
end)

-- Work inside the string library, where no Luau code runs.
mg.command("match", function(ctx)
    local text = string.rep("a", 200000)
    while true do
        string.find(text, ".-.-.-.-.-.-b")
    end
end)
