//! The Script Wizard: a conversation's condition ("Text Appears When…") or
//! action ("Actions Taken") script built from a few pages of choices, written
//! the way Aurora's wizard writes it (captured under `script_wizard/`):
//! the same header, sections, comments and helper calls, in the same order.
//!
//! Two of Aurora's outputs do not work in Enhanced Edition, and Moonglow
//! writes them differently:
//! - An action that opens a store with appraise checks includes
//!   `nw_i0_plot`; with a party reward it also includes `nw_i0_tool`, and
//!   the two define `HasItem` twice, so the script does not compile. Such a
//!   script includes only `nw_i0_plot`, gives party gold with its
//!   `RewardGP` and party XP with a loop over the party.
//! - "Take XP" called `GiveXPToCreature` with a negative amount, which the
//!   engine ignores; Moonglow lowers the XP with `SetXP`.
//!
//! The pages' lists come from the 2DAs ([`Lists`]): names from the talk
//! table, sorted as Aurora sorts them, and each row's `Constant` when
//! `nwscript.nss` defines it (else the row number, e.g. for a hak's rows).

use std::collections::HashSet;
use std::fmt::Write;

use mg_core::{ResType, StrRef};
use mg_rules::GameData;
use mg_script::spec::Spec;

use crate::new::word_sort_key;

/// A comparison on the Abilities and Local Variable pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Compare {
    #[default]
    Equal,
    NotEqual,
    Less,
    Greater,
}

impl Compare {
    pub const ALL: [Compare; 4] =
        [Compare::Equal, Compare::NotEqual, Compare::Less, Compare::Greater];

    pub fn label(self) -> &'static str {
        match self {
            Compare::Equal => "is equal to",
            Compare::NotEqual => "is not equal to",
            Compare::Less => "is less than",
            Compare::Greater => "is greater than",
        }
    }

    pub fn op(self) -> &'static str {
        match self {
            Compare::Equal => "==",
            Compare::NotEqual => "!=",
            Compare::Less => "<",
            Compare::Greater => ">",
        }
    }
}

/// The six abilities: name and constant.
pub const ABILITIES: [(&str, &str); 6] = [
    ("Strength", "ABILITY_STRENGTH"),
    ("Dexterity", "ABILITY_DEXTERITY"),
    ("Constitution", "ABILITY_CONSTITUTION"),
    ("Intelligence", "ABILITY_INTELLIGENCE"),
    ("Wisdom", "ABILITY_WISDOM"),
    ("Charisma", "ABILITY_CHARISMA"),
];

/// The alignment checkboxes of each axis.
pub const GOOD_EVIL: [(&str, &str); 3] =
    [("Good", "ALIGNMENT_GOOD"), ("Neutral", "ALIGNMENT_NEUTRAL"), ("Evil", "ALIGNMENT_EVIL")];
pub const LAW_CHAOS: [(&str, &str); 3] = [
    ("Lawful", "ALIGNMENT_LAWFUL"),
    ("Neutral", "ALIGNMENT_NEUTRAL"),
    ("Chaotic", "ALIGNMENT_CHAOTIC"),
];

/// A skill check's difficulty (`AutoDC`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    #[default]
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard];

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Normal => "Normal",
            Difficulty::Hard => "Hard",
        }
    }

    pub fn constant(self) -> &'static str {
        match self {
            Difficulty::Easy => "DC_EASY",
            Difficulty::Normal => "DC_MEDIUM",
            Difficulty::Hard => "DC_HARD",
        }
    }
}

/// The type of a local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VarType {
    Float,
    #[default]
    Int,
    String,
}

impl VarType {
    pub const ALL: [VarType; 3] = [VarType::Float, VarType::Int, VarType::String];

