//! A creature given as values (`Look::creature` of a `DressedLook`, what a
//! game client is told of a creature) is the creature read from its fields
//! (`Look::read`, an area's GIT), what it wears and holds included, where
//! the fields carry the feats the game asks of its items; and without
//! them, the reading leaves those items out, as the game does, while the
//! values draw what they are given. Reads the installed game; skipped
//! without one.

use glam::Vec3;
use mg_area::values::{Area, Look, Object};
use mg_area::{AreaModel, ObjectKind};
use mg_core::ResType;
use mg_gff::{Gff, Struct, Value};
use mg_preview::{CreatureLook, DressedLook, ItemLook};
use mg_resman::GameInstall;
use mg_rules::GameData;

fn game() -> Option<GameData> {
    let root = match mg_testkit::nwn_root() {
        Some(r) => r,
        None if mg_testkit::corpus_required() => {
            panic!("MOONGLOW_REQUIRE_CORPUS is set but no game install was found (set NWN_ROOT)")
        }
        None => return None,
    };
    let install = GameInstall::new(&root, None, "en");
    let rm = mg_testkit::shared_resman(&install).ok()?;
    let tlk = std::sync::Arc::new(GameData::open_tlk(&install).ok()?);
    Some(GameData::with_shared_tlk(rm, tlk))
}

/// A stock item of base item `base` with no properties (so no use
/// limitation) that a creature of no feat may not wear: the game asks a
/// feat of it.
fn needs_a_feat(game: &GameData, base: i64) -> Struct {
    let mut names = game.resman.list(ResType::UTI);
    names.sort_by_key(|r| r.to_string());
    names
        .into_iter()
        .filter_map(|r| Gff::read(&game.resman.get_named(r.as_str()?, ResType::UTI).ok()?).ok())
        .map(|g| g.root)
        .find(|it| {
            it.integer("BaseItem") == Some(base)
                && it.list("PropertiesList").unwrap_or(&[]).is_empty()
                && !game.missing_feats(it, &[]).is_empty()
        })
        .unwrap_or_else(|| panic!("no stock item of base item {base} that asks a feat"))
}

fn feat_list(feats: &[u16]) -> Value {
    let entry = |f: &u16| {
        let mut e = Struct::new(1);
        e.set("Feat", Value::Word(*f));
        e
    };
    Value::List(feats.iter().map(entry).collect())
}

#[test]
fn a_dressed_creature_from_values_is_the_creature_read_from_fields() {
    let Some(game) = game() else {
        eprintln!("skipped: no game install");
        return;
    };
    // A human in armor past cloth with a longsword and a tower shield
    // (base items 16, 1 and 57): each asks a proficiency feat.
    let (plate, sword, shield) =
        (needs_a_feat(&game, 16), needs_a_feat(&game, 1), needs_a_feat(&game, 57));
    let bare =
        DressedLook::from(CreatureLook { head: 2, colors: [3, 4, 0, 0], ..CreatureLook::new(6) });
    let dressed = DressedLook {
        armor: Some(ItemLook::from_uti(&plate)),
        right_hand: Some(ItemLook::from_uti(&sword)),
        left_hand: Some(ItemLook::from_uti(&shield)),
        ..bare.clone()
    };
    let feats: Vec<u16> =
        [&plate, &sword, &shield].iter().map(|it| game.missing_feats(it, &[])[0]).collect();
    let mut proficient = dressed.to_utc();
    proficient.id = ObjectKind::Creature.struct_id();
    proficient.set("FeatList", feat_list(&feats));
    for it in [&plate, &sword, &shield] {
        assert_eq!(game.wear_problem(&proficient, it), None);
    }

    // Values and fields: one look, the items drawn.
    let from_values = Look::creature(&game, dressed.clone());
    assert_eq!(from_values, Look::read(&game, ObjectKind::Creature, &proficient));
    let (Look::Model(clothed), Look::Model(naked)) =
        (&from_values, Look::creature(&game, bare.clone()))
    else {
        panic!("{from_values:?}")
    };
    assert!(
        clothed.parts.iter().filter(|p| p.item).count() >= 2,
        "the sword and shield are drawn: {:?}",
        clothed.parts.iter().map(|p| &p.model).collect::<Vec<_>>()
    );
    assert_ne!(clothed, &naked);

    // The fields without the feats: read as the game loads them, the
    // three items off; the values draw what they are given all the same.
    let mut unskilled = proficient.clone();
    unskilled.set("FeatList", Value::List(Vec::new()));
    assert_eq!(
        Look::read(&game, ObjectKind::Creature, &unskilled),
        Look::creature(&game, bare.clone())
    );

    // A bare body's look, borrowed (the call before DressedLook, still made) or given.
    let body = CreatureLook::new(6);
    #[allow(clippy::needless_borrows_for_generic_args)] // (The borrowed call is the one held.)
    let borrowed = Look::creature(&game, &body);
    assert_eq!(borrowed, Look::creature(&game, body));
    assert_eq!(borrowed, Look::Model(mg_preview::creature_look(&game, &body).unwrap()));

    // In an area: the creature placed from values is the GIT's.
    let mut placed = proficient.clone();
    for (l, v) in [("XPosition", 3.0f32), ("YPosition", 4.0), ("XOrientation", 1.0)] {
        placed.set(l, Value::Float(v));
    }
    let mut git = Struct::new(0xFFFF_FFFF);
    git.set(ObjectKind::Creature.list(), Value::List(vec![placed]));
    let read = AreaModel::read(&game, &Struct::new(0), &git, None);
    let creature = Object::facing(ObjectKind::Creature, Vec3::new(3.0, 4.0, 0.0), 0.0, from_values);
    let built =
        AreaModel::from_values(&game, Area { objects: vec![creature], ..Area::default() }, None);
    assert_eq!(built.objects, read.objects);
}
