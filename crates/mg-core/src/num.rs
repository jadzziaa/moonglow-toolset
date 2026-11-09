//! Numbers as the game reads them from its text files (2DA cells, a
//! tileset's values): the longest prefix that is one, what follows
//! ignored.

/// Parses a float like C's `atof`: the longest valid prefix (a sign,
/// digits with or without a point, an exponent), found in one pass. `None`
/// if there is none.
pub fn parse_float(s: &str) -> Option<f32> {
    let s = s.trim();
    let b = s.as_bytes();
    let mut end = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        end = 1;
    }
    let digits_start = end;
    while end < b.len() && b[end].is_ascii_digit() {
        end += 1;
    }
    if end < b.len() && b[end] == b'.' {
        end += 1;
        while end < b.len() && b[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end == digits_start || (end == digits_start + 1 && b[digits_start] == b'.') {
        return None;
    }
    // Optional exponent.
    if end < b.len() && matches!(b[end], b'e' | b'E') {
        let mut e = end + 1;
        if matches!(b.get(e), Some(b'+' | b'-')) {
            e += 1;
        }
        if b.get(e).is_some_and(u8::is_ascii_digit) {
            while b.get(e).is_some_and(u8::is_ascii_digit) {
                e += 1;
            }
            end = e;
        }
    }
    s[..end].parse().ok()
}

/// Parses a decimal integer: a sign, then digits, what follows ignored
/// (`250;` is 250). `None` if there are no digits or they are out of an
/// `i32`'s range.
pub fn parse_decimal(s: &str) -> Option<i32> {
    let s = s.trim();
    let end = s
        .char_indices()
        .find(|&(i, c)| !(c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+'))))
        .map_or(s.len(), |(i, _)| i);
    s[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_longest_prefix_is_the_number() {
        assert_eq!(parse_float(" 1.5f "), Some(1.5));
        assert_eq!(parse_float("-2"), Some(-2.0));
        assert_eq!(parse_float("+.5x"), Some(0.5));
        assert_eq!(parse_float("5."), Some(5.0));
        assert_eq!(parse_float("1e3"), Some(1000.0));
        assert_eq!(parse_float("1e"), Some(1.0));
        assert_eq!(parse_float("1e+"), Some(1.0));
        assert_eq!(parse_float("2.5E-1;"), Some(0.25));
        for none in ["", ".", "-", "x1", "****", "e5", "inf", "nan"] {
            assert_eq!(parse_float(none), None, "{none}");
        }
        assert_eq!(parse_decimal(" 250; "), Some(250));
        assert_eq!(parse_decimal("-4x"), Some(-4));
        assert_eq!(parse_decimal("+7"), Some(7));
        assert_eq!(parse_decimal("0x10"), Some(0));
        for none in ["", "-", "x", "99999999999"] {
            assert_eq!(parse_decimal(none), None, "{none}");
        }
    }

    /// A value of a megabyte is read once, not once for each of its
    /// prefixes.
    #[test]
    fn a_long_value_is_read_in_one_pass() {
        let long = format!("{}{}", "7".repeat(500_000), "x".repeat(500_000));
        let started = std::time::Instant::now();
        assert_eq!(parse_float(&long), Some(f32::INFINITY));
        assert_eq!(parse_float(&long[500_000..]), None);
        assert_eq!(parse_decimal(&long), None);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
    }
}