    pub fn label(self) -> &'static str {
        match self {
            VarType::Float => "float",
            VarType::Int => "int",
            VarType::String => "string",
        }
    }

    fn suffix(self) -> &'static str {
        match self {
            VarType::Float => "Float",
            VarType::Int => "Int",
            VarType::String => "String",
        }
    }

    /// The comparisons offered for this type (Aurora offers only "is equal
    /// to" for strings; Moonglow adds "is not equal to").
    pub fn comparisons(self) -> &'static [Compare] {
        match self {
            VarType::String => &[Compare::Equal, Compare::NotEqual],
            _ => &Compare::ALL,
        }
    }

    /// The value as NWScript: a constant as typed, checked for the type
    /// (floats written as C's `%f`, strings quoted), or another variable.
    pub fn value(self, v: &Operand) -> Option<String> {
        match v {
            Operand::Variable(name) => Some(local(self, name)),
            Operand::Constant(c) => match self {
                VarType::Int => c.trim().parse::<i32>().ok().map(|n| n.to_string()),
                VarType::Float => c.trim().parse::<f32>().ok().map(|f| format!("{f:.6}")),
                VarType::String => Some(quote(c)),
            },
        }
    }
}

/// The right-hand side of a local variable comparison or assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Constant(String),
    /// Another local variable of the same type on the PC speaker.
    Variable(String),
}

/// A local variable test on the PC speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCheck {
    pub ty: VarType,
    pub name: String,
    pub compare: Compare,
    pub value: Operand,
}

impl LocalCheck {
    /// `GetLocalInt(GetPCSpeaker(), "nQuest") == 2`; `None` when the value
    /// is not valid for the type.
    pub fn expression(&self) -> Option<String> {
        let value = self.ty.value(&self.value)?;
        Some(format!("{} {} {value}", local(self.ty, &self.name), self.compare.op()))
    }
}

/// A local variable set on the PC speaker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSet {
    pub ty: VarType,
    pub name: String,
    pub value: Operand,
}

impl LocalSet {
    /// `SetLocalInt(GetPCSpeaker(), "nQuest", 3);`.
    pub fn statement(&self) -> Option<String> {
        let value = self.ty.value(&self.value)?;
        Some(format!(
            "SetLocal{}(GetPCSpeaker(), {}, {value});",
            self.ty.suffix(),
            quote(&self.name)
        ))
    }
}

fn local(ty: VarType, name: &str) -> String {
    format!("GetLocal{}(GetPCSpeaker(), {})", ty.suffix(), quote(name))
}

/// A string literal.
fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// A row of a wizard list: its 2DA row, name and NWScript constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub row: usize,
    pub name: String,
    pub constant: String,
}

/// The lists the wizard's pages offer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lists {
    /// Player classes (`classes.2da` `PlayerClass` 1), by name.
    pub classes: Vec<Entry>,
    /// `gender.2da`, by name.
    pub genders: Vec<Entry>,
    /// Player races (`racialtypes.2da` `PlayerRace` 1), by name.
    pub player_races: Vec<Entry>,
    /// The other named races, by name.
    pub other_races: Vec<Entry>,
    /// Named feats, by name.
    pub feats: Vec<Entry>,
    /// Named skills, in row order (the Skill Check page's order; the Skills
    /// page sorts them by name).
    pub skills: Vec<Entry>,
}

impl Lists {
    /// Reads the lists from the game's (and the module's haks') 2DAs.
    pub fn load(game: &GameData) -> Lists {
        let spec = game
            .resman
            .get_named("nwscript", ResType::NSS)
            .map(|d| Spec::parse(&d))
            .unwrap_or_default();
        let rows = |table: &str, name: &str, prefix: &str, keep: &dyn Fn(usize) -> bool| {
            let Ok(t) = game.table(table) else { return Vec::new() };
            let (Some(name_col), constant_col) = (t.column(name), t.column("Constant")) else {
                return Vec::new();
            };
            (0..t.len())
                .filter(|&row| keep(row))
                .filter_map(|row| {
                    let strref = t.cell(row, name_col).and_then(mg_2da::parse_int)?;
                    let name = game.string(StrRef(strref as u32)).filter(|s| !s.is_empty())?;
                    let declared = constant_col.and_then(|c| t.cell(row, c));
                    Some(Entry { row, name, constant: constant(&spec, prefix, row, declared) })
                })
                .collect::<Vec<_>>()
        };
        let flag = |table: &str, column: &'static str| {
            let t = game.table(table).ok();
            move |row: usize| {
                t.as_ref().and_then(|t| t.get(row, column)).and_then(mg_2da::parse_int).unwrap_or(0)
                    == 1
            }
        };
        let player_class = flag("classes", "PlayerClass");
        let player_race = flag("racialtypes", "PlayerRace");
        let by_name = |mut v: Vec<Entry>| {
            v.sort_by_cached_key(|e| word_sort_key(&e.name));
            v
        };
        Lists {
            classes: by_name(rows("classes", "Name", "CLASS_TYPE_", &player_class)),
            genders: by_name(rows("gender", "NAME", "GENDER_", &|_| true)),
            player_races: by_name(rows("racialtypes", "Name", "RACIAL_TYPE_", &player_race)),
            other_races: by_name(rows("racialtypes", "Name", "RACIAL_TYPE_", &|r| !player_race(r))),
            feats: by_name(rows("feat", "FEAT", "FEAT_", &|_| true)),
            skills: rows("skills", "Name", "SKILL_", &|_| true),
        }
    }

    /// The skills sorted by name (the Skills page).
    pub fn skills_by_name(&self) -> Vec<Entry> {
        let mut v = self.skills.clone();
        v.sort_by_cached_key(|e| word_sort_key(&e.name));
        v
    }
}

