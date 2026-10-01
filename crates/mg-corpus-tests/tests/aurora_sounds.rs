//! Sound Properties against Aurora: six placed "animalcriesday" sounds
//! (`examples/sound_probe_module.rs`, `sounds/before.mod`), each given a
//! play style and positioning in Aurora's Sound Properties and saved
//! (`sounds/after.mod`): Once, Repeating and (cut to one sound) Seamlessly
//! looping, each everywhere in the area and from a specific position.

use mg_core::ResType;
use mg_module::Module;
use mg_module::blueprints::{SOUND_PLAY_STYLES, sound_priority};
use mg_resman::ResKey;
use mg_testkit::aurora_capture;

#[test]
fn play_styles_and_priorities_match_aurora() {
    let after = Module::open(&aurora_capture!("sounds/after.mod")).unwrap();
    let git = after.gff(&ResKey::parse("field", ResType::GIT).unwrap()).unwrap().unwrap();
    let sounds = git.root.list("SoundList").unwrap();
    // (tag, play style, from a specific position)
    let set = [
        ("SND0", 0, false),
        ("SND1", 0, true),
        ("SND2", 1, false),
        ("SND3", 1, true),
        ("SND4", 2, false),
        ("SND5", 2, true),
    ];
    for (tag, style, positional) in set {
        let s = sounds.iter().find(|s| s.string("Tag") == Some(tag.as_bytes())).unwrap();
        let int = |l: &str| s.integer(l).unwrap();
        let (_, looping, continuous) = SOUND_PLAY_STYLES[style];
        assert_eq!(
            (int("Looping"), int("Continuous")),
            (looping.into(), continuous.into()),
            "{tag}"
        );
        assert_eq!(int("Positional"), i64::from(positional), "{tag}");
        assert_eq!(int("RandomPosition"), 0, "{tag}");
        assert_eq!(int("Priority"), i64::from(sound_priority(looping, positional)), "{tag}");
        if looping {
            assert_eq!(int("Random"), 0, "{tag}: seamless looping plays in order");
            assert_eq!(s.list("Sounds").unwrap().len(), 1, "{tag}");
        }
    }
}
