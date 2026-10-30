//! `mg haks --usage`'s JSON: what the module uses of its haks, hak by
//! hak (the walk is `mg_preview::usage`, which the window's Check for
//! Conflicts reads too).

use std::collections::BTreeMap;

pub(crate) use mg_preview::usage::{
    ByNumber, HakUsage, NOT_FOLLOWED, by_number_line, hak_usage, line, total_line,
};
use mg_resman::ResKey;

/// The JSON of `mg haks --usage`, hak by hak.
pub(crate) fn json(usage: &[HakUsage]) -> serde_json::Value {
    use serde_json::json;
    let names =
        |v: &mut dyn Iterator<Item = &ResKey>| v.map(ToString::to_string).collect::<Vec<_>>();
    serde_json::Value::Array(
        usage
            .iter()
            .map(|u| {
                let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
                for (k, _, _) in &u.used {
                    *by_kind.entry(k.restype.extension().unwrap_or("?")).or_default() += 1;
                }
                json!({
                    "name": u.name,
                    "path": u.path.as_ref().map(|p| p.display().to_string()),
                    "resources": u.total(),
                    "bytes": u.bytes(),
                    "used": u.used.len(),
                    "used_bytes": u.used_bytes(),
                    "unused": u.unused.len(),
                    "shadowed": u.shadowed.len(),
                    "not_judged": u.not_judged.len(),
                    "used_by_type": by_kind,
                    "used_resources": u.used.iter().map(|(k, _, from)| json!({
                        "resource": k.to_string(), "from": from.to_string(),
                    })).collect::<Vec<_>>(),
                    "unused_resources": names(&mut u.unused.iter().map(|u| &u.0)),
                    "shadowed_resources": u.shadowed.iter().map(|(k, _, by)| json!({
                        "resource": k.to_string(), "by": by,
                    })).collect::<Vec<_>>(),
                    "not_judged_resources": names(&mut u.not_judged.iter().map(|u| &u.0)),
                })
            })
            .collect(),
    )
}

/// What scripts name by a number, in the JSON: the calls followed, and
/// each script and function whose number is worked out as it runs.
pub(crate) fn by_number_json(b: &ByNumber) -> serde_json::Value {
    serde_json::json!({
        "followed": b.followed,
        "not_followed": b.not_followed.iter().map(|(k, f)| serde_json::json!({
            "script": k.to_string(), "function": f,
        })).collect::<Vec<_>>(),
    })
}
