//! The icons' SVG markup read into polylines (shared by `build.rs`, which
//! turns `icons/icons.json` into Rust, and the crate's tests).
//!
//! An icon is a few elements in a 24 by 24 box, drawn as strokes: `path`
//! (every command: M m L l H h V v C c S s Q q T t A a Z z), `circle`,
//! `ellipse`, `rect` (with `rx` and `ry`), `line`, `polyline` and
//! `polygon`. Curves and arcs are cut into straight pieces of at most
//! [`STEP`] units, fine enough to look round at 32 px. A `fill` other
//! than `none` on an element marks its shape filled.

/// The longest straight piece a curve or an arc is cut into, in the 24
/// box's units (at 32 px, two thirds of a pixel; a circle of radius 1
/// strays from its piece by under 0.04 units).
pub(crate) const STEP: f32 = 0.5;

/// One stroke of an icon: points in the 24 box, joined in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Poly {
    pub(crate) points: Vec<[f32; 2]>,
    /// The last point joins the first.
    pub(crate) closed: bool,
    /// Filled as well as stroked (an element with a `fill`).
    pub(crate) filled: bool,
}

/// Reads an icon's markup (`<path d="..."/><circle .../>`) into its
/// strokes.
pub(crate) fn parse_icon(markup: &str) -> Result<Vec<Poly>, String> {
    let mut polys = Vec::new();
    let mut rest = markup.trim();
    while !rest.is_empty() {
        let open = rest.find('<').ok_or_else(|| format!("text outside an element: {rest:?}"))?;
        if !rest[..open].trim().is_empty() {
            return Err(format!("text outside an element: {:?}", &rest[..open]));
        }
        let close = rest[open..].find('>').ok_or("an element not closed")? + open;
        let element = &rest[open + 1..close];
        rest = rest[close + 1..].trim_start();
        let element = element.trim_end_matches('/').trim();
        if element.starts_with('/') {
            // (`</path>` after `<path ...>`.)
            continue;
        }
        let (tag, attrs) = element.split_once(char::is_whitespace).unwrap_or((element, ""));
        let attrs = attributes(attrs)?;
        let get = |name: &str| attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| v.as_str());
        let num = |name: &str| -> Result<f32, String> {
            let v = get(name).ok_or_else(|| format!("<{tag}> without {name}"))?;
            v.trim().parse::<f32>().map_err(|_| format!("<{tag}> {name}={v:?} is not a number"))
        };
        let num_or = |name: &str, or: f32| if get(name).is_some() { num(name) } else { Ok(or) };
        let filled = get("fill").is_some_and(|f| f.trim() != "none");
        let start = polys.len();
        match tag {
            "path" => path(get("d").ok_or("<path> without d")?, &mut polys)?,
            "circle" => {
                let r = num("r")?;
                polys.push(ellipse(num_or("cx", 0.0)?, num_or("cy", 0.0)?, r, r));
            }
            "ellipse" => {
                let (rx, ry) = (num("rx")?, num("ry")?);
                polys.push(ellipse(num_or("cx", 0.0)?, num_or("cy", 0.0)?, rx, ry));
            }
            "rect" => {
                let (x, y) = (num_or("x", 0.0)?, num_or("y", 0.0)?);
                let (w, h) = (num("width")?, num("height")?);
                // (One of rx and ry stands for both when the other is missing.)
                let rx = get("rx").map(|_| num("rx")).transpose()?;
                let ry = get("ry").map(|_| num("ry")).transpose()?;
                let (rx, ry) = match (rx, ry) {
                    (Some(a), Some(b)) => (a, b),
                    (Some(a), None) | (None, Some(a)) => (a, a),
                    (None, None) => (0.0, 0.0),
                };
                polys.push(rect(x, y, w, h, rx, ry));
            }
            "line" => polys.push(Poly {
                points: vec![
                    [num_or("x1", 0.0)?, num_or("y1", 0.0)?],
                    [num_or("x2", 0.0)?, num_or("y2", 0.0)?],
                ],
                closed: false,
                filled: false,
            }),
            "polyline" | "polygon" => {
                let values = numbers(get("points").ok_or("<polyline> without points")?)?;
                if values.len() < 4 || values.len() % 2 != 0 {
                    return Err(format!("<{tag}> points {values:?}"));
                }
                polys.push(Poly {
                    points: values.chunks(2).map(|p| [p[0], p[1]]).collect(),
                    closed: tag == "polygon",
                    filled: false,
                });
            }
            other => return Err(format!("<{other}> is not a shape an icon may have")),
        }
        for p in &mut polys[start..] {
            p.filled = filled;
        }
    }
    if polys.is_empty() {
        return Err("no shapes".into());
    }
    Ok(polys)
}

