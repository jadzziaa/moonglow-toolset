//! Sounds a builder keeps under the name `.bmu` (a WAVE renamed so the
//! game reads it), in the hak editor's viewer. (The area view's music is in
//! `area_view.rs`.)

use crate::area_view::{mp3_in_wave_track, pcm_track};
use crate::common::*;

/// What the viewer makes of a PCM WAVE named `.bmu` (a sound with Play,
/// marked as one the game does not play, and one line in the log), of an
/// MP3 in a WAVE named `.bmu` (the game plays it: no mark), and of a
/// `.bmu` that holds no sound (the decoder's reason, no player).
#[test]
fn a_hak_editor_views_a_wave_named_bmu_and_marks_it() {
    let dir = mg_testkit::scratch_dir("ui-hak-view-bmu");
    let files = [
        ("mg_music.bmu", pcm_track()),
        ("mg_mp3.bmu", mp3_in_wave_track()),
        ("mg_note.bmu", b"this is a note, not a sound".to_vec()),
    ];
    let mut hak = mg_module::hak_edit::Hak::new();
    let sources = files
        .iter()
        .map(|(n, d)| {
            let key = ResKey::from_filename(n).unwrap();
            (key, mg_module::hak_edit::Source::Bytes(d.clone().into()))
        })
        .collect();
    hak.add_sources(sources).unwrap();
    let path = dir.join("mg_view_bmu.hak");
    hak.save(Some(&path)).unwrap();
    let mut h = harness(app_with(Vec::new()), 1400.0, 900.0);
    h.run();
    mg_ui::hak_view::open_at(h.state_mut(), &path);
    h.run();
    let view = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.get_all_by_label(name).next().unwrap().click_secondary();
        h.run();
        h.get_all_by_label("View").last().unwrap().click();
        h.run();
        h.run();
        let shown = h.state().haks[0].viewing().unwrap();
        assert_eq!(shown.0.to_string(), name);
        shown.1
    };
    let mark = "The game does not play a WAVE under .bmu: music must be MP3";
    let told = |h: &Harness<'_, Moonglow>| {
        h.state().log.entries.iter().filter(|(_, m)| m.contains("is a WAVE, which")).count()
    };
    assert_eq!(view(&mut h, "mg_music.bmu"), "sound");
    h.get_by_label_contains(mark);
    assert_eq!(told(&h), 1);
    h.get_by_label("Play").click();
    h.run();
    h.get_by_label("Stop");
    // Viewed twice: said once.
    assert_eq!(view(&mut h, "mg_music.bmu"), "sound");
    assert_eq!(told(&h), 1);
    // An MP3 in a WAVE (the form the game plays as music) is a sound too,
    // and the game plays it: no mark, no line.
    assert_eq!(view(&mut h, "mg_mp3.bmu"), "sound");
    assert!(h.query_by_label_contains(mark).is_none());
    assert_eq!(told(&h), 1);
    // One that is no sound shows the decoder's reason, not a player.
    assert_ne!(view(&mut h, "mg_note.bmu"), "sound");
    h.get_by_label_contains("neither WAVE nor MP3");
}
