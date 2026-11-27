//! A player's CD key, as far as a server needs it: its public part, the
//! name of the player's folder in a server vault
//! (`servervault/<public key>/`), where a server started with
//! `-servervault 1` keeps their characters.
//!
//! Ported from nwn-message's `CDKey` (niv, GPL-3.0-or-later,
//! `nwn_message/cdkey.py`, 0.1.1): the key unscrambled into its public,
//! private and check parts, and the check made. The private part never
//! leaves this module: nothing here returns, prints or writes it.

/// The 26 letters and digits a key is written in, by value.
const BASE26: &[u8; 26] = b"UANPXTFWCQ9DVLE3GYJHK74R6M";
const PUBLIC: usize = 8;
const PRIVATE: usize = 20;
const CHECK: usize = 7;
const LENGTH: usize = 41;

/// CRC-32's table (the reflected polynomial `0xEDB88320`).
fn table() -> [u32; 256] {
    let mut t = [0; 256];
    for (i, v) in t.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
        *v = c;
    }
    t
}

/// The check part a key's public and private parts make.
fn check(public: &[u8], private: &[u8]) -> Vec<u8> {
    let t = table();
    let mut crc: u32 = 0;
    for b in public.iter().chain(private) {
        crc = ((crc >> 8) & 0x00FF_FFFF) ^ t[((crc ^ u32::from(*b)) & 0xFF) as usize];
    }
    let mut out = vec![0; CHECK];
    for slot in out.iter_mut().rev() {
        *slot = BASE26[(crc % 26) as usize];
        crc /= 26;
    }
    out
}

/// Why a text is not a CD key, in words that never quote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// Not 41 characters with dashes after every five.
    Form,
    /// A character not of the key's alphabet.
    Letters,
    /// Its check part does not match the rest.
    Check,
}

impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            KeyError::Form => "not a CD key (41 characters, five between dashes)",
            KeyError::Letters => "not a CD key (a character keys are not written in)",
            KeyError::Check => "not a CD key (its check does not match)",
        })
    }
}

/// The public part of a CD key (`XXXXX-XXXXX-…`, as `cdkey.ini` keeps
/// it): eight characters, the player's vault folder's name.
pub fn public_key(key: &str) -> Result<String, KeyError> {
    let key = key.trim().as_bytes();
    if key.len() != LENGTH || [5, 11, 17, 23, 29, 35].iter().any(|&i| key[i] != b'-') {
        return Err(KeyError::Form);
    }
    let clean: Vec<u8> = key.iter().copied().filter(|c| *c != b'-').collect();
    if clean.len() != PUBLIC + PRIVATE + CHECK {
        return Err(KeyError::Form);
    }
    if !clean.iter().all(|c| BASE26.contains(c)) {
        return Err(KeyError::Letters);
    }
    // The characters dealt in turn to the private part, the check (once
    // the private part is past its start) and the public part, a part
    // that is full passing its turn on.
    let (mut private, mut crc, mut public) = (Vec::new(), Vec::new(), Vec::new());
    let crc_from = PRIVATE - CHECK - 1;
    let mut part = 0;
    for &c in &clean {
        loop {
            if part == 0 {
                part = 1;
                if private.len() < PRIVATE {
                    private.push(c);
                    break;
                }
            }
            if part == 1 {
                part = 2;
                if private.len() > crc_from && crc.len() < CHECK {
                    crc.push(c);
                    break;
                }
            }
            if part == 2 {
                part = 0;
                if public.len() < PUBLIC {
                    public.push(c);
                    break;
                }
                if private.len() < PRIVATE || crc.len() < CHECK {
                    continue;
                }
                break;
            }
        }
    }
    if check(&public, &private) != crc {
        return Err(KeyError::Check);
    }
    Ok(String::from_utf8_lossy(&public).into_owned())
}

