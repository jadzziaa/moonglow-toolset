//! Where the game plays a sound: the names it looks for in each role and
//! the forms it plays there. These rules live here alone: the window's log
//! (`mg-ui`'s `warn_silent`) and Verify's `sound-silent` check (`mg-module`)
//! both ask [`silent_in_game`], and Verify's missing references look for a
//! sound under [`Role::looked_for`].
//!
//! Read in the real client (`docs/findings.md`, "What the game plays as
//! music and as sounds"; the hand-run test `client_audio.rs` of
//! `mg-corpus-tests`):
//! - music (an `ambientmusic.2da` row an area plays by day or night, and
//!   its battle music, started by script) is looked for only as
//!   `NAME.bmu`, and played if it holds MP3;
//! - an ambient sound (an `ambientsound.2da` row) is looked for as
//!   `NAME.wav`, else `NAME.bmu`; a `.wav` plays if it holds a WAVE, a `.bmu`
//!   if it holds MP3;
//! - a sound `PlaySound` plays, and a placed sound object's, is looked for
//!   only as `NAME.wav`, and played if it holds a WAVE or MP3 behind the
//!   `BMU V1.0` header (not MP3 without it); a placed sound that is
//!   positional plays no stereo WAVE, PCM and IMA ADPCM alike, at any
//!   distance tried (min 1, max 10; 50 and 100), and an area-wide one plays
//!   the same files with both channels (the game's own `lavalakeloop.uts`
//!   and `sludgelake.uts` are silent as shipped so).
//!
//! Not read in the client: battle music in real combat, a conversation
//! line's sound (taken to be `PlaySound`'s), stereo MP3 by `PlaySound` or a
//! placed sound. When the client says more, the arms of
//! [`Role::looked_for`], [`silent_in_game`] and [`silent_positional`] are
//! what change.

use mg_core::ResType;

use crate::Form;

/// What a sound is for, as far as the game's lookup and playing go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// An `ambientmusic.2da` row an area plays by day or night (its
    /// `MusicDay`, `MusicNight`).
    Music,
    /// An area's battle music (`MusicBattle`, an `ambientmusic.2da` row):
    /// read as music, started by script (`MusicBattlePlay`).
    BattleMusic,
    /// An `ambientsound.2da` row (an area's `AmbientSndDay`,
    /// `AmbientSndNight`).
    Ambient,
    /// A sound a script plays (`PlaySound`, read in the client), and the
    /// other sounds taken to be played as it is: a conversation line's (not
    /// read), a Play button's.
    Other,
    /// A placed sound object's or a sound blueprint's sounds: read in the
    /// client, as `PlaySound`'s, and a positional one plays no stereo WAVE
    /// ([`silent_positional`]).
    Placed,
    /// A file shown in the hak editor's View, which has no role.
    Viewed,
}

impl Role {
    pub const ALL: [Role; 6] =
        [Role::Music, Role::BattleMusic, Role::Ambient, Role::Other, Role::Placed, Role::Viewed];

    /// The types the game looks for a sound of this role under, in order.
    /// (The hak editor's View has no role: it looks for none.)
    pub fn looked_for(self) -> &'static [ResType] {
        match self {
            Role::Music | Role::BattleMusic => &[ResType::BMU],
            Role::Ambient => &[ResType::WAV, ResType::BMU],
            Role::Other | Role::Placed => &[ResType::WAV],
            Role::Viewed => &[],
        }
    }

    /// The types Moonglow looks for a sound of this role under, in order:
    /// those the game looks for first, then the other, which Moonglow also
    /// plays (music as `.wav`, a sound as `.bmu`), and of which
    /// [`silent_in_game`] says the game finds nothing.
    pub fn lookup(self) -> [ResType; 2] {
        match self {
            Role::Music | Role::BattleMusic => [ResType::BMU, ResType::WAV],
            _ => [ResType::WAV, ResType::BMU],
        }
    }
}

/// Why the game would stay silent on a file that Moonglow plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Silent {
    /// Music that is a WAVE under `.bmu`.
    MusicWave,
    /// Music that exists only as `.wav`.
    MusicOnlyWav,
    /// An ambient sound that is MP3 under `.wav`.
    AmbientMp3,
    /// An ambient sound that is a WAVE under `.bmu`.
    AmbientWave,
    /// A sound that exists only as `.bmu`.
    OnlyBmu,
    /// A sound (`PlaySound`'s, a placed sound's) that is MP3 under `.wav`
    /// without the `BMU V1.0` header.
    HeaderlessMp3,
    /// A placed sound that is positional, of a stereo WAVE.
    StereoPositional,
    /// A WAVE under `.bmu`, seen without a role: played in none.
    ViewedWave,
}

