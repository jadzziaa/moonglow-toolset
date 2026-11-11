//! A creature's parts that are effects: CEP's `@VFX` tails and wings (the
//! game's effect models, emitters and no mesh, named by tailmodel.2da and
//! wingmodel.2da), a held torch's flame. Their particles are drawn in the
//! area view, the creature's preview and its picture, as the client draws
//! them (`mg-corpus-tests`' `client_render/part_emitters.rs`).

use crate::common::*;

/// A user folder whose override has a tailmodel.2da whose row 1 is
/// `mg_tailfx`, an effect of one emitter throwing magenta squares upward
/// (no mesh): ours, as the game's own and CEP's are never committed.
fn user_with_effect_tail(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let user = user_folder(&dir, &["override"]);
    let over = user.join("override");
    let table = "2DA V2.0\n\n LABEL MODEL ENVMAP\n0 None **** ****\n1 Effect mg_tailfx ****\n";
    std::fs::write(over.join("tailmodel.2da"), table).unwrap();
    let model = "newmodel mg_tailfx\nsetsupermodel mg_tailfx NULL\nclassification effects\n\
        setanimationscale 1\nbeginmodelgeom mg_tailfx\nnode dummy mg_tailfx\n  parent NULL\nendnode\n\
        node emitter jet\n  parent mg_tailfx\n  update Fountain\n  render Normal\n  blend Normal\n\
        texture mg_magenta\n  xgrid 1\n  ygrid 1\n  birthrate 20\n  lifeExp 1.5\n  velocity 1\n\
        sizeStart 0.2\n  sizeEnd 0.2\n  alphaStart 1\n  alphaEnd 1\n  colorStart 1 1 1\n\
        colorEnd 1 1 1\nendnode\nendmodelgeom mg_tailfx\ndonemodel mg_tailfx\n";
    std::fs::write(over.join("mg_tailfx.mdl"), model).unwrap();
    let mut tga = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
    for _ in 0..64 {
        tga.extend([255, 0, 255, 255]);
    }
    std::fs::write(over.join("mg_magenta.tga"), tga).unwrap();
    user
}

/// The commoner (`nw_commale`) made a human (appearance 6, head 1) with
/// tail `tail`, as the module's blueprint `resref`.
fn tailed(game: &mg_rules::GameData, resref: &str, tail: u32) -> Gff {
    let data = game.resman.get(&ResKey::parse("nw_commale", ResType::UTC).unwrap()).unwrap();
    let mut bp = Gff::read(&data).unwrap();
    bp.root.set("Appearance_Type", mg_gff::Value::Word(6));
    bp.root.set("Appearance_Head", mg_gff::Value::Byte(1));
    bp.root.set("Tail_New", mg_gff::Value::Dword(tail));
    bp.root.set("TemplateResRef", mg_gff::Value::resref(ResRef::from_str(resref).unwrap()));
    bp
}

/// How many of a picture's pixels are the effect's magenta.
fn magenta(img: &image::RgbaImage) -> usize {
    img.pixels().filter(|p| p[0] > 180 && p[1] < 90 && p[2] > 180).count()
}