/// `name="value"` pairs.
fn attributes(text: &str) -> Result<Vec<(&str, String)>, String> {
    let mut out = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let eq = rest.find('=').ok_or_else(|| format!("an attribute without a value: {rest:?}"))?;
        let name = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'');
        let quote = quote.ok_or_else(|| format!("{name}= not quoted"))?;
        let end = after[1..].find(quote).ok_or_else(|| format!("{name}= not closed"))? + 1;
        out.push((name, after[1..end].to_string()));
        rest = after[end + 1..].trim_start();
    }
    Ok(out)
}

/// The numbers of a list (`points`): separated by spaces and commas, or
/// by a sign or a second point alone (`1-14`, `.5.5`).
fn numbers(text: &str) -> Result<Vec<f32>, String> {
    let mut lex = Lexer { s: text.as_bytes(), at: 0 };
    let mut out = Vec::new();
    while lex.more() {
        out.push(lex.number()?);
    }
    Ok(out)
}

struct Lexer<'a> {
    s: &'a [u8],
    at: usize,
}

impl Lexer<'_> {
    fn skip(&mut self) {
        while self.at < self.s.len()
            && (self.s[self.at].is_ascii_whitespace() || self.s[self.at] == b',')
        {
            self.at += 1;
        }
    }

    /// Whether anything but separators is left.
    fn more(&mut self) -> bool {
        self.skip();
        self.at < self.s.len()
    }

    /// A command letter next (not a number).
    fn command(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.at)?;
        c.is_ascii_alphabetic().then(|| {
            self.at += 1;
            c
        })
    }

    /// Whether a number comes next.
    fn number_next(&mut self) -> bool {
        self.skip();
        self.s.get(self.at).is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Result<f32, String> {
        self.skip();
        let start = self.at;
        let s = self.s;
        let mut i = self.at;
        if i < s.len() && matches!(s[i], b'-' | b'+') {
            i += 1;
        }
        let mut dot = false;
        let mut digits = false;
        while i < s.len() && (s[i].is_ascii_digit() || (s[i] == b'.' && !dot)) {
            dot |= s[i] == b'.';
            digits |= s[i].is_ascii_digit();
            i += 1;
        }
        if digits && i < s.len() && matches!(s[i], b'e' | b'E') {
            let mut j = i + 1;
            if j < s.len() && matches!(s[j], b'-' | b'+') {
                j += 1;
            }
            if j < s.len() && s[j].is_ascii_digit() {
                while j < s.len() && s[j].is_ascii_digit() {
                    j += 1;
                }
                i = j;
            }
        }
        let text = std::str::from_utf8(&s[start..i]).map_err(|e| e.to_string())?;
        if !digits {
            return Err(format!("a number expected at {:?}", self.rest()));
        }
        self.at = i;
        text.parse::<f32>().map_err(|_| format!("{text:?} is not a number"))
    }

    /// An arc's flag: one `0` or `1`, which may touch what follows
    /// (`a1 1 0 01.5 2`).
    fn flag(&mut self) -> Result<bool, String> {
        self.skip();
        let flag = match self.s.get(self.at) {
            Some(b'0') => false,
            Some(b'1') => true,
            _ => return Err(format!("an arc's flag expected at {:?}", self.rest())),
        };
        self.at += 1;
        Ok(flag)
    }

    fn rest(&self) -> String {
        String::from_utf8_lossy(&self.s[self.at..]).into_owned()
    }
}

/// Builds the strokes of one path.
struct Pen<'a> {
    polys: &'a mut Vec<Poly>,
    /// The stroke being drawn (its first point where it began).
    current: Vec<[f32; 2]>,
    at: [f32; 2],
    start: [f32; 2],
}

