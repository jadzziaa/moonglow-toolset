-- Comma-separated values, as spreadsheets write them: a cell with a
-- comma, a quote or a line end is quoted, its quotes doubled.
local csv = {}

local function cell(text)
    if string.find(text, '[,"\r\n]') then
        return '"' .. string.gsub(text, '"', '""') .. '"'
    end
    return text
end

function csv.write(rows)
    local lines = {}
    for _, cells in rows do
        local out = {}
        for _, c in cells do
            table.insert(out, cell(c))
        end
        table.insert(lines, table.concat(out, ","))
    end
    return table.concat(lines, "\r\n") .. "\r\n"
end

function csv.read(text)
    local rows, cells, at, n = {}, {}, 1, #text
    -- (A byte order mark is no part of the first cell.)
    if string.sub(text, 1, 3) == "\239\187\191" then
        at = 4
    end
    while at <= n do
        local value
        if string.sub(text, at, at) == '"' then
            local parts = {}
            at += 1
            while at <= n do
                local q = string.find(text, '"', at, true) or n + 1
                table.insert(parts, string.sub(text, at, q - 1))
                at = q + 1
                if string.sub(text, at, at) == '"' then
                    table.insert(parts, '"')
                    at += 1
                else
                    break
                end
            end
            value = table.concat(parts)
        else
            local stop = string.find(text, "[,\r\n]", at) or n + 1
            value = string.sub(text, at, stop - 1)
            at = stop
        end
        table.insert(cells, value)
        local sep = string.sub(text, at, at)
        if sep == "," then
            at += 1
        else
            if sep == "\r" then
                at += 1
            end
            if string.sub(text, at, at) == "\n" then
                at += 1
            end
            table.insert(rows, cells)
            cells = {}
        end
    end
    -- (A row that ends in a comma at the end of the text: its last cell
    -- is empty.)
    if #cells > 0 then
        table.insert(cells, "")
        table.insert(rows, cells)
    end
    return rows
end

return csv
