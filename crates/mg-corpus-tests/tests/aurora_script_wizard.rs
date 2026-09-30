//! The Script Wizard compared with Aurora's: Aurora's wizard, opened from
//! the Conversation Editor's Text Appears When… (`sc_*`) and Actions Taken
//! (`at_*`) tabs, was given the choices written out below as `Condition`
//! and `ActionScript` values (captures `script_wizard/*.nss`). Moonglow
//! writes the same scripts, and every script compiles. Aurora wrote "9" as
//! the date; the tests pass the same.
//!
//! `at_001` is the exception: Aurora's does not compile in EE (`nw_i0_tool`
//! and `nw_i0_plot` both define `HasItem`) and its "take XP" does nothing
//! (`GiveXPToCreature` ignores negative amounts), so Moonglow's differs in
//! exactly those places.

use mg_core::ResType;
use mg_module::script_wizard::{
    ActionScript, Alignment, ClassLevel, Compare, Condition, Difficulty, Lists, LocalCheck,
    LocalSet, Operand, Perform, VarType, action_script, condition_script,
};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_rules::GameData;
use mg_script::Compiler;
use mg_testkit::{aurora_capture, corpus};

fn capture(name: &str) -> Option<String> {
    let path = mg_testkit::aurora_capture(&format!("script_wizard/{name}.nss"))?;
    Some(String::from_utf8(std::fs::read(path).unwrap()).unwrap())
}

/// Compiles a script; the error message on failure.
fn compile(rm: &ResMan, name: &str, source: &str) -> Result<(), String> {
    let mut c = Compiler::new(|n, t| {
        if t == ResType::NSS && n == name {
            return Some(source.as_bytes().to_vec());
        }
        ResKey::parse(n, t).and_then(|k| rm.get(&k).ok()).map(|d| d.into_owned())
    });
    c.compile(name).map(|_| ()).map_err(|e| e.message)
}

fn row(list: &[mg_module::script_wizard::Entry], name: &str) -> usize {
    list.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("{name} not listed")).row
}

fn check(name: &str, value: Operand, ty: VarType, compare: Compare) -> LocalCheck {
    LocalCheck { ty, name: name.into(), compare, value }
}

