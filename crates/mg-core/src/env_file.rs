//! Environment files: the `NAME=value` lines a server is given (NWNX's
//! settings), read by `mg serve --env`, the project's `[nwnx] env` and the
//! engine tests alike.

/// The settings of an environment file: `NAME=value` lines, `#` comments,
/// blank lines, an optional `export ` before a name, a value in single or
/// double quotes taken without them. Nothing is expanded.
pub fn parse(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").map_or(line, str::trim_start);
        let Some((name, value)) = line.split_once('=') else {
            return Err(format!("line {}: no NAME=value: {line}", i + 1));
        };
        let (name, value) = (name.trim(), value.trim());
        let plain = |c: char| c.is_ascii_alphanumeric() || c == '_';
        if name.is_empty() || !name.chars().all(plain) {
            return Err(format!("line {}: not a name: {name}", i + 1));
        }
        let quoted = |q: char| value.len() >= 2 && value.starts_with(q) && value.ends_with(q);
        let value = match quoted('"') || quoted('\'') {
            true => &value[1..value.len() - 1],
            false => value,
        };
        out.push((name.to_string(), value.to_string()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_environment_file_is_names_and_values() {
        let text = "# NWNX\nNWNX_CORE_LOG_LEVEL=7\nexport A_B = \"two words\"\n\nC='x=y'\nD=\n";
        let read = parse(text).unwrap();
        let pairs: Vec<(&str, &str)> = read.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        assert_eq!(
            pairs,
            [("NWNX_CORE_LOG_LEVEL", "7"), ("A_B", "two words"), ("C", "x=y"), ("D", "")]
        );
        assert!(parse("just words").unwrap_err().contains("line 1"));
        assert!(parse("ok=1\nbad name=2").unwrap_err().contains("line 2"));
    }
}
