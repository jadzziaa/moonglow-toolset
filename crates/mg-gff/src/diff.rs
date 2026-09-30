//! Structural differences between two GFF trees, as readable paths.

use crate::Gff;
use crate::value::{Struct, Value};

/// Lists the differences between two files (at most `limit`), as lines like
/// `/Creature List[3]/Tag: "a" != "b"`. Empty means equal.
pub fn diff(a: &Gff, b: &Gff, limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    if a.file_type != b.file_type {
        out.push(format!(
            "file type: {:?} != {:?}",
            String::from_utf8_lossy(&a.file_type),
            String::from_utf8_lossy(&b.file_type)
        ));
    }
    diff_structs(&a.root, &b.root, "", limit, &mut out);
    out
}

/// Appends the differences between two structs under `path`.
pub fn diff_structs(a: &Struct, b: &Struct, path: &str, limit: usize, out: &mut Vec<String>) {
    if out.len() >= limit {
        return;
    }
    if a.id != b.id {
        out.push(format!("{path}: struct id {} != {}", a.id, b.id));
    }
    let labels_a: Vec<_> = a.fields.iter().map(|f| f.label).collect();
    let labels_b: Vec<_> = b.fields.iter().map(|f| f.label).collect();
    if labels_a != labels_b {
        let missing: Vec<_> = labels_a.iter().filter(|l| !labels_b.contains(l)).collect();
        let extra: Vec<_> = labels_b.iter().filter(|l| !labels_a.contains(l)).collect();
        if missing.is_empty() && extra.is_empty() {
            out.push(format!("{path}: same fields in a different order"));
        } else {
            out.push(format!("{path}: fields only in first {missing:?}, only in second {extra:?}"));
        }
    }
    for fa in &a.fields {
        let Some(fb) = b.fields.iter().find(|f| f.label == fa.label) else { continue };
        let p = format!("{path}/{}", fa.label);
        diff_values(&fa.value, &fb.value, &p, limit, out);
        if out.len() >= limit {
            return;
        }
    }
}

fn diff_values(a: &Value, b: &Value, path: &str, limit: usize, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Struct(x), Value::Struct(y)) => diff_structs(x, y, path, limit, out),
        (Value::List(x), Value::List(y)) => {
            if x.len() != y.len() {
                out.push(format!("{path}: list of {} != list of {}", x.len(), y.len()));
            }
            for (i, (sx, sy)) in x.iter().zip(y).enumerate() {
                diff_structs(sx, sy, &format!("{path}[{i}]"), limit, out);
            }
        }
        _ if a != b => out.push(format!("{path}: {} != {}", brief(a), brief(b))),
        _ => {}
    }
}

fn brief(v: &Value) -> String {
    let s = match v {
        Value::String(b) | Value::ResRef(b) => format!("{:?}", String::from_utf8_lossy(b)),
        Value::Void(b) => format!("void[{}]", b.len()),
        other => format!("{other:?}"),
    };
    let t = v.field_type().json_name();
    if s.len() > 120 {
        format!("{t} {}…", &s[..s.floor_char_boundary(120)])
    } else {
        format!("{t} {s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_paths() {
        let mut a = Gff::new(*b"GFF ");
        let mut s = Struct::new(1);
        s.set("X", Value::Int(1));
        a.root.set("L", Value::List(vec![s]));
        let mut b = a.clone();
        b.root.list_mut("L").unwrap()[0].set("X", Value::Int(2));
        b.root.set("New", Value::Byte(0));
        let d = diff(&a, &b, 10);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d[0].contains("only in second"));
        assert_eq!(d[1], "/L[0]/X: int Int(1) != int Int(2)");
        assert!(diff(&a, &a, 10).is_empty());
    }
}