#[test]
fn script_wizard_matches_aurora() {
    let root = corpus!();
    let _ = aurora_capture!("script_wizard/sc_001.nss");
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let lists = Lists::load(&game);
    let c = Operand::Constant;
    let v = |s: &str| Operand::Variable(s.into());

    // The lists as Aurora showed them.
    assert_eq!(
        lists.player_races.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["Dwarf", "Elf", "Gnome", "Half-Elf", "Halfling", "Half-Orc", "Human"]
    );
    assert_eq!(lists.other_races.len(), 18);
    assert_eq!(
        lists.genders.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["Both", "Female", "Male", "None", "Other"]
    );
    assert_eq!(
        lists.feats.iter().take(3).map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["Air Domain Powers", "Alertness", "Ambidexterity"]
    );
    assert_eq!(lists.classes[0].name, "Arcane Archer");
    assert_eq!(lists.skills.last().unwrap().name, "Ride");

    let sc_001 = Condition {
        abilities: vec![(0, Compare::Greater, 12), (5, Compare::Equal, 8)],
        classes: vec![
            ClassLevel { class: Some(row(&lists.classes, "Bard")), level: Some(3) },
            ClassLevel { class: Some(row(&lists.classes, "Druid")), level: None },
        ],
        all_classes: false,
        genders: Some(vec![row(&lists.genders, "Female")]),
        races: Some(vec![row(&lists.player_races, "Elf"), row(&lists.other_races, "Aberration")]),
        alignment: Some(Alignment {
            good_evil: [true, true, false],
            law_chaos: [true, false, false],
        }),
        feats: vec![row(&lists.feats, "Alertness")],
        skills: vec![row(&lists.skills, "Bluff")],
        skill_checks: vec![(Difficulty::Normal, row(&lists.skills, "Persuade"))],
        items: vec!["key_1".into(), "Gem_A".into()],
        locals: vec![
            check("nQuest", c("2".into()), VarType::Int, Compare::Equal),
            check("sName", v("sOther"), VarType::String, Compare::Equal),
            check("fLevel", c("1.5".into()), VarType::Float, Compare::Less),
            check("nGold", v("nPrice"), VarType::Int, Compare::Greater),
        ],
        random: Some((3, 20)),
    };
    let sc_002 = Condition {
        abilities: vec![(1, Compare::NotEqual, 10), (4, Compare::Less, 14)],
        classes: vec![
            ClassLevel { class: None, level: Some(5) },
            ClassLevel { class: Some(row(&lists.classes, "Fighter")), level: None },
        ],
        all_classes: true,
        genders: Some(vec![row(&lists.genders, "Female"), row(&lists.genders, "Male")]),
        races: Some(vec![row(&lists.player_races, "Human")]),
        alignment: Some(Alignment { good_evil: [false; 3], law_chaos: [true, false, true] }),
        feats: vec![row(&lists.feats, "Ambidexterity"), row(&lists.feats, "Alertness")],
        skills: vec![row(&lists.skills, "Hide"), row(&lists.skills, "Appraise")],
        skill_checks: vec![
            (Difficulty::Easy, row(&lists.skills, "Hide")),
            (Difficulty::Hard, row(&lists.skills, "Spot")),
        ],
        ..Default::default()
    };
    let at_001 = ActionScript {
        give_gold: Some((50, false)),
        give_xp: Some((100, true)),
        give_items: vec!["nw_wswls001".into(), "nw_it_mpotion001".into()],
        take_gold: Some((25, true)),
        take_xp: Some(10),
        take_items: vec!["key_1".into(), "Gem_A".into()],
        destroy_items: false,
        locals: vec![
            LocalSet { ty: VarType::Int, name: "nQuest".into(), value: c("3".into()) },
            LocalSet { ty: VarType::String, name: "sName".into(), value: c("Bob".into()) },
            LocalSet { ty: VarType::Float, name: "fX".into(), value: v("fY") },
        ],
        perform: Perform::Store { tag: "store_1".into(), appraise: true },
        faction: -20,
    };
    let at_002 = ActionScript {
        give_gold: Some((50, true)),
        give_xp: Some((100, false)),
        take_gold: Some((25, false)),
        take_items: vec!["key_1".into()],
        destroy_items: true,
        perform: Perform::Attack,
        faction: -100,
        ..Default::default()
    };
    let at_003 = ActionScript {
        perform: Perform::Store { tag: "store_2".into(), appraise: false },
        faction: 30,
        ..Default::default()
    };

    let mut scripts = vec![
        ("sc_001", condition_script("sc_001", "9", &sc_001, &lists)),
        ("sc_002", condition_script("sc_002", "9", &sc_002, &lists)),
        ("at_001", action_script("at_001", "9", &at_001)),
        ("at_002", action_script("at_002", "9", &at_002)),
        ("at_003", action_script("at_003", "9", &at_003)),
    ];
    for (name, ours) in &scripts {
        let Some(mut aurora) = capture(name) else { continue };
        if *name == "at_001" {
            assert!(
                compile(&game.resman, name, &aurora).unwrap_err().contains("DUPLICATE FUNCTION"),
                "Aurora's at_001 does not compile"
            );
            aurora = aurora
                .replace("#include \"nw_i0_tool\"\r\n\r\n", "")
                .replace(
                    "\tRewardPartyXP(100, GetPCSpeaker());\r\n",
                    "\tobject oPartyMember = GetFirstFactionMember(GetPCSpeaker(), TRUE);\r\n\
                     \twhile(GetIsObjectValid(oPartyMember))\r\n\
                     \t{\r\n\
                     \t\tGiveXPToCreature(oPartyMember, 100);\r\n\
                     \t\toPartyMember = GetNextFactionMember(GetPCSpeaker(), TRUE);\r\n\
                     \t}\r\n",
                )
                .replace(
                    "\tGiveXPToCreature(GetPCSpeaker(), -10);\r\n",
                    "\tint nXP = GetXP(GetPCSpeaker()) - 10;\r\n\
                     \tif(nXP < 0)\r\n\
                     \t\tnXP = 0;\r\n\
                     \tSetXP(GetPCSpeaker(), nXP);\r\n",
                );
        }
        assert_eq!(ours, &aurora, "{name}");
    }

    // Every combination of the action page's includes compiles.
    for (appraise, attack, party) in
        [(true, false, true), (false, true, true), (false, false, false), (true, false, false)]
    {
        let a = ActionScript {
            give_gold: Some((5, party)),
            give_xp: Some((5, party)),
            take_xp: Some(1),
            perform: if attack {
                Perform::Attack
            } else {
                Perform::Store { tag: "s".into(), appraise }
            },
            ..Default::default()
        };
        scripts.push(("mg_combo", action_script("mg_combo", "today", &a)));
    }
    for (name, source) in &scripts {
        if let Err(e) = compile(&game.resman, name, source) {
            panic!("{name} does not compile: {e}\n{source}");
        }
    }
}
