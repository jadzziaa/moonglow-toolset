//! The ASCII model keywords the game knows, by where they may appear
//! (nwnmdlcomp's attribute tables, the keywords in the game's own ASCII
//! models and the EE client's parser strings, as surveyed in the toolset's
//! `notes_models.md` B.22 and the viewer's research). Keywords are
//! case-insensitive; these are lower case.

/// Outside nodes.
pub const TOP_LEVEL: &[&str] = &[
    "filedependancy",
    "filedependency",
    "newmodel",
    "setsupermodel",
    "classification",
    "ignorefog",
    "setanimationscale",
    "beginmodelgeom",
    "endmodelgeom",
    "beginwalkmeshgeom",
    "endwalkmeshgeom",
    "newanim",
    "doneanim",
    "donemodel",
    "length",
    "transtime",
    "animroot",
    "event",
    "node",
    "endnode",
];

pub const NODE_TYPES: &[&str] = &[
    "dummy",
    "trimesh",
    "skin",
    "danglymesh",
    "animmesh",
    "aabb",
    "emitter",
    "light",
    "reference",
    "camera",
    "patch",
    // Walkmesh files' pseudo types, read as dummies.
    "pwk",
    "dwk",
    "wok",
];

/// Keywords followed by a count and that many rows.
pub const LISTS: &[&str] = &[
    "verts",
    "tverts",
    "tverts0",
    "tverts1",
    "tverts2",
    "tverts3",
    "faces",
    "normals",
    "tangents",
    "colors",
    "weights",
    "constraints",
    "animverts",
    "animtverts",
    "flarepositions",
    "flaresizes",
    "flarecolorshifts",
    "texturenames",
    "multimaterial",
    "qbone_ref_inv",
    "tbone_ref_inv",
    "boneconstantindices",
    "texindices1",
    "texindices2",
    "texindices3",
    "vertexindices",
    "mirrorlist",
];

/// Lists whose rows start with a name.
pub const NAMED_LISTS: &[&str] = &["weights", "texturenames", "multimaterial"];

/// Every node.
const COMMON: &[&str] =
    &["parent", "position", "orientation", "scale", "wirecolor", "inheritcolor"];

const MESH: &[&str] = &[
    "ambient",
    "diffuse",
    "specular",
    "shininess",
    "shadow",
    "beaming",
    "render",
    "transparencyhint",
    "tilefade",
    "rotatetexture",
    "lightmapped",
    "bitmap",
    "texture0",
    "texture1",
    "texture2",
    "texture3",
    "texture4",
    "texture5",
    "texture6",
    "texture7",
    "texture8",
    "texture9",
    "texture10",
    "texture11",
    "texture12",
    "texture13",
    "texture14",
    "materialname",
    "renderhint",
    "selfillumcolor",
    "setfillumcolor",
    "alpha",
    "center",
    "gizmo",
    "showdispl",
    "displtype",
    "bmin",
    "bmax",
    "vertexindicescount",
    "leftoverfaces",
    "ignorefog",
    // GUI models.
    "clipu",
    "clipv",
    "clipw",
    "cliph",
];

const SKIN: &[&str] = &[];
const DANGLY: &[&str] = &["displacement", "tightness", "period"];
const ANIMMESH: &[&str] = &["sampleperiod"];
const AABB: &[&str] = &["aabb", "walkmesh"];
const LIGHT: &[&str] = &[
    "radius",
    "multiplier",
    "color",
    "ambientonly",
    "ambient_only",
    "isdynamic",
    "ndynamictype",
    "n_dynamic_type",
    "affectdynamic",
    "affect_dynamic",
    "shadow",
    "lightpriority",
    "fadinglight",
    "fading_light",
    "generateflare",
    "lensflares",
    "flareradius",
    "shadowradius",
    "verticaldisplacement",
    "negativelight",
];
const EMITTER: &[&str] = &[
    "update",
    "update_sel",
    "render",
    "render_sel",
    "blend",
    "blend_sel",
    "spawntype",
    "spawntype_sel",
    "renderorder",
    "birthrate",
    "lifeexp",
    "mass",
    "velocity",
    "randvel",
    "particlerot",
    "spread",
    "splat",
    "affectedbywind",
    "colorstart",
    "colormid",
    "colorend",
    "alphastart",
    "alphamid",
    "alphaend",
    "sizestart",
    "sizemid",
    "sizeend",
    "sizestart_y",
    "sizemid_y",
    "sizeend_y",
    "percentstart",
    "percentmid",
    "percentend",
    "bounce",
    "bounce_co",
    "blurlength",
    "deadspace",
    "texture",
    "chunkname",
    "twosidedtex",
    "m_istinted",
    "m_istnited",
    "xgrid",
    "ygrid",
    "fps",
    "framestart",
    "frameend",
    "random",
    "p2p",
    "p2p_sel",
    "p2p_type",
    "p2p_bezier2",
    "p2p_bezier3",
    "combinetime",
    "grav",
    "drag",
    "threshold",
    "blastradius",
    "blastlength",
    "lightningdelay",
    "lightningradius",
    "lightningscale",
    "lightningsubdiv",
    "inherit",
    "inheritvel",
    "inherit_local",
    "inherit_part",
    "loop",
    "opacity",
    "xsize",
    "ysize",
    "detonate",
];
const REFERENCE: &[&str] = &["refmodel", "reattachable"];