impl Silent {
    /// The window log's line about `name`.
    pub fn message(self, name: mg_core::ResRef) -> String {
        match self {
            Silent::MusicWave => format!(
                "Music {name} is a WAVE under .bmu, which the game does not play: music must be MP3"
            ),
            Silent::MusicOnlyWav => format!(
                "Music {name} exists only as a .wav, which the game does not look for: music must be a .bmu"
            ),
            Silent::AmbientMp3 => format!(
                "Ambient sound {name} is MP3 under .wav, which the game does not play: it plays a WAVE there"
            ),
            Silent::AmbientWave => format!(
                "Ambient sound {name} is a WAVE under .bmu, which the game does not play: it plays MP3 there"
            ),
            Silent::OnlyBmu => format!(
                "Sound {name} exists only as a .bmu, which the game does not look for: sounds must be a .wav"
            ),
            Silent::HeaderlessMp3 => format!(
                "Sound {name} is MP3 under .wav without the BMU V1.0 header, which the game does not play"
            ),
            Silent::StereoPositional => format!(
                "Sound {name} is a stereo WAVE, which the game does not play from a positional sound"
            ),
            Silent::ViewedWave => format!(
                "{name}.bmu is a WAVE, which the game does not play under .bmu: music must be MP3"
            ),
        }
    }
}

/// Whether the game stays silent on a file it would not play in a role,
/// and why: `found` is the type the file was found as (by the lookup in
/// [`Role::lookup`] order, so for music a `.wav` means there is no `.bmu`,
/// and for the others a `.bmu` means there is no `.wav`), `form` what it
/// holds ([`crate::form`]).
pub fn silent_in_game(role: Role, found: ResType, form: Form) -> Option<Silent> {
    use Form::{BmuMp3, Mp3, Wave};
    match (role, found, form) {
        // The client never finds a music track named .wav.
        (Role::Music | Role::BattleMusic, ResType::WAV, _) => Some(Silent::MusicOnlyWav),
        // It finds the .bmu and plays it only as MP3.
        (Role::Music | Role::BattleMusic, ResType::BMU, Wave) => Some(Silent::MusicWave),
        (Role::Music | Role::BattleMusic, ..) => None,
        // MP3 under .wav is silent, a WAVE plays.
        (Role::Ambient, ResType::WAV, Mp3 | BmuMp3) => Some(Silent::AmbientMp3),
        // Only a .bmu with MP3 plays as an ambient sound.
        (Role::Ambient, ResType::BMU, Wave) => Some(Silent::AmbientWave),
        (Role::Ambient, ..) => None,
        // Only NAME.wav is looked for (PlaySound, placed sound objects).
        (Role::Other | Role::Placed, ResType::BMU, _) => Some(Silent::OnlyBmu),
        // An MP3 there needs the header.
        (Role::Other | Role::Placed, ResType::WAV, Mp3) => Some(Silent::HeaderlessMp3),
        (Role::Other | Role::Placed, ..) => None,
        // A WAVE under .bmu is played in no role.
        (Role::Viewed, ResType::BMU, Wave) => Some(Silent::ViewedWave),
        (Role::Viewed, ..) => None,
    }
}

/// Whether the game stays silent on a sound of a placed sound object
/// because of where it plays it: a positional one plays no WAVE of two or
/// more channels (`channels`, from its `fmt `), an area-wide one does. Read
/// in the client for PCM and IMA ADPCM, so for every WAVE codec here; the
/// distances were ruled out (silent at the stock 1 to 10 and at 50 and
/// 100). Stereo MP3 was not settled in the client: nothing is said of it.
pub fn silent_positional(
    role: Role,
    form: Form,
    channels: Option<u16>,
    positional: bool,
) -> Option<Silent> {
    let stereo = channels.is_some_and(|c| c >= 2);
    (role == Role::Placed && positional && form == Form::Wave && stereo)
        .then_some(Silent::StereoPositional)
}

#[cfg(test)]
mod tests {
    use super::*;
    use Form::{BmuMp3, Mp3, Wave};
    use Role::*;
    const BMU: ResType = ResType::BMU;
    const WAV: ResType = ResType::WAV;