/// The constant naming a row: the 2DA's `Constant` if `nwscript.nss`
/// defines it with the row's value, else any constant with the prefix and
/// that value, else the number.
fn constant(spec: &Spec, prefix: &str, row: usize, declared: Option<&str>) -> String {
    let value_is_row =
        |c: &mg_script::spec::Constant| c.value.trim().parse::<i64>().ok() == Some(row as i64);
    if let Some(c) = declared.and_then(|d| spec.constant(d)).filter(|c| value_is_row(c)) {
        return c.name.clone();
    }
    spec.constants
        .iter()
        .find(|c| c.ty == "int" && c.name.starts_with(prefix) && value_is_row(c))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| row.to_string())
}

/// A class restriction: a class (or any, by hit dice) and a minimum level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassLevel {
    /// A `classes.2da` row; `None` for Any.
    pub class: Option<usize>,
    /// The minimum level; `None` for any level.
    pub level: Option<u32>,
}

/// The alignments allowed on each axis (Good, Neutral, Evil; Lawful,
/// Neutral, Chaotic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Alignment {
    pub good_evil: [bool; 3],
    pub law_chaos: [bool; 3],
}

/// The pages of a condition script. A page not used is empty (or `None`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Condition {
    /// Ability tests: an index into [`ABILITIES`], a comparison, a score.
    pub abilities: Vec<(usize, Compare, i32)>,
    pub classes: Vec<ClassLevel>,
    /// The player needs all the class restrictions (else only one).
    pub all_classes: bool,
    /// The genders allowed (`gender.2da` rows).
    pub genders: Option<Vec<usize>>,
    /// The races accepted (`racialtypes.2da` rows); all others are rejected.
    pub races: Option<Vec<usize>>,
    pub alignment: Option<Alignment>,
    /// Required feats and skills (rows).
    pub feats: Vec<usize>,
    pub skills: Vec<usize>,
    /// Skill checks in the order added: difficulty and `skills.2da` row.
    pub skill_checks: Vec<(Difficulty, usize)>,
    /// Tags of items the PC speaker must have.
    pub items: Vec<String>,
    pub locals: Vec<LocalCheck>,
    /// A chance of `n` in `m`.
    pub random: Option<(u32, u32)>,
}

/// The action page's main action.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Perform {
    #[default]
    Nothing,
    /// Turn hostile and attack the PC speaker.
    Attack,
    /// Open the nearest store with this tag.
    Store { tag: String, appraise: bool },
}

/// The pages of an action script.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActionScript {
    /// Gold and XP to give, and whether to the whole party.
    pub give_gold: Option<(u32, bool)>,
    pub give_xp: Option<(u32, bool)>,
    /// Blueprints (resrefs) of items to give.
    pub give_items: Vec<String>,
    /// Gold to take, and whether it is destroyed (else the speaker keeps it).
    pub take_gold: Option<(u32, bool)>,
    pub take_xp: Option<u32>,
    /// Tags of items to take, and whether they are destroyed.
    pub take_items: Vec<String>,
    pub destroy_items: bool,
    pub locals: Vec<LocalSet>,
    pub perform: Perform,
    /// A change to the speaker's faction's reputation with the PC
    /// (-100..=100; Attack sets -100).
    pub faction: i32,
}

