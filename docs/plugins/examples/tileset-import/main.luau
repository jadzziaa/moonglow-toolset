local mg = require("@moonglow")

-- What a .set says that the import needs: its groups, each with its name
-- and size. (A .set is written like an .ini file.)
local function groups_of(text: string)
    local groups, current = {}, nil
    for line in string.gmatch(text, "[^\r\n]+") do
        local section = string.match(line, "^%s*%[(.-)%]")
        if section then
            current = nil
            if string.match(string.upper(section), "^GROUP%d+$") then
                current = { name = section, rows = 1, columns = 1 }
                table.insert(groups, current)
            end
        elseif current then
            local key, value = string.match(line, "^%s*([%w_]+)%s*=%s*(.-)%s*$")
            if key == "Name" then
                current.name = value
            elseif key == "Rows" then
                current.rows = tonumber(value) or 1
            elseif key == "Columns" then
                current.columns = tonumber(value) or 1
            end
        end
    end
    return groups
end

mg.command("import", function(ctx)
    local folder = ctx.ui:open_folder({ title = "The folder with the tileset's files" })
    if not folder then
        return
    end
    local files = folder:files()
    local sets = {}
    for _, name in files do
        if string.match(string.lower(name), "^[^/]+%.set$") then
            table.insert(sets, name)
        end
    end
    if #sets == 0 then
        error(`{folder.name} has no .set file: it is not a tileset's folder`)
    end

    -- Which tileset (a folder has one, as a rule), and what to call the hak.
    local set = sets[1]
    local tileset = string.lower(string.sub(set, 1, -5))
    local groups = groups_of(folder:text(set))
    local names = { "(none)" }
    for _, group in groups do
        table.insert(names, group.name)
    end
    local answers = ctx.ui:form({
        title = `Import {folder.name}`,
        fields = {
            { id = "set", label = "Tileset", type = "choice", choices = sets, default = set },
            { id = "hak", label = "Into the hak", type = "text", default = tileset },
            { id = "group", label = "Make an area of the group", type = "choice",
              choices = names, default = names[#names] },
            { id = "area", label = "Called", type = "text", default = names[#names] },
        },
    })
    if not answers then
        return
    end
    if answers.set ~= set then
        set = answers.set
        tileset = string.lower(string.sub(set, 1, -5))
        groups = groups_of(folder:text(set))
    end

    -- Every file the game could read goes into the hak as it is: Moonglow
    -- copies it, so a large texture never passes through this code. What
    -- a hak can't hold (a name too long, a type the game doesn't know) is
    -- left out, and said.
    local copied, left = 0, 0
    for i, name in files do
        ctx.progress(i, #files, name)
        if string.find(name, "/", 1, true) then
            left += 1
        else
            local ok, why = pcall(folder.to_hak, folder, answers.hak, name)
            if ok then
                copied += 1
            elseif string.find(tostring(why), "not allowed", 1, true) then
                error(why, 0)
            else
                left += 1
                ctx.log:warn(`left out: {why}`)
            end
        end
    end
    if copied == 0 then
        error("none of the folder's files can go into a hak")
    end
    ctx.hak:attach(answers.hak)
    ctx.log:info(`{copied} files into {answers.hak}.hak, {left} left out`)

    -- An area the size of the group, with the group in its corner. The
    -- tileset is read from the hak being written: the game data doesn't
    -- have it yet.
    if answers.group ~= "(none)" then
        local group
        for _, g in groups do
            if g.name == answers.group then
                group = g
            end
        end
        local resref = ctx.terrain:new_area({
            name = if answers.area == "" then group.name else answers.area,
            tileset = tileset,
            width = math.clamp(group.columns, 2, 32),
            height = math.clamp(group.rows, 2, 32),
        })
        local area = ctx.terrain:open(resref)
        if not area:place_group(group.name, 0, 0) then
            ctx.log:warn(`the group {group.name} does not fit the area: {resref} was left as it is`)
        else
            ctx.log:info(`made the area {resref} with the group {group.name}`)
        end
    end
    return { label = `Import tileset {tileset}` }
end)