/// Runs the harness until `done` holds, at most ten seconds.
fn until(h: &mut Harness<'static, Moonglow>, done: &dyn Fn(&Harness<'static, Moonglow>) -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !done(h) && std::time::Instant::now() < deadline {
        h.run_steps(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

/// A creature wearing an effect as its tail shows the effect's particles
/// in the area view, in its preview (the placed creature's model view, as
/// its Properties show it) and in its blueprint's picture; a creature
/// without one shows none. (Until 2026-11-10 only a creature's base model's
/// emitters were simulated: a builder's report, "With CEP you can attach
/// visual effects as tail to a creature. These seem to be not rendered in
/// Moonglow".)
#[test]
fn an_effect_worn_as_a_tail_shows_its_particles() {
    use mg_module::instances::{Placement, Placing, instance};
    let user = user_with_effect_tail("ui-part-emitters-user");
    let prepare = |m: &mut Module, game: &mg_rules::GameData, area: ResRef| {
        for (resref, tail) in [("mg_tailed", 1), ("mg_plain", 0)] {
            let key = ResKey::parse(resref, ResType::UTC).unwrap();
            m.set_gff(key, &tailed(game, resref, tail)).unwrap();
        }
        let git_key = ResKey::new(area, ResType::GIT);
        let mut git = m.gff(&git_key).unwrap().unwrap();
        // (Its clothes, of its blueprint's equipment: the body under them.)
        let item = |r: ResRef| {
            let data = game.resman.get(&ResKey::new(r, ResType::UTI)).ok()?;
            Gff::read(&data).ok().map(|g| g.root)
        };
        let placing = Placing { game, item: &item };
        let at = Placement { position: [15.0, 20.0, 0.0], rotation: 0.0 };
        let c = instance(&placing, ResType::UTC, &tailed(game, "mg_tailed", 1).root, at, &[]);
        git.root.set("Creature List", mg_gff::Value::List(vec![c.unwrap()]));
        m.set_gff(git_key, &git).unwrap();
        // (In the module's palette: its pictures.)
        mg_module::palette::rebuild_custom_palettes(m, game).unwrap();
    };
    let Some((mut h, area)) =
        area_harness_with("part-emitters", "ttr01", None, 4, Some(user), &prepare)
    else {
        return;
    };
    let out = mg_testkit::scratch_dir("ui-part-emitters");

    // The area view.
    until(&mut h, &|h| h.state().area_views[&area].particles_shown > 0);
    if let Some(o) = h.state_mut().area_views.get_mut(&area).unwrap().orbit.as_mut() {
        o.target = glam::Vec3::new(15.0, 20.0, 1.0);
        (o.distance, o.yaw, o.pitch) = (6.0, -1.57, 0.4);
    }
    h.run_steps(20);
    let img = h.render().expect("render");
    img.save(out.join("area.png")).unwrap();
    // (Each view's pictures are kept before what they show is checked.)
    let area_shown = (h.state().area_views[&area].particles_shown, magenta(&img));

    // The placed creature's preview.
    let path = mg_edit::GffPath::root().item("Creature List", 0);
    let tab = Tab::InstanceModel { area, path: path.clone() };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
    let source = mg_ui::model_view::Source::Instance { area, path };
    let emitting = |h: &Harness<'static, Moonglow>| {
        h.state().model_views.get(&source).map(|v| v.parts_emitting())
    };
    until(&mut h, &|h| emitting(h).is_some_and(|n| n > 0));
    let preview_emitting = emitting(&h);
    // (From behind, farther: the tail is at its back.)
    let view = h.state_mut().model_views.get_mut(&source).unwrap();
    view.orbit.yaw += std::f32::consts::PI;
    view.orbit.distance *= 1.6;
    h.run_steps(20);
    let img = h.render().expect("render");
    img.save(out.join("preview.png")).unwrap();
    let preview_seen = magenta(&img);

    // Its blueprint's picture, and the plain one's.
    close_windows(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    {
        let p = &mut h.state_mut().palette;
        p.kind = mg_module::palette::BlueprintKind::Creature;
        (p.tiles, p.custom) = (false, true);
    }
    h.state_mut().settings.palette_gallery = true;
    h.state_mut().settings.gallery_tile = Some(200);
    let mut seen = Vec::new();
    for name in ["mg_tailed", "mg_plain"] {
        h.state_mut().palette.filter = name.into();
        h.run_steps(2);
        until(&mut h, &|h| h.state().thumbnails.waiting() == 0);
        h.run_steps(4);
        let img = h.render().expect("render");
        img.save(out.join(format!("picture-{name}.png"))).unwrap();
        // (The palette's, the window's right fifth: the area view's own
        // particles are left of it.)
        let x = img.width() * 78 / 100;
        let right = image::imageops::crop_imm(&img, x, 0, img.width() - x, img.height());
        seen.push(magenta(&right.to_image()));
    }
    assert!(area_shown.0 > 0 && area_shown.1 > 50, "the area view's particles: {area_shown:?}");
    assert_eq!(preview_emitting, Some(1), "the tail's particles in its preview");
    assert!(preview_seen > 50, "the preview's particles seen: {preview_seen}");
    assert!(seen[0] > 50 && seen[1] == 0, "the picture's particles: {seen:?}");
}