impl Pen<'_> {
    fn finish(&mut self, closed: bool) {
        if self.current.len() > 1 {
            let mut points = std::mem::take(&mut self.current);
            if closed && points.len() > 2 && points.first() == points.last() {
                points.pop();
            }
            self.polys.push(Poly { points, closed, filled: false });
        }
        self.current.clear();
    }

    fn move_to(&mut self, p: [f32; 2]) {
        self.finish(false);
        self.at = p;
        self.start = p;
    }

    fn line_to(&mut self, p: [f32; 2]) {
        if self.current.is_empty() {
            self.current.push(self.at);
        }
        self.current.push(p);
        self.at = p;
    }
}

fn path(d: &str, polys: &mut Vec<Poly>) -> Result<(), String> {
    let mut lex = Lexer { s: d.as_bytes(), at: 0 };
    let mut pen = Pen { polys, current: Vec::new(), at: [0.0, 0.0], start: [0.0, 0.0] };
    // The last curve's second control point (for S and T), and its kind.
    let mut last_cubic: Option<[f32; 2]> = None;
    let mut last_quad: Option<[f32; 2]> = None;
    let mut cmd = lex.command().ok_or_else(|| format!("a path must begin with M: {d:?}"))?;
    if !matches!(cmd, b'M' | b'm') {
        return Err(format!("a path must begin with M: {d:?}"));
    }
    loop {
        let rel = cmd.is_ascii_lowercase();
        let base = if rel { pen.at } else { [0.0, 0.0] };
        let point = |lex: &mut Lexer<'_>| -> Result<[f32; 2], String> {
            Ok([base[0] + lex.number()?, base[1] + lex.number()?])
        };
        let (mut cubic, mut quad) = (None, None);
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let p = point(&mut lex)?;
                pen.move_to(p);
                // (Pairs after the first are lines.)
                cmd = if rel { b'l' } else { b'L' };
                if lex.number_next() {
                    continue;
                }
            }
            b'L' => {
                let p = point(&mut lex)?;
                pen.line_to(p);
            }
            b'H' => {
                let x = lex.number()? + if rel { pen.at[0] } else { 0.0 };
                pen.line_to([x, pen.at[1]]);
            }
            b'V' => {
                let y = lex.number()? + if rel { pen.at[1] } else { 0.0 };
                pen.line_to([pen.at[0], y]);
            }
            b'C' | b'S' => {
                let c1 = if cmd.eq_ignore_ascii_case(&b'C') {
                    point(&mut lex)?
                } else {
                    reflect(last_cubic, pen.at)
                };
                let c2 = point(&mut lex)?;
                let p = point(&mut lex)?;
                let from = pen.at;
                let n = pieces(&[from, c1, c2, p]);
                for i in 1..=n {
                    let t = i as f32 / n as f32;
                    pen.line_to(cubic_at(from, c1, c2, p, t));
                }
                cubic = Some(c2);
            }
            b'Q' | b'T' => {
                let c = if cmd.eq_ignore_ascii_case(&b'Q') {
                    point(&mut lex)?
                } else {
                    reflect(last_quad, pen.at)
                };
                let p = point(&mut lex)?;
                let from = pen.at;
                let n = pieces(&[from, c, p]);
                for i in 1..=n {
                    let t = i as f32 / n as f32;
                    let u = 1.0 - t;
                    pen.line_to([
                        u * u * from[0] + 2.0 * u * t * c[0] + t * t * p[0],
                        u * u * from[1] + 2.0 * u * t * c[1] + t * t * p[1],
                    ]);
                }
                quad = Some(c);
            }
            b'A' => {
                let (rx, ry) = (lex.number()?.abs(), lex.number()?.abs());
                let rotation = lex.number()?;
                let (large, sweep) = (lex.flag()?, lex.flag()?);
                let p = point(&mut lex)?;
                for q in arc(pen.at, rx, ry, rotation, large, sweep, p) {
                    pen.line_to(q);
                }
            }
            b'Z' => {
                if pen.current.len() > 1 && pen.at != pen.start {
                    pen.line_to(pen.start);
                }
                pen.finish(true);
                pen.at = pen.start;
            }
            other => return Err(format!("path command {:?} unknown", other as char)),
        }
        last_cubic = cubic;
        last_quad = quad;
        if let Some(next) = lex.command() {
            cmd = next;
        } else if !lex.more() {
            break;
        } else if cmd.eq_ignore_ascii_case(&b'Z') {
            return Err(format!("a number after Z: {d:?}"));
        }
        // (Otherwise the same command again with more numbers.)
    }
    pen.finish(false);
    Ok(())
}