    /// The whole matrix: the roles by two types by two containers.
    #[test]
    fn the_game_is_silent_where_the_client_was() {
        let cases = [
            // Music: only NAME.bmu is looked for, and plays as MP3.
            (Music, BMU, BmuMp3, None),
            (Music, BMU, Mp3, None),
            (Music, BMU, Wave, Some(Silent::MusicWave)),
            (Music, WAV, BmuMp3, Some(Silent::MusicOnlyWav)),
            (Music, WAV, Mp3, Some(Silent::MusicOnlyWav)),
            (Music, WAV, Wave, Some(Silent::MusicOnlyWav)),
            // Battle music: as music (started by script).
            (BattleMusic, BMU, BmuMp3, None),
            (BattleMusic, BMU, Mp3, None),
            (BattleMusic, BMU, Wave, Some(Silent::MusicWave)),
            (BattleMusic, WAV, BmuMp3, Some(Silent::MusicOnlyWav)),
            (BattleMusic, WAV, Mp3, Some(Silent::MusicOnlyWav)),
            (BattleMusic, WAV, Wave, Some(Silent::MusicOnlyWav)),
            // Ambient: .wav (else .bmu); a .wav plays as a WAVE, a .bmu as MP3.
            (Ambient, WAV, Wave, None),
            (Ambient, WAV, BmuMp3, Some(Silent::AmbientMp3)),
            (Ambient, WAV, Mp3, Some(Silent::AmbientMp3)),
            (Ambient, BMU, BmuMp3, None),
            (Ambient, BMU, Mp3, None),
            (Ambient, BMU, Wave, Some(Silent::AmbientWave)),
            // PlaySound and placed sounds: only NAME.wav, a WAVE or MP3
            // behind the header.
            (Other, WAV, Wave, None),
            (Other, WAV, BmuMp3, None),
            (Other, WAV, Mp3, Some(Silent::HeaderlessMp3)),
            (Other, BMU, BmuMp3, Some(Silent::OnlyBmu)),
            (Other, BMU, Mp3, Some(Silent::OnlyBmu)),
            (Other, BMU, Wave, Some(Silent::OnlyBmu)),
            (Placed, WAV, Wave, None),
            (Placed, WAV, BmuMp3, None),
            (Placed, WAV, Mp3, Some(Silent::HeaderlessMp3)),
            (Placed, BMU, BmuMp3, Some(Silent::OnlyBmu)),
            (Placed, BMU, Mp3, Some(Silent::OnlyBmu)),
            (Placed, BMU, Wave, Some(Silent::OnlyBmu)),
            // Viewed: a WAVE under .bmu plays in no role.
            (Viewed, BMU, Wave, Some(Silent::ViewedWave)),
            (Viewed, BMU, BmuMp3, None),
            (Viewed, BMU, Mp3, None),
            (Viewed, WAV, Wave, None),
            (Viewed, WAV, BmuMp3, None),
            (Viewed, WAV, Mp3, None),
        ];
        for (role, found, form, want) in cases {
            assert_eq!(silent_in_game(role, found, form), want, "{role:?} {found:?} {form:?}");
        }
        // Every role, type and form is in the table.
        assert_eq!(cases.len(), Role::ALL.len() * 2 * Form::ALL.len());
    }

    /// A positional placed sound plays no stereo WAVE; an area-wide one,
    /// mono, MP3 (not settled) and the other roles are not said.
    #[test]
    fn a_positional_sound_is_silent_on_a_stereo_wave() {
        let stereo = Some(Silent::StereoPositional);
        assert_eq!(silent_positional(Placed, Wave, Some(2), true), stereo);
        assert_eq!(silent_positional(Placed, Wave, Some(6), true), stereo);
        assert_eq!(silent_positional(Placed, Wave, Some(2), false), None);
        assert_eq!(silent_positional(Placed, Wave, Some(1), true), None);
        assert_eq!(silent_positional(Placed, Wave, None, true), None);
        assert_eq!(silent_positional(Placed, BmuMp3, Some(2), true), None);
        for role in [Music, BattleMusic, Ambient, Other, Viewed] {
            assert_eq!(silent_positional(role, Wave, Some(2), true), None, "{role:?}");
        }
    }

    #[test]
    fn music_looks_for_the_bmu_first() {
        assert_eq!(Music.lookup(), [BMU, WAV]);
        assert_eq!(BattleMusic.lookup(), [BMU, WAV]);
        assert_eq!(Ambient.lookup(), [WAV, BMU]);
        assert_eq!(Other.lookup(), [WAV, BMU]);
        assert_eq!(Placed.lookup(), [WAV, BMU]);
    }

    /// What the game looks for and what it is silent on agree: a file of a
    /// type the role doesn't look for is silent whatever it holds, and one
    /// it looks for first is silent only for what it holds.
    #[test]
    fn a_type_not_looked_for_is_silent_whatever_it_holds() {
        for role in Role::ALL.into_iter().filter(|r| *r != Viewed) {
            assert_eq!(role.looked_for()[0], role.lookup()[0], "{role:?}");
            for t in role.lookup() {
                let looked = role.looked_for().contains(&t);
                let silent = Form::ALL.map(|f| silent_in_game(role, t, f).is_some());
                if looked {
                    assert!(silent.contains(&false), "{role:?} {t:?}: something plays");
                } else {
                    assert_eq!(silent, [true; 3], "{role:?} {t:?}");
                }
            }
        }
        assert!(Viewed.looked_for().is_empty());
    }

    #[test]
    fn the_messages_are_one_sentence_each() {
        let name = mg_core::ResRef::from_str("mg_track").unwrap();
        for r in [
            Silent::MusicWave,
            Silent::MusicOnlyWav,
            Silent::AmbientMp3,
            Silent::AmbientWave,
            Silent::OnlyBmu,
            Silent::HeaderlessMp3,
            Silent::StereoPositional,
            Silent::ViewedWave,
        ] {
            let m = r.message(name);
            assert!(m.contains("mg_track") && !m.contains(';') && !m.ends_with('.'), "{m}");
        }
    }
}
