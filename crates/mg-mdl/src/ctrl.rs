//! Controller names and binary IDs. IDs depend on the node type: 100 is
//! self-illumination on meshes and vertical displacement on lights, for
//! instance (nwnmdlcomp's `NmcController.cpp`, checked on every compiled
//! model in the game). The emitters' three-stop values (`alphaMid`,
//! `colorMid`, `percentStart`/`Mid`/`End`, `sizeMid`, `sizeMid_y`), which
//! no game model uses, have the IDs the game's own compiler gives them;
//! nwnmdlcomp's differ (464 alphaMid, 468 colorMid, 480–482, 484, 488), and
//! the game reads its 464 and 468 as `percentStart` and `sizeMid`.

/// Node flag bits (binary node header +0x6C).
pub mod flags {
    pub const HEADER: u32 = 0x1;
    pub const LIGHT: u32 = 0x2;
    pub const EMITTER: u32 = 0x4;
    pub const CAMERA: u32 = 0x8;
    pub const REFERENCE: u32 = 0x10;
    pub const MESH: u32 = 0x20;
    pub const SKIN: u32 = 0x40;
    pub const ANIM: u32 = 0x80;
    pub const DANGLY: u32 = 0x100;
    pub const AABB: u32 = 0x200;
}

const COMMON: [(u32, &str, usize); 3] =
    [(8, "position", 3), (20, "orientation", 4), (36, "scale", 1)];

const MESH: [(u32, &str, usize); 2] = [(100, "selfillumcolor", 3), (128, "alpha", 1)];

const LIGHT: [(u32, &str, usize); 6] = [
    (76, "color", 3),
    (88, "radius", 1),
    (96, "shadowradius", 1),
    (100, "verticaldisplacement", 1),
    (140, "multiplier", 1),
    (144, "ctrl144", 1),
];

/// Emitter controllers (the particle parameters).
pub const EMITTER: [(u32, &str, usize); 39] = [
    (80, "alphaend", 1),
    (84, "alphastart", 1),
    (88, "birthrate", 1),
    (92, "bounce_co", 1),
    (96, "colorend", 3),
    (108, "colorstart", 3),
    (120, "combinetime", 1),
    (124, "drag", 1),
    (128, "fps", 1),
    (132, "frameend", 1),
    (136, "framestart", 1),
    (140, "grav", 1),
    (144, "lifeexp", 1),
    (148, "mass", 1),
    (152, "p2p_bezier2", 1),
    (156, "p2p_bezier3", 1),
    (160, "particlerot", 1),
    (164, "randvel", 1),
    (168, "sizestart", 1),
    (172, "sizeend", 1),
    (176, "sizestart_y", 1),
    (180, "sizeend_y", 1),
    (184, "spread", 1),
    (188, "threshold", 1),
    (192, "velocity", 1),
    (196, "xsize", 1),
    (200, "ysize", 1),
    (204, "blurlength", 1),
    (208, "lightningdelay", 1),
    (212, "lightningradius", 1),
    (216, "lightningscale", 1),
    (228, "detonate", 0),
    (448, "alphamid", 1),
    (452, "colormid", 3),
    (464, "percentstart", 1),
    (465, "percentmid", 1),
    (466, "percentend", 1),
    (468, "sizemid", 1),
    (472, "sizemid_y", 1),
];

fn tables(node_flags: u32) -> impl Iterator<Item = &'static (u32, &'static str, usize)> {
    let mesh: &[_] = if node_flags & flags::MESH != 0 { &MESH } else { &[] };
    let light: &[_] = if node_flags & flags::LIGHT != 0 { &LIGHT } else { &[] };
    let emitter: &[_] = if node_flags & flags::EMITTER != 0 { &EMITTER } else { &[] };
    COMMON.iter().chain(mesh).chain(light).chain(emitter)
}

/// A binary controller ID's name for a node type (`ctrl<ID>` if unknown).
pub fn name(node_flags: u32, id: u32) -> String {
    tables(node_flags).find(|t| t.0 == id).map_or_else(|| format!("ctrl{id}"), |t| t.1.to_string())
}

/// Whether an ASCII keyword (lower case, without `key`) is a controller of
/// this node type, and its number of values.
pub fn columns(node_flags: u32, name: &str) -> Option<usize> {
    let name = match name {
        "setfillumcolor" => "selfillumcolor",
        n => n,
    };
    tables(node_flags).find(|t| t.1 == name).map(|t| t.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_depend_on_the_node_type() {
        assert_eq!(name(0x21, 100), "selfillumcolor");
        assert_eq!(name(0x3, 100), "verticaldisplacement");
        assert_eq!(name(0x5, 88), "birthrate");
        assert_eq!(name(0x3, 88), "radius");
        assert_eq!(name(0x1, 999), "ctrl999");
        assert_eq!(columns(0x21, "setfillumcolor"), Some(3));
        assert_eq!(columns(0x1, "alpha"), None);
    }

    /// What `nwmain compilemodel` writes for an emitter's three-stop values.
    #[test]
    fn mid_values_have_the_games_ids() {
        let names: Vec<String> =
            [448, 452, 464, 465, 466, 468, 472].iter().map(|&id| name(0x5, id)).collect();
        assert_eq!(
            names,
            [
                "alphamid",
                "colormid",
                "percentstart",
                "percentmid",
                "percentend",
                "sizemid",
                "sizemid_y"
            ]
        );
        assert_eq!(columns(0x5, "colormid"), Some(3));
    }
}