/// The key a `cdkey.ini` holds (`[NWN1]`'s `YourKey=`, else the first
/// `YourKey=`), only read.
pub fn key_in_ini(text: &str) -> Option<&str> {
    let mut section = "";
    let mut first = None;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            section = line;
            continue;
        }
        let Some((name, value)) = line.split_once('=') else { continue };
        if name.trim().eq_ignore_ascii_case("YourKey") {
            let value = value.trim();
            if section.eq_ignore_ascii_case("[NWN1]") {
                return Some(value);
            }
            first.get_or_insert(value);
        }
    }
    first
}

/// The public key of the player whose user folder has this `cdkey.ini`.
pub fn public_key_of_ini(path: &std::path::Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let key = key_in_ini(&text).ok_or_else(|| format!("{}: no YourKey=", path.display()))?;
    public_key(key).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Keys nwn-message's `CDKey.generate` made (made up, registered
    /// nowhere), with their public parts.
    const MADE: [(&str, &str); 3] = [
        ("HUHAQ-NMPXX-HTDFJ-WFELW-YL7NG-6W4UV-DTV7P", "UANPXTFW"),
        ("MQKRU-CKJK9-CKWLL-ENCWC-9VN6Y-AX9UY-NUG4W", "QRCJ9KLE"),
        ("LCVFW-QF77J-46VF3-FLKDE-7L67E-449EG-64KG6", "CFQ7J6FF"),
    ];

    #[test]
    fn a_key_s_public_part_is_read_and_checked() {
        for (key, public) in MADE {
            assert_eq!(public_key(key).as_deref(), Ok(public), "{key}");
        }
        let (key, _) = MADE[0];
        // One character changed: its check fails.
        let mut wrong = key.as_bytes().to_vec();
        wrong[0] = if wrong[0] == b'U' { b'A' } else { b'U' };
        assert_eq!(public_key(std::str::from_utf8(&wrong).unwrap()), Err(KeyError::Check));
        assert_eq!(public_key(&key.replace('-', "")), Err(KeyError::Form));
        assert_eq!(public_key(&key.replacen('H', "B", 1)), Err(KeyError::Letters));
        assert_eq!(public_key("short"), Err(KeyError::Form));
        // The message names no part of the key.
        assert!(!KeyError::Check.to_string().contains(&key[..5]));
    }

    #[test]
    fn the_key_is_read_from_cdkey_ini() {
        let text = format!("[XP1]\nYourKey=other\n[NWN1]\nYourKey = {}\n", MADE[1].0);
        assert_eq!(key_in_ini(&text), Some(MADE[1].0));
        assert_eq!(key_in_ini("[X]\nYourKey=a\n"), Some("a"));
        assert_eq!(key_in_ini("[NWN1]\n"), None);
    }

    /// Against nwn-message itself, where it is installed (the Python of
    /// `NWN_MESSAGE_PYTHON`, else `~/.local/opt/nwnx/msgenv`): keys it
    /// makes now give the public part it says.
    #[test]
    fn the_public_part_is_as_nwn_message_reads_it() {
        let python =
            std::env::var_os("NWN_MESSAGE_PYTHON").map(std::path::PathBuf::from).or_else(|| {
                let home = std::env::var_os("HOME")?;
                Some(std::path::PathBuf::from(home).join(".local/opt/nwnx/msgenv/bin/python"))
            });
        let Some(python) = python.filter(|p| p.is_file()) else {
            eprintln!("skipped: nwn-message's Python is not installed");
            return;
        };
        let code = "from nwn_message.cdkey import CDKey\nfor _ in range(50):\n    k = \
                    CDKey.generate()\n    print(k.full_key(), k.public_key())\n";
        let out = std::process::Command::new(python).args(["-I", "-c", code]).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let text = String::from_utf8(out.stdout).unwrap();
        assert_eq!(text.lines().count(), 50);
        for line in text.lines() {
            let (key, public) = line.split_once(' ').unwrap();
            assert_eq!(public_key(key).as_deref(), Ok(public));
        }
    }
}
