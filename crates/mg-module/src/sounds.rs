//! What the game plays of an area's music and ambient sounds and of its
//! placed sounds, and what it would be silent on: the rows an area names
//! (`MusicDay`, `MusicNight`, `MusicBattle` of `ambientmusic.2da`;
//! `AmbientSndDay`, `AmbientSndNight` of `ambientsound.2da`) and the sounds
//! a placed sound object or a sound blueprint lists, the file each is found
//! as in the load order, and what that file holds, told from its first
//! bytes. The rules of each role are `mg_audio`'s ([`mg_audio::role`]);
//! Verify's `sound-silent` check (`doctor.rs`) says what this finds.
//!
//! A row's stingers (`Stinger1` to `Stinger3`) are not looked at: what the
//! game does with them was not read in the client.

use std::collections::HashMap;
use std::sync::Mutex;

use mg_audio::{Form, Role, Silent};
use mg_core::{ResRef, ResType};
use mg_resman::{ResKey, ResMan};

/// The fields of an area's `AreaProperties` (in its GIT) that name a row of
/// a sound table: the field, the table and the role its sound plays in.
pub const AREA_SOUNDS: [(&str, &str, Role); 5] = [
    ("MusicDay", "ambientmusic", Role::Music),
    ("MusicNight", "ambientmusic", Role::Music),
    ("MusicBattle", "ambientmusic", Role::BattleMusic),
    ("AmbientSndDay", "ambientsound", Role::Ambient),
    ("AmbientSndNight", "ambientsound", Role::Ambient),
];

/// Why the game plays nothing of a sound in its role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unheard {
    /// Found under none of the names the role is looked for under (nor
    /// under the other, which Moonglow plays).
    Missing,
    /// Found as `found`, where the game does not look for it, in a form it
    /// does not play there, or (a positional placed sound's) in stereo.
    Silent { found: ResType, why: Silent },
    /// Found as `found`, and neither WAVE nor MP3.
    NotASound { found: ResType },
    /// Found as `found`, and it can't be read (an archive's past 2 GiB).
    Unreadable { found: ResType, why: String },
}

/// What a file holds, and a WAVE's channels: `None` for neither WAVE nor
/// MP3; an error where it can't be read.
pub type Held = Result<Option<(Form, Option<u16>)>, String>;

/// What a file holds and a WAVE's channels, from as few of its first bytes
/// as that needs ([`mg_audio::form_by_start`]: 4 KiB for a `BMU V1.0`
/// header or a WAVE, a mebibyte for a bare MP3), not its whole: an
/// archive's, a folder's and the game's files are read only that far.
pub fn form_of(rm: &ResMan, key: &ResKey) -> Held {
    mg_audio::form_by_start(|n| rm.get_start(key, n)).map_err(|e| e.to_string())
}

/// What files hold, each read once a run however many areas and sounds
/// name it, by every worker of Verify.
#[derive(Debug, Default)]
pub struct Judged(Mutex<HashMap<ResKey, Held>>);

impl Judged {
    /// [`form_of`], read the first time it is asked for. (Read with the
    /// lock let go: two workers may read one file at once, and the first
    /// answer kept is the one both use.)
    pub fn form(&self, rm: &ResMan, key: &ResKey) -> Held {
        let known = self.0.lock().ok().and_then(|held| held.get(key).cloned());
        let told = known.unwrap_or_else(|| form_of(rm, key));
        match self.0.lock() {
            Ok(mut held) => held.entry(*key).or_insert(told).clone(),
            Err(_) => told,
        }
    }
}