/// The comment block Aurora's wizard starts each script with.
fn header(out: &mut String, name: &str, date: &str) {
    let _ = write!(
        out,
        "//::///////////////////////////////////////////////\n\
         //:: FileName {name}\n\
         //:://////////////////////////////////////////////\n\
         //:://////////////////////////////////////////////\n\
         //:: Created By: Script Wizard\n\
         //:: Created On: {date}\n\
         //:://////////////////////////////////////////////\n"
    );
}

/// Lines with the wizard's line endings (CRLF).
fn crlf(s: &str) -> String {
    s.replace('\n', "\r\n")
}

fn reject(out: &mut String, test: &str) {
    let _ = write!(out, "\tif({test})\n\t\treturn FALSE;\n");
}

/// The tests of one alignment axis: none when no box or every box is
/// ticked, "not X" for one box, else "not each unticked one".
fn axis(out: &mut String, getter: &str, names: &[(&str, &str); 3], allowed: [bool; 3]) {
    choose(out, getter, names.iter().map(|n| n.1).zip(allowed));
}

/// "Must be one of the ticked" as Aurora writes it: `!= X` for a single
/// choice, else `== Y` for each unticked one; nothing when none or all are
/// ticked.
fn choose<'a>(out: &mut String, getter: &str, options: impl Iterator<Item = (&'a str, bool)>) {
    let options: Vec<(&str, bool)> = options.collect();
    let ticked = options.iter().filter(|o| o.1).count();
    if ticked == 1 {
        let (c, _) = options.iter().find(|o| o.1).expect("one ticked");
        reject(out, &format!("{getter}(GetPCSpeaker()) != {c}"));
    } else if ticked > 1 && ticked < options.len() {
        for (c, _) in options.iter().filter(|o| !o.1) {
            reject(out, &format!("{getter}(GetPCSpeaker()) == {c}"));
        }
    }
}

fn sorted_items(items: &[String]) -> Vec<&String> {
    let mut v: Vec<&String> = items.iter().collect();
    v.sort_by_cached_key(|s| (s.to_lowercase(), (*s).clone()));
    v
}