/// A smooth curve's first control point: the last one's mirror image.
fn reflect(last: Option<[f32; 2]>, at: [f32; 2]) -> [f32; 2] {
    last.map_or(at, |c| [2.0 * at[0] - c[0], 2.0 * at[1] - c[1]])
}

fn cubic_at(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    [w0 * a[0] + w1 * b[0] + w2 * c[0] + w3 * d[0], w0 * a[1] + w1 * b[1] + w2 * c[1] + w3 * d[1]]
}

/// How many pieces a curve with these control points is cut into: its
/// control polygon (never shorter than the curve) in [`STEP`]s.
fn pieces(control: &[[f32; 2]]) -> usize {
    let length: f32 = control.windows(2).map(|w| distance(w[0], w[1])).sum();
    ((length / STEP).ceil() as usize).clamp(1, 200)
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// An ellipse as a closed stroke.
fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Poly {
    let n = ((std::f32::consts::TAU * rx.max(ry) / STEP).ceil() as usize).max(16);
    let points = (0..n)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / n as f32;
            [cx + rx * a.cos(), cy + ry * a.sin()]
        })
        .collect();
    Poly { points, closed: true, filled: false }
}

/// A rectangle, its corners rounded by `rx` and `ry` (as SVG clamps them,
/// to half its sides).
fn rect(x: f32, y: f32, w: f32, h: f32, rx: f32, ry: f32) -> Poly {
    let (rx, ry) = (rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0));
    if rx <= 0.0 || ry <= 0.0 {
        let points = vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
        return Poly { points, closed: true, filled: false };
    }
    let n = ((std::f32::consts::FRAC_PI_2 * rx.max(ry) / STEP).ceil() as usize).max(3);
    let mut points = Vec::new();
    // Each corner's center and the angle its quarter starts at, clockwise
    // (y down) from the top right.
    let corners = [
        ([x + w - rx, y + ry], -std::f32::consts::FRAC_PI_2),
        ([x + w - rx, y + h - ry], 0.0),
        ([x + rx, y + h - ry], std::f32::consts::FRAC_PI_2),
        ([x + rx, y + ry], std::f32::consts::PI),
    ];
    for (c, from) in corners {
        for i in 0..=n {
            let a = from + std::f32::consts::FRAC_PI_2 * i as f32 / n as f32;
            points.push([c[0] + rx * a.cos(), c[1] + ry * a.sin()]);
        }
    }
    Poly { points, closed: true, filled: false }
}

