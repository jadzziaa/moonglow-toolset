-- The team's rules, in a file of their own: main.luau gets what this
-- returns with require("rules").
local rules = {}

-- The tag a creature should have.
function rules.wanted(tag: string): string
    return string.upper(tag)
end

return rules
