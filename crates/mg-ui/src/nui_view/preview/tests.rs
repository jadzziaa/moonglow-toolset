//! The preview's tests: what its private functions give (here), the canvas
//! under the pointer (`canvas`), the numbers measured in the game client
//! (`native`) and windows no editor would write (`hostile`).
use super::*;

mod canvas;
mod hostile;
mod native;

/// Numbers in text as the client writes them (NWN EE 8193.37,
/// `nui_readings_in_the_client`): a 32-bit float's digits to as many places
/// as asked, no cap at 16, and ties to even.
#[test]
fn numbers_in_text_are_the_client_s() {
    let third = 1.0 / 3.0;
    for (n, places, shown) in [
        (third, 16, "0.3333333432674408"),
        (third, 17, "0.33333334326744080"),
        (third, 20, "0.33333334326744079590"),
        (third, 30, "0.333333343267440795898437500000"),
        (0.5, 0, "0"),
        (2.5, 0, "2"),
        (-2.5, 0, "-2"),
        (1.5, 0, "2"),
        // (Read in the client on 2026-10-29, the third batch: ties to even
        // at 2 places too.)
        (42.125, 2, "42.12"),
        (-42.125, 2, "-42.12"),
        (0.125, 2, "0.12"),
    ] {
        assert_eq!(native_decimal(n, places), shown, "{n} to {places}");
    }
    assert_eq!(native_decimal(1e40, 0), "inf");
    assert_eq!(native_decimal(third, 100_000).len(), 2 + MOST_PLACES);
    let mut s = Settings::default();
    s.bindings.insert("n".into(), Binding { value: json!(third), ..Default::default() });
    let text = string(&json!({"bind": "n", "number_precision": 30}), &s, None);
    assert_eq!(text, "0.333333343267440795898437500000");
}

#[test]
fn native_text_flags_change_ascii_case_and_preserve_unicode() {
    let mut s = Settings::default();
    s.bindings.insert(
        "value".into(),
        Binding { value: json!("MiXeD Ala Żółć 123"), ..Default::default() },
    );
    for (flags, expected) in [
        (0, "MiXeD Ala Żółć 123"),
        (1, "mixed ala Żółć 123"),
        (2, "MIXED ALA Żółć 123"),
        (3, "MIXED ALA Żółć 123"),
    ] {
        assert_eq!(string(&json!({"bind":"value","text_flags":flags}), &s, None), expected);
    }
}

#[test]
fn preview_aspect_alignment_and_list_binds_follow_stock_constants() {
    let target = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 100.0));
    let size = vec2(200.0, 100.0);
    assert_eq!(image_rect(target, size, 0, alignment(0, 0), 1.0).size(), vec2(100.0, 50.0));
    assert_eq!(image_rect(target, size, 1, alignment(0, 0), 1.0).size(), vec2(200.0, 100.0));
    assert_eq!(image_rect(target, size, 5, alignment(0, 0), 1.0), target);
    assert_eq!(image_rect(target, size, 3, alignment(1, 1), 1.0).min, target.min);
    let mut s = Settings::default();
    s.bindings.insert("rows".into(), Binding { value: json!(["A", "B"]), ..Default::default() });
    let v = json!({"bind":"rows"});
    assert_eq!(val(&v, &s, Some(1)), "B");
    assert!(val(&v, &s, Some(2)).is_null());
    assert!(val(&v, &s, None).is_array());
}
