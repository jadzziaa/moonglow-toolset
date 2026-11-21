//! The lights Moonglow makes (`mg_module::lights`, which writes them):
//! how their models are named, so that the area's view tells one without
//! the module workspace.

/// The models' names begin so: `mgl_` and twelve hexadecimal digits of
/// the settings.
pub const MODEL_PREFIX: &str = "mgl_";

/// Whether `model` is a light Moonglow made (`mg_module::lights::LightSpec::model_name`).
pub fn is_generated(model: &str) -> bool {
    model.len() == MODEL_PREFIX.len() + 12
        && model.get(..MODEL_PREFIX.len()).is_some_and(|p| p.eq_ignore_ascii_case(MODEL_PREFIX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_generated_light_is_told_by_its_name() {
        assert!(is_generated("mgl_0123456789ab"));
        assert!(is_generated("MGL_0123456789AB"));
        assert!(!is_generated("mgl_0123"));
        assert!(!is_generated("dag_invisible"));
        assert!(!is_generated("mgl\u{e9}0123456789a"), "not cut inside a character");
    }
}
