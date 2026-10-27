//! Helpers shared by the comparisons with Aurora.

use mg_gff::{Struct, Value};

/// How a GFF is written as lines for [`lines`].
#[derive(Clone, Copy)]
pub(crate) struct Shown<'a> {
    /// Each struct's fields by label, not in their order.
    pub sorted: bool,
    /// Floats to this many decimals, with Aurora's noise (1.8e-40 for 0)
    /// as 0; `None` writes them as they are.
    pub decimals: Option<usize>,
    /// Top-level fields whose value is not compared (Aurora leaves them
    /// uninitialized), only that they are there.
    pub uninitialized: &'a [&'a str],
}

/// Every field in its order, every value as it is.
pub(crate) const PLAIN: Shown<'static> =
    Shown { sorted: false, decimals: None, uninitialized: &[] };

/// A GFF struct as lines (labels, types and values), for a readable
/// difference between Moonglow's and Aurora's.
pub(crate) fn lines(s: &Struct, shown: Shown<'_>) -> Vec<String> {
    let mut out = Vec::new();
    add(s, "", shown, &mut out);
    out
}

fn add(s: &Struct, indent: &str, shown: Shown<'_>, out: &mut Vec<String>) {
    let mut fields: Vec<_> = s.fields.iter().collect();
    if shown.sorted {
        fields.sort_by_key(|f| f.label.to_string_lossy());
    }
    for f in fields {
        let label = f.label.to_string_lossy();
        if indent.is_empty() && shown.uninitialized.contains(&label.as_str()) {
            out.push(format!("{label} (uninitialized)"));
            continue;
        }
        match &f.value {
            Value::List(items) => {
                out.push(format!("{indent}{label} list[{}]", items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    add(item, &format!("{indent}    "), shown, out);
                }
            }
            Value::Struct(c) => {
                out.push(format!("{indent}{label} struct {}", c.id));
                add(c, &format!("{indent}  "), shown, out);
            }
            Value::Float(v) if let Some(d) = shown.decimals => {
                let v = if v.abs() < 1e-6 { 0.0 } else { *v };
                out.push(format!("{indent}{label} Float({v:.d$})"));
            }
            v => out.push(format!("{indent}{label} {v:?}")),
        }
    }
}
