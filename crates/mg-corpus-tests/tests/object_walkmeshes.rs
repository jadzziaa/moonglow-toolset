//! Placeables' and doors' walkmeshes (`.pwk`, `.dwk`) as the area view draws
//! them, in every shipped area: each placed placeable or door whose model
//! has a walkmesh with surfaces gets its faces, around where it stands; and
//! a door's faces follow its state (closed, or open one way or the other).
//!
//! Around: within 35 m (Darkness over Daggerford's cliffs reach 31 m).
//! Tyrants of the Moonsea's `ptm_candle02.pwk` puts its mesh node at
//! (15, 70): whether the engine honors a placeable walkmesh's node position
//! isn't tested; Moonglow does, as it does for models.

use glam::Vec3;
use mg_area::walk::{ObjectWalkmeshes, Walkmesh};
use mg_area::{AreaModel, ObjectKind};
use mg_core::ResType;
use mg_gff::Gff;
use mg_mdl::walkmesh::DoorState;
use mg_module::Module;
use mg_resman::{GameInstall, LayerClass, ResKey, priority};
use mg_rules::GameData;
use mg_testkit::{bundled_modules, corpus};

#[test]
fn placed_placeables_and_doors_have_their_walkmeshes() {
    let root = corpus!();
    let install = GameInstall::new(&root, None, "en");
    let base = GameData::open(&install).unwrap();
    // A door's states differ; a placeable's walkmesh sits around it.
    let closed = Walkmesh::of_object(&base, "t_door01", Some(DoorState::Closed)).unwrap();
    let open = Walkmesh::of_object(&base, "t_door01", Some(DoorState::Open1)).unwrap();
    assert_ne!(closed.faces, open.faces);
    let armoire = Walkmesh::of_object(&base, "plc_a01", None).unwrap();
    assert!(armoire.min.truncate().length() < 3.0 && armoire.max.truncate().length() < 3.0);

    let (mut placed, mut with_faces, mut failures) = (0, 0, Vec::new());
    for path in bundled_modules(&root) {
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        let m = Module::open(&path).unwrap();
        let mut game = GameData::open(&install).unwrap();
        let haks = m.haks().unwrap();
        game.resman
            .add_haks(&install, &haks.iter().map(String::as_str).collect::<Vec<_>>())
            .unwrap();
        game.resman.add(priority::MODULE, "module", LayerClass::Erf, m.container());
        let mut walkmeshes = ObjectWalkmeshes::default();
        for area in m.areas().unwrap() {
            let gff = |t: ResType| Gff::read(&game.resman.get(&ResKey::new(area, t)).ok()?).ok();
            let (Some(are), Some(git)) = (gff(ResType::ARE), gff(ResType::GIT)) else { continue };
            let model = AreaModel::read(&game, &are.root, &git.root, None);
            let faces = walkmeshes.faces(&game, &model);
            for (i, o) in model.objects.iter().enumerate() {
                if !matches!(o.kind, ObjectKind::Placeable | ObjectKind::Door) {
                    continue;
                }
                let Some(p) = &o.preview else { continue };
                let door = (o.kind == ObjectKind::Door).then_some(match o.state {
                    1 => DoorState::Open1,
                    2 => DoorState::Open2,
                    _ => DoorState::Closed,
                });
                placed += 1;
                let has = Walkmesh::of_object(&game, &p.base.model, door).is_some();
                let mine: Vec<&([Vec3; 3], usize)> = faces.iter().filter(|f| f.1 == i).collect();
                if has != !mine.is_empty() {
                    failures.push(format!(
                        "{name}/{area} {}: walkmesh {has}, faces {}",
                        o.tag,
                        mine.len()
                    ));
                }
                if has {
                    with_faces += 1;
                    let far = mine
                        .iter()
                        .flat_map(|f| f.0)
                        .map(|c| (c - o.position).truncate().length())
                        .fold(0.0f32, f32::max);
                    if far > 35.0 && p.base.model != "ptm_candle02" {
                        failures.push(format!(
                            "{name}/{area} {} ({}): a face {far:.1} m away",
                            o.tag, p.base.model
                        ));
                    }
                }
            }
        }
    }
    println!("{placed} placeables and doors placed, {with_faces} with walkmeshes");
    assert!(with_faces > 1000, "{with_faces}");
    failures.truncate(30);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