/// The condition script (`StartingConditional`), CRLF line endings.
/// `date` goes in the header's "Created On".
pub fn condition_script(name: &str, date: &str, c: &Condition, lists: &Lists) -> String {
    let mut body = String::new();
    // Abilities: no comment, first.
    for &(i, cmp, value) in &c.abilities {
        let Some((_, constant)) = ABILITIES.get(i) else { continue };
        reject(
            &mut body,
            &format!("!(GetAbilityScore(GetPCSpeaker(), {constant}) {} {value})", cmp.op()),
        );
    }
    let class_test = |cl: &ClassLevel| -> String {
        match cl.class {
            None => "GetHitDice(GetPCSpeaker())".to_string(),
            Some(row) => {
                let constant = lists
                    .classes
                    .iter()
                    .find(|e| e.row == row)
                    .map_or_else(|| row.to_string(), |e| e.constant.clone());
                format!("GetLevelByClass({constant}, GetPCSpeaker())")
            }
        }
    };
    let section = |body: &mut String, comment: &str, lines: String| {
        if !lines.is_empty() {
            let _ = write!(body, "\n\t// {comment}\n{lines}");
        }
    };
    let mut lines = String::new();
    if !c.classes.is_empty() {
        if c.all_classes {
            for cl in &c.classes {
                reject(&mut lines, &format!("{} < {}", class_test(cl), cl.level.unwrap_or(1)));
            }
        } else {
            lines.push_str("\tint iPassed = 0;\n");
            for (i, cl) in c.classes.iter().enumerate() {
                let test = format!("{} >= {}", class_test(cl), cl.level.unwrap_or(1));
                if i == 0 {
                    let _ = write!(lines, "\tif({test})\n\t\tiPassed = 1;\n");
                } else {
                    let _ = write!(lines, "\tif((iPassed == 0) && ({test}))\n\t\tiPassed = 1;\n");
                }
            }
            reject(&mut lines, "iPassed == 0");
        }
    }
    section(&mut body, "Restrict based on the player's class", std::mem::take(&mut lines));

    let feats: HashSet<usize> = c.feats.iter().copied().collect();
    for e in lists.feats.iter().filter(|e| feats.contains(&e.row)) {
        reject(&mut lines, &format!("!GetHasFeat({}, GetPCSpeaker())", e.constant));
    }
    section(&mut body, "Make sure the player has the required feats", std::mem::take(&mut lines));

    if let Some(genders) = &c.genders {
        let on: HashSet<usize> = genders.iter().copied().collect();
        choose(
            &mut lines,
            "GetGender",
            lists.genders.iter().map(|e| (e.constant.as_str(), on.contains(&e.row))),
        );
    }
    section(&mut body, "Add the gender restrictions", std::mem::take(&mut lines));

    for tag in sorted_items(&c.items) {
        reject(&mut lines, &format!("!HasItem(GetPCSpeaker(), {})", quote(tag)));
    }
    section(
        &mut body,
        "Make sure the PC speaker has these items in their inventory",
        std::mem::take(&mut lines),
    );

    let mut locals: Vec<String> = c.locals.iter().filter_map(LocalCheck::expression).collect();
    locals.sort_by_cached_key(|s| (s.to_lowercase(), s.clone()));
    for e in &locals {
        reject(&mut lines, &format!("!({e})"));
    }
    section(&mut body, "Inspect local variables", std::mem::take(&mut lines));

    if let Some(races) = &c.races {
        let accepted: HashSet<usize> = races.iter().copied().collect();
        for (list, comment) in [
            (&lists.player_races, "Reject player races"),
            (&lists.other_races, "Reject other races"),
        ] {
            for e in list.iter().filter(|e| !accepted.contains(&e.row)) {
                reject(&mut lines, &format!("GetRacialType(GetPCSpeaker()) == {}", e.constant));
            }
            section(&mut body, comment, std::mem::take(&mut lines));
        }
    }

    if let Some(a) = c.alignment {
        axis(&mut lines, "GetAlignmentGoodEvil", &GOOD_EVIL, a.good_evil);
        axis(&mut lines, "GetAlignmentLawChaos", &LAW_CHAOS, a.law_chaos);
    }
    section(&mut body, "Restrict based on the player's alignment", std::mem::take(&mut lines));

    let skills: HashSet<usize> = c.skills.iter().copied().collect();
    for e in lists.skills_by_name().iter().filter(|e| skills.contains(&e.row)) {
        reject(&mut lines, &format!("!GetHasSkill({}, GetPCSpeaker())", e.constant));
    }
    section(&mut body, "Make sure the player has the required skills", std::mem::take(&mut lines));

    for &(d, row) in &c.skill_checks {
        let constant = lists
            .skills
            .iter()
            .find(|e| e.row == row)
            .map_or_else(|| row.to_string(), |e| e.constant.clone());
        reject(&mut lines, &format!("!(AutoDC({}, {constant}, GetPCSpeaker()))", d.constant()));
    }
    section(&mut body, "Perform skill checks", std::mem::take(&mut lines));

    if let Some((n, m)) = c.random {
        reject(&mut lines, &format!("Random({m}) >= {n}"));
    }
    section(&mut body, "Add the randomness", std::mem::take(&mut lines));

    let mut out = String::new();
    header(&mut out, name, date);
    if !c.items.is_empty() || !c.skill_checks.is_empty() {
        out.push_str("#include \"nw_i0_tool\"\n\n");
    }
    let _ = write!(out, "int StartingConditional()\n{{\n{body}\n\treturn TRUE;\n}}\n");
    crlf(&out)
}

