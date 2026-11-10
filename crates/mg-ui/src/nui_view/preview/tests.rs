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

/// The widest width after each child, in one pass from the end, is what
/// looking at every later child for each gave.
#[test]
fn the_widths_after_each_child_are_those_of_every_later_child() {
    let mut s = Settings::default();
    s.bindings.insert("wide".into(), Binding { value: json!(400.0), ..Default::default() });
    let widths = [
        json!(null),
        json!(150.0),
        json!({"bind": "wide"}),
        json!(-5.0),
        json!("x"),
        json!(90.0),
        json!(1e308),
        json!(null),
        json!(20.0),
        json!({"bind": "missing"}),
    ];
    for take in 0..=widths.len() {
        for skip in 0..=take {
            let children: Vec<Value> = widths[skip..take]
                .iter()
                .map(|w| json!({"type": "row", "width": w, "children": []}))
                .collect();
            let slow: Vec<f32> = (0..children.len())
                .map(|i| {
                    (children[i + 1..].iter())
                        .map(|n| num(&n["width"], &s, None, -1.0))
                        .fold(-1.0, f32::max)
                })
                .collect();
            assert_eq!(containers::widths_after(&children, &s, None), slow, "{skip}..{take}");
        }
    }
    assert_eq!(containers::widths_after(&[], &s, Some(3)), Vec::<f32>::new());
}

/// A chart's line has a point a value while they are no more than two a
/// pixel, and of more the lowest and highest of each pixel's: as many
/// points as the chart is wide, whatever the count, its extremes among
/// them.
#[test]
fn a_chart_s_line_has_no_more_points_than_its_pixels_show() {
    let y = |v: f64| 100.0 - v as f32;
    // A few values: each one's point, what is no number left out.
    let few = [json!(1.0), json!("x"), json!(3.0), json!(null), json!(2.0), json!([4.0])];
    let points = chart::line_points(&few, 10.0, 50.0, 1.0, y);
    assert_eq!(points, [pos2(10.0, 99.0), pos2(110.0, 97.0), pos2(210.0, 98.0)]);
    // 600 values on 300 points, a pixel a point: still every one.
    let wave = |n: usize| -> Vec<Value> {
        (0..n).map(|i| json!(((i * 7919) % 1000) as f64 / 10.0)).collect()
    };
    let all = chart::line_points(&wave(600), 0.0, 0.5, 1.0, y);
    assert_eq!(all.len(), 600);
    // A million on the same 300: two a pixel at most, from left to right,
    // the lowest and the highest of them all still there.
    let data = wave(1_000_000);
    let step = 300.0 / data.len() as f32;
    for pixels in [1.0, 2.0] {
        let points = chart::line_points(&data, 0.0, step, pixels, y);
        let most = (2.0 * 300.0 * pixels) as usize + 2;
        assert!(points.len() <= most && points.len() >= 300, "{} points", points.len());
        assert!(points.windows(2).all(|p| p[0].x <= p[1].x));
        assert!(points.iter().all(|p| (0.0..=300.0).contains(&p.x)));
        let ys = |pick: fn(f32, f32) -> f32, from: f32| points.iter().map(|p| p.y).fold(from, pick);
        assert_eq!((ys(f32::min, f32::MAX), ys(f32::max, f32::MIN)), (y(99.9), y(0.0)));
    }
    assert!(chart::line_points(&[], 0.0, 1.0, 1.0, y).is_empty());
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