/// The points of an SVG arc from `from` to `to` (the last one `to`), by the
/// endpoint-to-center conversion of SVG 1.1's implementation notes (F.6).
fn arc(
    from: [f32; 2],
    rx: f32,
    ry: f32,
    rotation_degrees: f32,
    large: bool,
    sweep: bool,
    to: [f32; 2],
) -> Vec<[f32; 2]> {
    if from == to {
        return Vec::new();
    }
    if rx == 0.0 || ry == 0.0 {
        return vec![to];
    }
    let (sin, cos) = rotation_degrees.to_radians().sin_cos();
    let dx = (from[0] - to[0]) / 2.0;
    let dy = (from[1] - to[1]) / 2.0;
    let x1 = cos * dx + sin * dy;
    let y1 = -sin * dx + cos * dy;
    let (mut rx, mut ry) = (rx, ry);
    // Radii too small to reach are scaled up until they do.
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        rx *= lambda.sqrt();
        ry *= lambda.sqrt();
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = (num / den).max(0.0).sqrt();
    if large == sweep {
        k = -k;
    }
    let cx1 = k * rx * y1 / ry;
    let cy1 = -k * ry * x1 / rx;
    let cx = cos * cx1 - sin * cy1 + (from[0] + to[0]) / 2.0;
    let cy = sin * cx1 + cos * cy1 + (from[1] + to[1]) / 2.0;
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let dot = ux * vx + uy * vy;
        let len = (ux * ux + uy * uy).sqrt() * (vx * vx + vy * vy).sqrt();
        let a = (dot / len).clamp(-1.0, 1.0).acos();
        if ux * vy - uy * vx < 0.0 { -a } else { a }
    };
    let theta1 = angle(1.0, 0.0, (x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut delta = angle((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
    let tau = std::f32::consts::TAU;
    if !sweep && delta > 0.0 {
        delta -= tau;
    } else if sweep && delta < 0.0 {
        delta += tau;
    }
    let n = ((delta.abs() * rx.max(ry) / STEP).ceil() as usize).max(2);
    let mut points: Vec<[f32; 2]> = (1..n)
        .map(|i| {
            let t = theta1 + delta * i as f32 / n as f32;
            let (x, y) = (rx * t.cos(), ry * t.sin());
            [cos * x - sin * y + cx, sin * x + cos * y + cy]
        })
        .collect();
    points.push(to);
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(markup: &str) -> Poly {
        let mut polys = parse_icon(markup).unwrap();
        assert_eq!(polys.len(), 1, "{markup}");
        polys.remove(0)
    }

    fn near(a: [f32; 2], b: [f32; 2]) -> bool {
        distance(a, b) < 0.001
    }

    #[test]
    fn numbers_touch_as_svg_lets_them() {
        assert_eq!(
            numbers(".5.5 1-14,2e1 -3.25e-1").unwrap(),
            [0.5, 0.5, 1.0, -14.0, 20.0, -0.325]
        );
        assert_eq!(numbers("+4 - 1").map_err(|_| ()), Err(()));
    }

    #[test]
    fn lines_and_their_short_forms() {
        let p = one(r#"<path d="M4 21v-7h2.5V10H1L2 3l1 1 1 1z"/>"#);
        assert!(p.closed);
        let expect = [[4.0, 21.0], [4.0, 14.0], [6.5, 14.0], [6.5, 10.0], [1.0, 10.0], [2.0, 3.0]];
        for (a, b) in p.points.iter().zip(expect) {
            assert!(near(*a, b), "{a:?} {b:?}");
        }
        // (Implicit l after l: the pairs go on as lines.)
        assert!(near(p.points[6], [3.0, 4.0]) && near(p.points[7], [4.0, 5.0]));
        // M with more pairs: lines; m relative.
        let p = one(r#"<path d="m1 1 2 0 0 2"/>"#);
        assert_eq!(p.points.len(), 3);
        assert!(near(p.points[2], [3.0, 3.0]));
    }

    #[test]
    fn several_strokes_in_a_path() {
        let polys = parse_icon(r#"<path d="M1 1h4M1 5h4zl2 2"/>"#).unwrap();
        assert_eq!(polys.len(), 3);
        // (After z, the pen is back at the subpath's start.)
        assert!(near(polys[2].points[0], [1.0, 5.0]));
        assert!(near(polys[2].points[1], [3.0, 7.0]));
    }

    /// An arc's points all lie on its circle, glued flags and all.
    #[test]
    fn arcs_lie_on_their_circle() {
        // A half circle of radius 5 about (12, 12), the flags touching.
        let p = one(r#"<path d="M7 12a5 5 0 0110 0"/>"#);
        assert!(p.points.len() > 10);
        for q in &p.points {
            assert!((distance(*q, [12.0, 12.0]) - 5.0).abs() < 0.01, "{q:?}");
        }
        assert!(p.points.iter().all(|q| q[1] <= 12.001), "sweep 1 goes over the top");
        assert!(near(*p.points.last().unwrap(), [17.0, 12.0]));
        // The other sweep goes under.
        let p = one(r#"<path d="M7 12A5 5 0 0 0 17 12"/>"#);
        assert!(p.points.iter().all(|q| q[1] >= 11.999));
        // Radii too small are scaled up to reach.
        let p = one(r#"<path d="M7 12a1 1 0 0 1 10 0"/>"#);
        for q in &p.points {
            assert!((distance(*q, [12.0, 12.0]) - 5.0).abs() < 0.01);
        }
    }

    #[test]
    fn curves_end_where_they_should() {
        let p = one(r#"<path d="M2 2C2 10 10 10 10 2s8 8 8 0"/>"#);
        assert!(p.points.iter().any(|q| near(*q, [10.0, 2.0])));
        assert!(near(*p.points.last().unwrap(), [18.0, 2.0]));
        // The cubic's middle: (6, 8).
        assert!(p.points.iter().any(|q| distance(*q, [6.0, 8.0]) < 0.3));
        let p = one(r#"<path d="M2 2Q6 10 10 2T18 2"/>"#);
        assert!(p.points.iter().any(|q| distance(*q, [6.0, 6.0]) < 0.3), "the quad's middle");
        // T mirrors the control point: its middle under the line.
        assert!(p.points.iter().any(|q| distance(*q, [14.0, -2.0]) < 0.3));
        assert!(near(*p.points.last().unwrap(), [18.0, 2.0]));
        let p = one(r#"<path d="M2 2q4 8 8 0t8 0"/>"#);
        assert!(near(*p.points.last().unwrap(), [18.0, 2.0]));
    }

    #[test]
    fn every_element_kind() {
        let c = one(r#"<circle cx="12" cy="12" r="3"/>"#);
        assert!(
            c.closed && c.points.iter().all(|q| (distance(*q, [12.0, 12.0]) - 3.0).abs() < 1e-4)
        );
        let e = one(r#"<ellipse cx="12" cy="12" rx="6" ry="2"/>"#);
        assert!(e.points.iter().any(|q| near(*q, [18.0, 12.0])));
        assert!(e.points.iter().all(|q| (q[1] - 12.0).abs() <= 2.0001));
        let r = one(r#"<rect x="2" y="4" width="20" height="16"/>"#);
        assert_eq!(r.points, [[2.0, 4.0], [22.0, 4.0], [22.0, 20.0], [2.0, 20.0]]);
        // Rounded: corners on quarter circles of radius 2, sides straight.
        let r = one(r#"<rect x="2" y="4" width="20" height="16" rx="2"/>"#);
        assert!(r.closed);
        assert!(
            r.points.iter().all(|q| (2.0..=22.0).contains(&q[0]) && (4.0..=20.0).contains(&q[1]))
        );
        assert!(
            r.points.iter().any(|q| near(*q, [20.0, 4.0]))
                && r.points.iter().any(|q| near(*q, [22.0, 6.0]))
        );
        assert!(!r.points.iter().any(|q| near(*q, [22.0, 4.0])), "the corner is cut");
        let l = one(r#"<line x1="1" y1="2" x2="3" y2="4"/>"#);
        assert_eq!(l.points, [[1.0, 2.0], [3.0, 4.0]]);
        let pl = one(r#"<polyline points="1,1 5,1 5,5"/>"#);
        assert!(!pl.closed && pl.points.len() == 3);
        let pg = one(r#"<polygon points="1 1 5 1 5 5"/>"#);
        assert!(pg.closed && pg.points.len() == 3);
        let f = one(r#"<circle cx="4" cy="4" r="1" fill="currentColor"/>"#);
        assert!(f.filled);
        assert!(!one(r#"<circle cx="4" cy="4" r="1" fill="none"/>"#).filled);
    }

    /// Bad markup is an error, never a panic.
    #[test]
    fn bad_markup_is_an_error() {
        for bad in [
            "",
            "text",
            "<path",
            r#"<path d="M1"/>"#,
            r#"<path d="L1 1"/>"#,
            r#"<path d="M1 1 a1 1 0 2 1 3 3"/>"#,
            r#"<path d="M1 1 X3"/>"#,
            r#"<path d="M1 1 z 4"/>"#,
            r#"<path d="M1 1 c1 2"/>"#,
            r#"<circle cx="1" cy="1"/>"#,
            r#"<circle cx="1" cy="1" r="x"/>"#,
            r#"<rect x="1"/>"#,
            r#"<polygon points="1 1 2"/>"#,
            r#"<image href="x"/>"#,
            r#"<path d="M1 1 h"/>"#,
            r#"<path d='M1 1 h2/>"#,
            r#"<path d=M1/>"#,
        ] {
            assert!(parse_icon(bad).is_err(), "{bad:?}");
        }
        // Every prefix of a real icon: parsed or refused, never a panic.
        let markup = r#"<path d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3"/><rect x="2" y="2" width="4" height="4" rx="1"/>"#;
        for end in 0..=markup.len() {
            let _ = parse_icon(&markup[..end]);
        }
    }
}