/// Whether the game plays nothing of sound `name` in `role` (a placed
/// sound's `positional` or area-wide), and why: it is looked for in
/// Moonglow's order for the role ([`Role::lookup`]: the types the game
/// looks for first), and what the first found holds is asked of `read`
/// only where the rules' answer depends on it (a track found only as
/// `.wav` is silent whatever it holds).
pub fn unheard_by(
    rm: &ResMan,
    role: Role,
    name: ResRef,
    positional: bool,
    read: &dyn Fn(&ResKey) -> Held,
) -> Option<Unheard> {
    let Some(key) =
        role.lookup().map(|t| ResKey::new(name, t)).into_iter().find(|k| rm.contains(k))
    else {
        return Some(Unheard::Missing);
    };
    let found = key.restype;
    let each = Form::ALL.map(|f| mg_audio::silent_in_game(role, found, f));
    if let Some(why) = each[0].filter(|_| each.iter().all(|w| *w == each[0])) {
        return Some(Unheard::Silent { found, why });
    }
    match read(&key) {
        Err(why) => Some(Unheard::Unreadable { found, why }),
        Ok(None) => Some(Unheard::NotASound { found }),
        Ok(Some((form, channels))) => mg_audio::silent_in_game(role, found, form)
            .or_else(|| mg_audio::silent_positional(role, form, channels, positional))
            .map(|why| Unheard::Silent { found, why }),
    }
}

/// [`unheard_by`], each file read as [`form_of`] reads it.
pub fn unheard(rm: &ResMan, role: Role, name: ResRef, positional: bool) -> Option<Unheard> {
    unheard_by(rm, role, name, positional, &|k| form_of(rm, k))
}

/// What a role's sound is called in a finding: "Music", "Battle music",
/// "Ambient sound", "Sound".
fn noun(role: Role) -> &'static str {
    match role {
        Role::Music => "Music",
        Role::BattleMusic => "Battle music",
        Role::Ambient => "Ambient sound",
        Role::Other | Role::Placed | Role::Viewed => "Sound",
    }
}

/// What Verify says of a row whose `Resource`, `cell`, is longer than a
/// resource's name (16 characters): the game finds no file of it (read in
/// the client for music; an ambient sound's by the same limit on names).
pub fn long_name_message(role: Role, cell: &str) -> String {
    format!(
        "{} {cell} is a name longer than 16 characters, which the game does not find",
        noun(role)
    )
}

/// A type's extension, as a file name has it.
fn ext(t: ResType) -> &'static str {
    t.extension().unwrap_or("?")
}

