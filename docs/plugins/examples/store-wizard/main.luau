local mg = require("@moonglow")

-- What each kind of store sells, by base item (baseitems.2da's rows), and
-- the prices it suggests.
local KINDS = {
    General = { sells = {}, markup = 120, markdown = 50 },
    Smith = { sells = { 0, 1, 2, 16, 36 }, markup = 150, markdown = 40 },
    Alchemist = { sells = { 49, 75, 105 }, markup = 140, markdown = 60 },
}

mg.wizard("new-store", {
    pages = {
        {
            id = "store",
            title = "The store",
            text = "What the store is called, and what kind of goods it keeps.",
            fields = {
                { id = "name", label = "Name", default = "General Store" },
                { id = "kind", label = "Kind", type = "choice", choices = { "General", "Smith", "Alchemist" } },
            },
            check = function(ctx, answers)
                if string.match(answers.name, "^%s*$") then
                    return "A store needs a name."
                end
                return nil
            end,
        },
        -- (Made from the answers so far: the kind suggests the prices.)
        function(ctx, answers)
            local kind = KINDS[answers.kind]
            return {
                id = "prices",
                title = `{answers.name}'s prices`,
                text = "As percentages of an item's value.",
                fields = {
                    { id = "markup", label = "Sells at (%)", type = "number", default = kind.markup, min = 1, max = 1000 },
                    { id = "markdown", label = "Buys at (%)", type = "number", default = kind.markdown, min = 0, max = 100 },
                },
                check = function(ctx, answers)
                    if answers.markdown >= answers.markup then
                        return "It would buy for more than it sells."
                    end
                    return nil
                end,
            }
        end,
        -- (A general store buys anything: no page.)
        function(ctx, answers)
            if answers.kind == "General" then
                return nil
            end
            return {
                id = "buys",
                title = "What it buys",
                fields = {
                    { id = "only", label = `Buys only what a {string.lower(answers.kind)} sells`, type = "check", default = true },
                },
            }
        end,
    },
    finish = function(ctx, answers)
        local name = ctx.module:new_name(answers.name, "utm")
        local resref = string.gsub(name, "%.utm$", "")
        local kind = KINDS[answers.kind]
        local buys = {}
        if answers.only then
            for _, item in kind.sells do
                table.insert(buys, { __struct_id = 97869, BaseItem = mg.int(item) })
            end
        end
        local pages = {}
        for _, id in { 0, 4, 2, 3, 1 } do
            table.insert(pages, { __struct_id = id })
        end
        ctx.edit:write_gff(name, {
            __data_type = "UTM ",
            ResRef = mg.resref(resref),
            LocName = mg.locstring(answers.name),
            Tag = mg.string(string.upper((string.gsub(answers.name, "%W", "_")))),
            MarkUp = mg.int(answers.markup),
            MarkDown = mg.int(answers.markdown),
            BlackMarket = mg.byte(0),
            BM_MarkDown = mg.int(25),
            IdentifyPrice = mg.int(100),
            MaxBuyPrice = mg.int(-1),
            StoreGold = mg.int(-1),
            OnOpenStore = mg.resref(""),
            OnStoreClosed = mg.resref(""),
            WillNotBuy = { type = "list", value = {} },
            WillOnlyBuy = { type = "list", value = buys },
            StoreList = { type = "list", value = pages },
            ID = mg.byte(0),
            Comment = mg.string(""),
        })
        ctx.log:info(`{name}: {answers.name}, selling at {answers.markup}%`)
        return { label = `New store {answers.name}` }
    end,
})
