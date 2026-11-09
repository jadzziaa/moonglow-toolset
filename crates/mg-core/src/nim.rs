//! Text as Nim's standard library writes it, which neverwinter.nim's tools
//! (`nwn_gff`, `nwn_tlk`) and so nasher's projects are written in: a float
//! as `$` and `addFloat` give it, a string as `escapeJson` does. One of
//! each, for every format's JSON: `nwn_gff` and `nwn_tlk` 2.1 write the
//! same for the same value (measured at the edges the tests name).

/// A float as Nim writes it: the shortest digits that read back to it,
/// with the point among them while it falls within -6 to 17 places of the
/// first digit (`0.0000001`, `10000000000000000.0`, with `.0` after a
/// whole number), else scientific with the exponent as it is (`1e-8`,
/// `1.5e+17`). `nan`, `inf` and `-inf` for what is no number (not JSON:
/// a caller writing JSON decides what stands there).
pub fn float(f: f64) -> String {
    if !f.is_finite() {
        return if f.is_nan() {
            "nan".into()
        } else if f > 0.0 {
            "inf".into()
        } else {
            "-inf".into()
        };
    }
    let mut out = String::new();
    if f.is_sign_negative() {
        out.push('-');
    }
    if f == 0.0 {
        out.push_str("0.0");
        return out;
    }
    // (Rust's `{:e}` gives as few digits, `1.5e0`, `9.99e-5`. Where two
    // of that many are as near, as 2.4235000610351562 and ...563 to
    // 2.42350006103515625, a single-precision float read as a double, it
    // takes the higher and Nim the even one, as rounding to that many
    // digits does: so rounded again to as many.)
    let shortest = format!("{:e}", f.abs());
    let places = shortest.split_once('e').map_or(0, |(m, _)| m.len().saturating_sub(2));
    let sci = format!("{:.places$e}", f.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exp: i32 = exp.parse().unwrap_or(0);
    let n = digits.len() as i32;
    // The decimal point's position after the first digit's.
    let point = exp + 1;
    if (-6..=17).contains(&point) {
        if point <= 0 {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-point) as usize));
            out.push_str(&digits);
        } else if point < n {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        } else {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', (point - n) as usize));
            out.push_str(".0");
        }
    } else {
        out.push_str(&digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if exp < 0 { '-' } else { '+' });
        out.push_str(&exp.abs().to_string());
    }
    out
}

/// What `escapeJson` writes for the control characters, by their number:
/// the short escapes it knows (not `\v`: `\u000b`, in lower case, where
/// the others' hexadecimal is in capitals).
const CONTROL: [&str; 32] = [
    "\\u0000", "\\u0001", "\\u0002", "\\u0003", "\\u0004", "\\u0005", "\\u0006", "\\u0007", "\\b",
    "\\t", "\\n", "\\u000b", "\\f", "\\r", "\\u000E", "\\u000F", "\\u0010", "\\u0011", "\\u0012",
    "\\u0013", "\\u0014", "\\u0015", "\\u0016", "\\u0017", "\\u0018", "\\u0019", "\\u001A",
    "\\u001B", "\\u001C", "\\u001D", "\\u001E", "\\u001F",
];

/// How `escapeJson` writes a byte of a string, where not as it is: the
/// control characters, `"` and `\` (no `/`, and nothing past ASCII).
pub fn json_escape(byte: u8) -> Option<&'static str> {
    match byte {
        0..0x20 => Some(CONTROL[usize::from(byte)]),
        b'"' => Some("\\\""),
        b'\\' => Some("\\\\"),
        _ => None,
    }
}

/// Adds `text` to `out` as the JSON string Nim's `escapeJson` writes, its
/// quotes too.
pub fn escape_json(out: &mut String, text: &str) {
    out.push('"');
    let mut from = 0;
    for (i, b) in text.bytes().enumerate() {
        if let Some(escape) = json_escape(b) {
            out.push_str(&text[from..i]);
            out.push_str(escape);
            from = i + 1;
        }
    }
    out.push_str(&text[from..]);
    out.push('"');
}

/// [`escape_json`] of bytes that are text but for stray ones (a byte a
/// codepage has no character for), which go as they are.
pub fn escape_json_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'"');
    for &b in bytes {
        match json_escape(b) {
            Some(escape) => out.extend_from_slice(escape.as_bytes()),
            None => out.push(b),
        }
    }
    out.push(b'"');
}