/// The action script (`main`), CRLF line endings.
pub fn action_script(name: &str, date: &str, a: &ActionScript) -> String {
    let plot = matches!(a.perform, Perform::Store { appraise: true, .. });
    let party = a.give_gold.is_some_and(|g| g.1) || a.give_xp.is_some_and(|x| x.1);
    let mut body = String::new();

    // Rewards.
    if let Some((gold, to_party)) = a.give_gold {
        let call = match (to_party, plot) {
            (false, _) => format!("GiveGoldToCreature(GetPCSpeaker(), {gold});"),
            (true, false) => format!("RewardPartyGP({gold}, GetPCSpeaker());"),
            (true, true) => format!("RewardGP({gold}, GetPCSpeaker());"),
        };
        let _ = write!(body, "\t// Give the speaker some gold\n\t{call}\n\n");
    }
    if let Some((xp, to_party)) = a.give_xp {
        let call = match (to_party, plot) {
            (false, _) => format!("\tGiveXPToCreature(GetPCSpeaker(), {xp});\n"),
            (true, false) => format!("\tRewardPartyXP({xp}, GetPCSpeaker());\n"),
            (true, true) => format!(
                "\tobject oPartyMember = GetFirstFactionMember(GetPCSpeaker(), TRUE);\n\
                 \twhile(GetIsObjectValid(oPartyMember))\n\
                 \t{{\n\
                 \t\tGiveXPToCreature(oPartyMember, {xp});\n\
                 \t\toPartyMember = GetNextFactionMember(GetPCSpeaker(), TRUE);\n\
                 \t}}\n"
            ),
        };
        let _ = write!(body, "\t// Give the speaker some XP\n{call}\n");
    }
    if !a.give_items.is_empty() {
        body.push_str("\t// Give the speaker the items\n");
        for item in &a.give_items {
            let _ = writeln!(body, "\tCreateItemOnObject({}, GetPCSpeaker(), 1);", quote(item));
        }
        body.push('\n');
    }

    // Take.
    if a.take_gold.is_some() || a.take_xp.is_some() || !a.take_items.is_empty() {
        body.push('\n');
    }
    if let Some((gold, destroy)) = a.take_gold {
        let destroy = if destroy { "TRUE" } else { "FALSE" };
        let _ = write!(
            body,
            "\t// Remove some gold from the player\n\
             \tTakeGoldFromCreature({gold}, GetPCSpeaker(), {destroy});\n\n"
        );
    }
    if let Some(xp) = a.take_xp {
        let _ = write!(
            body,
            "\t// Remove some xp from the player\n\
             \tint nXP = GetXP(GetPCSpeaker()) - {xp};\n\
             \tif(nXP < 0)\n\
             \t\tnXP = 0;\n\
             \tSetXP(GetPCSpeaker(), nXP);\n\n"
        );
    }
    if !a.take_items.is_empty() {
        body.push_str("\t// Remove items from the player's inventory\n\tobject oItemToTake;\n");
        let take = if a.destroy_items {
            "DestroyObject(oItemToTake);"
        } else {
            "ActionTakeItem(oItemToTake, GetPCSpeaker());"
        };
        for tag in &a.take_items {
            let _ = write!(
                body,
                "\toItemToTake = GetItemPossessedBy(GetPCSpeaker(), {});\n\
                 \tif(GetIsObjectValid(oItemToTake) != 0)\n\
                 \t\t{take}\n",
                quote(tag)
            );
        }
    }

    // Locals.
    let mut sets: Vec<String> = a.locals.iter().filter_map(LocalSet::statement).collect();
    sets.sort_by_cached_key(|s| (s.to_lowercase(), s.clone()));
    if !sets.is_empty() {
        body.push_str("\t// Set the variables\n");
        for s in &sets {
            let _ = writeln!(body, "\t{s}");
        }
        body.push('\n');
    }

    // Perform.
    let faction = if a.perform == Perform::Attack { 0 } else { a.faction.clamp(-100, 100) };
    if a.perform != Perform::Nothing || faction != 0 {
        body.push('\n');
    }
    match &a.perform {
        Perform::Nothing => {}
        Perform::Attack => body.push_str(
            "\t// Set the faction to hate the player, then attack the player\n\
             \tAdjustReputation(GetPCSpeaker(), OBJECT_SELF, -100);\n\
             \tDetermineCombatRound(GetPCSpeaker());\n",
        ),
        Perform::Store { tag, appraise } => {
            let open = if *appraise { "gplotAppraiseOpenStore" } else { "OpenStore" };
            let _ = write!(
                body,
                "\t// Either open the store with that tag or let the user know that no store exists.\n\
                 \tobject oStore = GetNearestObjectByTag({});\n\
                 \tif(GetObjectType(oStore) == OBJECT_TYPE_STORE)\n\
                 \t\t{open}(oStore, GetPCSpeaker());\n\
                 \telse\n\
                 \t\tActionSpeakStringByStrRef(53090, TALKVOLUME_TALK);\n\n",
                quote(tag)
            );
        }
    }
    if faction != 0 {
        let _ = write!(
            body,
            "\t// Modify the player's reputation\n\
             \tAdjustReputation(GetPCSpeaker(), OBJECT_SELF, {faction});\n"
        );
    }

    let mut out = String::new();
    header(&mut out, name, date);
    if party && !plot {
        out.push_str("#include \"nw_i0_tool\"\n\n");
    }
    if plot {
        out.push_str("#include \"nw_i0_plot\"\n\n");
    }
    if a.perform == Perform::Attack {
        out.push_str("#include \"nw_i0_generic\"\n\n");
    }
    let _ = write!(out, "void main()\n{{\n{body}}}\n");
    crlf(&out)
}

