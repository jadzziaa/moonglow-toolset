-- The server's rules, shared by the command and the check.
local rules = {}

function rules.wanted(tag: string): string
    return string.upper(tag)
end

return rules