#[cfg(test)]
mod tests {
    use super::*;

    /// As `nwn_gff` 2.1 wrote these as doubles, and `nwn_tlk` those a
    /// sound's length can show (it rounds to four places, so nothing
    /// below 0.0001): the two agree wherever both show a value.
    #[test]
    fn floats_are_written_as_nim_writes_them() {
        for (v, text) in [
            // The edges of the fixed form, measured with nwn_gff.
            (1e-4, "0.0001"),
            (1e-5, "0.00001"),
            (9.99e-6, "0.00000999"),
            (1e-6, "0.000001"),
            (1.5e-6, "0.0000015"),
            (1e-7, "0.0000001"),
            (1.25e-7, "0.000000125"),
            (-1e-7, "-0.0000001"),
            (1e-8, "1e-8"),
            (1e-10, "1e-10"),
            (1e15, "1000000000000000.0"),
            (1e16, "10000000000000000.0"),
            (1.5e16, "15000000000000000.0"),
            (1e17, "1e+17"),
            (1.5e17, "1.5e+17"),
            (123456789012345678.0, "1.2345678901234568e+17"),
            (1e18, "1e+18"),
            (-1e18, "-1e+18"),
            (1e22, "1e+22"),
            (1e100, "1e+100"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1.0 / 3.0, "0.3333333333333333"),
            // Single-precision values read as doubles, by both tools.
            (f64::from(0.1f32), "0.10000000149011612"),
            (f64::from(1e17f32), "99999998430674940.0"),
            (f64::from(1e18f32), "9.999999843067494e+17"),
            (f64::from(3e38f32), "3.0000000054977558e+38"),
            (f64::from(123456.79f32), "123456.7890625"),
            (f64::from(0.0001f32), "0.00009999999747378752"),
            (f64::from(1e-6f32), "0.0000009999999974752427"),
            (f64::from(1e-7f32), "0.00000010000000116860974"),
            // Halfway between two of as many digits: the even one, as
            // both tools write it (Rust's shortest is ...563).
            (f64::from(2.4235f32), "2.4235000610351562"),
            // The rest.
            (1e20, "1e+20"),
            (9.999999999999998e16, "99999999999999980.0"),
            (2.5e-7, "0.00000025"),
            (1.2345e-9, "1.2345e-9"),
            (-1e-8, "-1e-8"),
            (1.17549435e-38, "1.17549435e-38"),
            (1.0000000150474662e30, "1.0000000150474662e+30"),
            (-0.0, "-0.0"),
            (0.0, "0.0"),
            (3.0, "3.0"),
            (2.0, "2.0"),
            (-123.5, "-123.5"),
            (12345678.0, "12345678.0"),
            (1.5707942247390747, "1.5707942247390747"),
        ] {
            assert_eq!(float(v), text, "{v:e}");
        }
        // What is no number is no panic.
        assert_eq!(
            [float(f64::INFINITY), float(f64::NEG_INFINITY), float(f64::NAN)],
            ["inf", "-inf", "nan"]
        );
    }

    /// As `nwn_gff` and `nwn_tlk` wrote a string of every control
    /// character, a quote, a backslash, a slash and `é`.
    #[test]
    fn strings_are_escaped_as_nim_escapes_them() {
        let text: String = (1u8..0x20).map(char::from).chain("\"\\/\u{7f}\u{e9}".chars()).collect();
        let expected = "\"\\u0001\\u0002\\u0003\\u0004\\u0005\\u0006\\u0007\\b\\t\\n\\u000b\\f\\r\
                        \\u000E\\u000F\\u0010\\u0011\\u0012\\u0013\\u0014\\u0015\\u0016\\u0017\\u0018\
                        \\u0019\\u001A\\u001B\\u001C\\u001D\\u001E\\u001F\\\"\\\\/\u{7f}\u{e9}\"";
        let mut out = String::new();
        escape_json(&mut out, &text);
        assert_eq!(out, expected);
        let mut bytes = Vec::new();
        escape_json_bytes(&mut bytes, text.as_bytes());
        assert_eq!(bytes, expected.as_bytes());
        // A stray byte goes as it is; a NUL is escaped.
        let mut bytes = Vec::new();
        escape_json_bytes(&mut bytes, b"a\x00\xe9\"");
        assert_eq!(bytes, b"\"a\\u0000\xe9\\\"\"");
    }
}