/// The first free wizard script name: `sc_001`, `sc_002`, ... for
/// conditions, `at_001`, ... for actions (`taken` names in use).
pub fn default_name(condition: bool, taken: impl Fn(&str) -> bool) -> String {
    let prefix = if condition { "sc" } else { "at" };
    (1..).map(|n| format!("{prefix}_{n:03}")).find(|name| !taken(name)).expect("a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(row: usize, name: &str, constant: &str) -> Entry {
        Entry { row, name: name.into(), constant: constant.into() }
    }

    fn lists() -> Lists {
        Lists {
            classes: vec![
                entry(1, "Bard", "CLASS_TYPE_BARD"),
                entry(3, "Druid", "CLASS_TYPE_DRUID"),
            ],
            genders: vec![
                entry(2, "Both", "GENDER_BOTH"),
                entry(1, "Female", "GENDER_FEMALE"),
                entry(0, "Male", "GENDER_MALE"),
            ],
            player_races: vec![
                entry(0, "Dwarf", "RACIAL_TYPE_DWARF"),
                entry(1, "Elf", "RACIAL_TYPE_ELF"),
            ],
            other_races: vec![entry(8, "Animal", "RACIAL_TYPE_ANIMAL")],
            feats: vec![entry(0, "Alertness", "FEAT_ALERTNESS")],
            skills: vec![entry(5, "Hide", "SKILL_HIDE"), entry(20, "Appraise", "SKILL_APPRAISE")],
        }
    }

    #[test]
    fn an_empty_condition_passes() {
        let s = condition_script("sc_001", "today", &Condition::default(), &lists());
        assert!(s.ends_with("int StartingConditional()\r\n{\r\n\r\n\treturn TRUE;\r\n}\r\n"));
        assert!(s.starts_with("//::///"));
        assert!(!s.contains("#include"));
    }

    #[test]
    fn single_and_several_choices() {
        let c = Condition {
            genders: Some(vec![1]),
            alignment: Some(Alignment { good_evil: [true, true, false], law_chaos: [false; 3] }),
            ..Default::default()
        };
        let s = condition_script("x", "d", &c, &lists());
        assert!(s.contains("if(GetGender(GetPCSpeaker()) != GENDER_FEMALE)"));
        assert!(s.contains("if(GetAlignmentGoodEvil(GetPCSpeaker()) == ALIGNMENT_EVIL)"));
        assert!(!s.contains("GetAlignmentLawChaos"));
        let c = Condition { genders: Some(vec![0, 1]), ..Default::default() };
        let s = condition_script("x", "d", &c, &lists());
        assert!(s.contains("== GENDER_BOTH"));
        assert!(!s.contains("GENDER_MALE") && !s.contains("GENDER_FEMALE"));
    }

    #[test]
    fn local_values_follow_their_type() {
        let check = |ty, v: &str| {
            LocalCheck {
                ty,
                name: "v".into(),
                compare: Compare::Less,
                value: Operand::Constant(v.into()),
            }
            .expression()
        };
        assert_eq!(
            check(VarType::Float, "1.5").unwrap(),
            "GetLocalFloat(GetPCSpeaker(), \"v\") < 1.500000"
        );
        assert_eq!(check(VarType::Int, "x"), None);
        assert_eq!(
            check(VarType::String, "say \"hi\"").unwrap(),
            "GetLocalString(GetPCSpeaker(), \"v\") < \"say \\\"hi\\\"\""
        );
    }

    #[test]
    fn default_names_skip_those_taken() {
        assert_eq!(default_name(true, |_| false), "sc_001");
        assert_eq!(default_name(false, |n| n == "at_001" || n == "at_002"), "at_003");
    }
}