/// Extra keywords in animation nodes.
pub const ANIM_NODE: &[&str] = &["endlist", "sampleperiod"];

/// Whether a (lower-case) keyword means something in a node of this type.
/// Lists and key lists (`…key`) are checked separately.
pub fn known_in_node(ty: &str, key: &str) -> bool {
    let extra: &[&[&str]] = match ty {
        "trimesh" => &[MESH],
        "skin" => &[MESH, SKIN],
        "danglymesh" => &[MESH, DANGLY],
        "animmesh" => &[MESH, ANIMMESH],
        "aabb" => &[MESH, AABB],
        "light" => &[LIGHT],
        "emitter" => &[EMITTER],
        "reference" => &[REFERENCE],
        _ => &[],
    };
    COMMON.contains(&key) || LISTS.contains(&key) || extra.iter().any(|set| set.contains(&key))
}

/// Whether any node type uses a (lower-case) keyword.
pub fn known_anywhere(key: &str) -> bool {
    ["trimesh", "skin", "danglymesh", "animmesh", "aabb", "light", "emitter", "reference"]
        .iter()
        .any(|t| known_in_node(t, key))
}

/// What a word is, for highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// `node`, `endnode`, `newanim`, … (structure).
    Structure,
    /// A property or list keyword.
    Keyword,
    /// A key list (`positionkey`).
    Keyed,
    /// A node type after `node`.
    NodeType,
    Number,
    /// Anything else (names).
    Name,
}

/// Classifies a word (any case).
pub fn class(word: &str) -> Class {
    let lower = word.to_ascii_lowercase();
    let w = lower.as_str();
    if w.parse::<f32>().is_ok() {
        return Class::Number;
    }
    const STRUCTURE: &[&str] = &[
        "node",
        "endnode",
        "newmodel",
        "donemodel",
        "newanim",
        "doneanim",
        "beginmodelgeom",
        "endmodelgeom",
        "beginwalkmeshgeom",
        "endwalkmeshgeom",
        "endlist",
    ];
    if STRUCTURE.contains(&w) {
        return Class::Structure;
    }
    if NODE_TYPES.contains(&w) {
        return Class::NodeType;
    }
    if w.len() > 3 && w.ends_with("key") {
        return Class::Keyed;
    }
    let all: [&[&str]; 11] = [
        TOP_LEVEL, COMMON, LISTS, MESH, DANGLY, ANIMMESH, AABB, LIGHT, EMITTER, REFERENCE,
        ANIM_NODE,
    ];
    if all.iter().any(|s| s.contains(&w)) {
        return Class::Keyword;
    }
    Class::Name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classes() {
        assert_eq!(class("node"), Class::Structure);
        assert_eq!(class("TriMesh"), Class::NodeType);
        assert_eq!(class("positionkey"), Class::Keyed);
        assert_eq!(class("bitmap"), Class::Keyword);
        assert_eq!(class("-1.5e-3"), Class::Number);
        assert_eq!(class("c_wererat"), Class::Name);
        assert!(known_in_node("emitter", "birthrate"));
        assert!(!known_in_node("trimesh", "birthrate"));
        assert!(known_in_node("dummy", "position"));
    }
}