impl Unheard {
    /// What Verify says of it, for sound `name` in `role`: the file, what
    /// it is, and what the game needs there.
    pub fn message(&self, role: Role, name: ResRef) -> String {
        let n = noun(role);
        match self {
            Unheard::Missing => {
                let names: Vec<String> =
                    role.looked_for().iter().map(|t| format!("{name}.{}", ext(*t))).collect();
                match names.is_empty() {
                    true => format!("{n} {name} is found nowhere"),
                    false => format!(
                        "{n} {name} is found nowhere (the game looks for {})",
                        names.join(", then ")
                    ),
                }
            }
            Unheard::Silent { why, .. } => match why {
                Silent::MusicWave => format!(
                    "{n} {name}.bmu is a WAVE, which the game does not play as music (it must hold MP3)"
                ),
                Silent::MusicOnlyWav => format!(
                    "{n} {name} exists only as {name}.wav, which the game does not look for (music must be a .bmu holding MP3)"
                ),
                Silent::AmbientMp3 => format!(
                    "{n} {name}.wav holds MP3, which the game does not play under .wav (it plays a WAVE there, or MP3 as {name}.bmu)"
                ),
                Silent::AmbientWave => format!(
                    "{n} {name}.bmu is a WAVE, which the game does not play under .bmu (it plays MP3 there, or a WAVE as {name}.wav)"
                ),
                Silent::OnlyBmu => format!(
                    "{n} {name} exists only as {name}.bmu, which the game does not look for (sounds must be a .wav)"
                ),
                Silent::HeaderlessMp3 => format!(
                    "{n} {name}.wav is MP3 without the BMU V1.0 header, which the game does not play (it plays a WAVE there, or MP3 behind the header)"
                ),
                Silent::StereoPositional => format!(
                    "{n} {name}.wav is stereo, which the game does not play from a positional sound (make it mono, or the sound area-wide)"
                ),
                Silent::ViewedWave => {
                    format!("{name}.bmu is a WAVE, which the game plays in no role")
                }
            },
            Unheard::NotASound { found } => format!(
                "{n} {name}.{} is not a sound the game plays (neither WAVE nor MP3)",
                ext(*found)
            ),
            Unheard::Unreadable { found, why } => {
                format!("{n} {name}.{} can't be read ({why})", ext(*found))
            }
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use mg_resman::{LayerClass, MemContainer, priority};

    fn rr(s: &str) -> ResRef {
        ResRef::from_str(s).unwrap()
    }

    /// An MP3 of `frames` silent MPEG-1 layer III frames (417 bytes each).
    pub(crate) fn mp3(frames: usize) -> Vec<u8> {
        let mut frame = vec![0xFF, 0xFB, 0x90, 0xC4];
        frame.resize(417, 0);
        frame.repeat(frames)
    }

    /// A mono RIFF WAVE of format `tag` holding `data`.
    pub(crate) fn wave(tag: u16, data: &[u8]) -> Vec<u8> {
        wave_of(tag, 1, data)
    }

    /// A RIFF WAVE of format `tag` and `channels` holding `data`.
    pub(crate) fn wave_of(tag: u16, channels: u16, data: &[u8]) -> Vec<u8> {
        let mut fmt = tag.to_le_bytes().to_vec();
        fmt.extend(channels.to_le_bytes());
        fmt.extend(22050u32.to_le_bytes());
        fmt.extend(44100u32.to_le_bytes());
        fmt.extend([2, 0, 16, 0]);
        let mut out = b"RIFF".to_vec();
        out.extend((4 + 8 + fmt.len() as u32 + 8 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend((fmt.len() as u32).to_le_bytes());
        out.extend(fmt);
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
    }

    fn rm_of(files: &[(&str, ResType, Vec<u8>)]) -> ResMan {
        let mut mem = MemContainer::new();
        for (name, t, data) in files {
            mem.insert(ResKey::new(rr(name), *t), data.clone());
        }
        let mut rm = ResMan::new();
        rm.add(priority::MODULE, "module", LayerClass::Erf, mem);
        rm
    }

    #[test]
    fn each_role_hears_what_the_game_plays() {
        use ResType as T;
        let bmu_mp3 = [mg_audio::BMU_HEADER.as_slice(), &mp3(3)].concat();
        let pcm = wave(1, &[0; 64]);
        let in_wave = wave(0x55, &mp3(3));
        let noise = vec![0x42u8; 5000];
        let stereo = wave_of(1, 2, &[0; 64]);
        let rm = rm_of(&[
            ("wav_stereo", T::WAV, stereo),
            ("wav_bare_mp3", T::WAV, mp3(3)),
            ("bmu_mp3", T::BMU, bmu_mp3.clone()),
            ("bmu_pcm", T::BMU, pcm.clone()),
            ("bmu_in_wave", T::BMU, in_wave.clone()),
            ("bmu_noise", T::BMU, noise.clone()),
            ("wav_pcm", T::WAV, pcm.clone()),
            ("wav_mp3", T::WAV, bmu_mp3.clone()),
            ("wav_in_wave", T::WAV, in_wave),
            ("wav_noise", T::WAV, noise),
            // Both: music plays the .bmu, an ambient sound the .wav.
            ("both", T::BMU, bmu_mp3),
            ("both", T::WAV, pcm),
        ]);
        let silent = |found, why| Some(Unheard::Silent { found, why });
        let cases = [
            (Role::Music, "bmu_mp3", None),
            (Role::Music, "bmu_in_wave", None),
            (Role::Music, "bmu_pcm", silent(T::BMU, Silent::MusicWave)),
            (Role::Music, "bmu_noise", Some(Unheard::NotASound { found: T::BMU })),
            (Role::Music, "wav_pcm", silent(T::WAV, Silent::MusicOnlyWav)),
            (Role::Music, "wav_noise", silent(T::WAV, Silent::MusicOnlyWav)),
            (Role::Music, "both", None),
            (Role::Music, "gone", Some(Unheard::Missing)),
            (Role::BattleMusic, "bmu_pcm", silent(T::BMU, Silent::MusicWave)),
            (Role::BattleMusic, "bmu_mp3", None),
            (Role::Ambient, "wav_pcm", None),
            (Role::Ambient, "wav_mp3", silent(T::WAV, Silent::AmbientMp3)),
            (Role::Ambient, "wav_in_wave", silent(T::WAV, Silent::AmbientMp3)),
            (Role::Ambient, "wav_noise", Some(Unheard::NotASound { found: T::WAV })),
            (Role::Ambient, "bmu_mp3", None),
            (Role::Ambient, "bmu_pcm", silent(T::BMU, Silent::AmbientWave)),
            (Role::Ambient, "both", None),
            (Role::Ambient, "gone", Some(Unheard::Missing)),
            (Role::Placed, "bmu_mp3", silent(T::BMU, Silent::OnlyBmu)),
            (Role::Other, "wav_pcm", None),
            // PlaySound and placed sounds: MP3 under .wav needs the header.
            (Role::Placed, "wav_mp3", None),
            (Role::Placed, "wav_bare_mp3", silent(T::WAV, Silent::HeaderlessMp3)),
            (Role::Placed, "wav_in_wave", silent(T::WAV, Silent::HeaderlessMp3)),
            (Role::Other, "wav_in_wave", silent(T::WAV, Silent::HeaderlessMp3)),
            (Role::Other, "wav_mp3", None),
            (Role::Placed, "wav_noise", Some(Unheard::NotASound { found: T::WAV })),
            // Stereo is said where the sound is positional (below).
            (Role::Placed, "wav_stereo", None),
            (Role::Ambient, "wav_stereo", None),
        ];
        for (role, name, want) in cases {
            assert_eq!(unheard(&rm, role, rr(name), false), want, "{role:?} {name}");
        }
        // A positional placed sound: a stereo WAVE is silent; mono and MP3
        // are not; nor are the other roles.
        let positional = |role, name| unheard(&rm, role, rr(name), true);
        assert_eq!(
            positional(Role::Placed, "wav_stereo"),
            silent(T::WAV, Silent::StereoPositional)
        );
        assert_eq!(positional(Role::Placed, "wav_pcm"), None);
        assert_eq!(positional(Role::Placed, "wav_mp3"), None);
        assert_eq!(positional(Role::Other, "wav_stereo"), None);
        assert_eq!(positional(Role::Ambient, "wav_stereo"), None);
    }

    /// Where what a file holds can't change the answer, it isn't read.
    #[test]
    fn a_file_is_read_only_where_it_matters() {
        let rm = rm_of(&[("only_wav", ResType::WAV, wave(1, &[0; 8]))]);
        let asked = std::cell::Cell::new(0);
        let read = |k: &ResKey| {
            asked.set(asked.get() + 1);
            form_of(&rm, k)
        };
        let music = unheard_by(&rm, Role::Music, rr("only_wav"), false, &read);
        assert!(matches!(music, Some(Unheard::Silent { why: Silent::MusicOnlyWav, .. })));
        assert_eq!(asked.get(), 0);
        assert_eq!(unheard_by(&rm, Role::Ambient, rr("only_wav"), false, &read), None);
        assert_eq!(asked.get(), 1);
        // Judged once a run, however often asked.
        let judged = Judged::default();
        let key = ResKey::new(rr("only_wav"), ResType::WAV);
        for _ in 0..3 {
            assert_eq!(judged.form(&rm, &key), Ok(Some((Form::Wave, Some(1)))));
        }
        assert_eq!(judged.0.lock().unwrap().len(), 1);
    }

    /// Cut anywhere, a sound is an answer, not a panic.
    #[test]
    fn cut_and_garbled_sounds_are_answers() {
        let forms = [
            wave(1, &[0; 300]),
            wave(0x55, &mp3(4)),
            [mg_audio::BMU_HEADER.as_slice(), &mp3(4)].concat(),
            mp3(4),
        ];
        for form in forms {
            for cut in 0..form.len() {
                let rm = rm_of(&[("cut", ResType::BMU, form[..cut].to_vec())]);
                for role in Role::ALL {
                    let _ = unheard(&rm, role, rr("cut"), true);
                }
                let rm = rm_of(&[("cut", ResType::WAV, form[..cut].to_vec())]);
                for role in Role::ALL {
                    let _ = unheard(&rm, role, rr("cut"), true);
                }
            }
        }
        let rm = rm_of(&[("empty", ResType::WAV, Vec::new())]);
        let told = unheard(&rm, Role::Ambient, rr("empty"), false);
        assert_eq!(told, Some(Unheard::NotASound { found: ResType::WAV }));
    }

    #[test]
    fn the_messages_say_the_file_the_role_and_what_the_game_needs() {
        let name = rr("mg_track");
        let said = |u: Unheard, role| u.message(role, name);
        let silent = |found, why| Unheard::Silent { found, why };
        assert_eq!(
            said(silent(ResType::BMU, Silent::MusicWave), Role::Music),
            "Music mg_track.bmu is a WAVE, which the game does not play as music (it must hold MP3)"
        );
        assert_eq!(
            said(Unheard::Missing, Role::Ambient),
            "Ambient sound mg_track is found nowhere (the game looks for mg_track.wav, then \
             mg_track.bmu)"
        );
        assert_eq!(
            said(silent(ResType::WAV, Silent::StereoPositional), Role::Placed),
            "Sound mg_track.wav is stereo, which the game does not play from a positional sound \
             (make it mono, or the sound area-wide)"
        );
        assert_eq!(
            said(silent(ResType::WAV, Silent::HeaderlessMp3), Role::Placed),
            "Sound mg_track.wav is MP3 without the BMU V1.0 header, which the game does not play \
             (it plays a WAVE there, or MP3 behind the header)"
        );
        assert_eq!(
            long_name_message(Role::Music, "mus_wc_ruralguitar"),
            "Music mus_wc_ruralguitar is a name longer than 16 characters, which the game does not \
             find"
        );
        assert_eq!(
            said(Unheard::Missing, Role::BattleMusic),
            "Battle music mg_track is found nowhere (the game looks for mg_track.bmu)"
        );
        let all = [
            silent(ResType::BMU, Silent::MusicWave),
            silent(ResType::WAV, Silent::MusicOnlyWav),
            silent(ResType::WAV, Silent::AmbientMp3),
            silent(ResType::BMU, Silent::AmbientWave),
            silent(ResType::BMU, Silent::OnlyBmu),
            silent(ResType::WAV, Silent::HeaderlessMp3),
            silent(ResType::WAV, Silent::StereoPositional),
            silent(ResType::BMU, Silent::ViewedWave),
            Unheard::Missing,
            Unheard::NotASound { found: ResType::WAV },
            Unheard::Unreadable { found: ResType::BMU, why: "past 2 GiB".into() },
        ];
        for u in all {
            for role in Role::ALL {
                let m = said(u.clone(), role);
                assert!(m.contains("mg_track") && !m.contains(';') && !m.ends_with('.'), "{m}");
                let first = m.chars().next().unwrap();
                assert!(first.is_uppercase() || m.starts_with("mg_track"), "{m}");
            }
        }
    }
}
