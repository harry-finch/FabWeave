//! Paramètre `image` (R-025) : un dessin SVG ou PNG transformé en contours
//! fermés, centrés sur (0, 0) et mis à l'échelle (le plus grand côté vaut
//! `size` mm). Lecture pure et déterministe, dans le moteur ; le fichier
//! vient de l'utilisateur, rien n'y est supposé juste : toute faute donne un
//! code stable, jamais une panique.
//!
//! - **SVG** : `path` (toutes les commandes, courbes et arcs aplatis),
//!   `polygon`, `rect`, `circle`, `ellipse`, transformations (`translate`,
//!   `scale`, `rotate`, `matrix`) des éléments et des groupes ; le contenu de
//!   `defs`, `clipPath`, `mask`, `symbol` est ignoré.
//! - **PNG** : 8 bits par canal, non entrelacé (gris, RVB, palette, avec ou
//!   sans transparence). Encre : les pixels opaques si l'image a de la
//!   transparence, sinon ceux plus sombres que le seuil. Contours suivis le
//!   long des bords des pixels (par les milieux de leurs arêtes), puis
//!   simplifiés.
//!
//! Le script reçoit un objet `Drawing` : `d.contours()` (tableau de listes de
//! points à z = 0, un contour dans un autre est un trou), `d.width`,
//! `d.height`, `d.empty`.

use std::f64::consts::TAU;

use crate::image::base64_decode;

/// Taille de fichier, côté d'une image, contours et points au plus.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_SIDE: u32 = 2048;
const MAX_CONTOURS: usize = 2000;
const MAX_POINTS: usize = 200_000;
/// Points par courbe de Bézier et par arc (avant simplification).
const CURVE_STEPS: usize = 16;
/// Écart de simplification d'un contour tracé dans un PNG (pixels).
const PNG_SIMPLIFY: f64 = 0.35;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    /// Contours fermés (premier point non répété), en mm.
    pub contours: Vec<Vec<[f64; 2]>>,
    pub width: f64,
    pub height: f64,
}

type Err = (&'static str, String);

fn err(code: &'static str, d: impl Into<String>) -> Err {
    (code, d.into())
}

/// Dessin d'une valeur : texte SVG, ou PNG en `data:image/png;base64,…` ;
/// vide : aucun dessin.
pub fn read(value: &str, size: f64, threshold: f64) -> Result<Drawing, Err> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(Drawing::default());
    }
    if v.len() > MAX_BYTES * 4 / 3 + 64 {
        return Err(err("image.too_large", format!("{} octets", v.len())));
    }
    let raw = if let Some(b64) = v.strip_prefix("data:image/png;base64,") {
        let bytes = base64_decode(b64).ok_or_else(|| err("image.invalid", "base64"))?;
        png_contours(&bytes, threshold)?
    } else if v.starts_with('<') {
        svg_contours(v)?
    } else {
        return Err(err("image.invalid", "ni SVG ni PNG"));
    };
    let raw: Vec<Vec<[f64; 2]>> = raw
        .into_iter()
        .filter(|c| c.len() >= 3 && area(c).abs() > 1e-9)
        .collect();
    if raw.is_empty() {
        return Err(err("image.empty", ""));
    }
    if raw.len() > MAX_CONTOURS || raw.iter().map(Vec::len).sum::<usize>() > MAX_POINTS {
        return Err(err("image.too_large", format!("{} contours", raw.len())));
    }
    let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
    for p in raw.iter().flatten() {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let span = (hi[0] - lo[0]).max(hi[1] - lo[1]);
    if !(span > 0.0 && span.is_finite()) {
        return Err(err("image.empty", ""));
    }
    let k = size / span;
    let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
    Ok(Drawing {
        contours: raw
            .into_iter()
            .map(|l| {
                l.into_iter()
                    .map(|p| [(p[0] - c[0]) * k, (p[1] - c[1]) * k])
                    .collect()
            })
            .collect(),
        width: (hi[0] - lo[0]) * k,
        height: (hi[1] - lo[1]) * k,
    })
}

fn area(c: &[[f64; 2]]) -> f64 {
    (0..c.len())
        .map(|i| {
            let (a, b) = (c[i], c[(i + 1) % c.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        / 2.0
}

// --- SVG ----------------------------------------------------------------------

/// Transformation affine `[a, b, c, d, e, f]` : (x, y) → (ax + cy + e, bx + dy + f).
type Affine = [f64; 6];
const ID: Affine = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

fn mul(m: &Affine, n: &Affine) -> Affine {
    [
        m[0] * n[0] + m[2] * n[1],
        m[1] * n[0] + m[3] * n[1],
        m[0] * n[2] + m[2] * n[3],
        m[1] * n[2] + m[3] * n[3],
        m[0] * n[4] + m[2] * n[5] + m[4],
        m[1] * n[4] + m[3] * n[5] + m[5],
    ]
}

fn apply(m: &Affine, p: [f64; 2]) -> [f64; 2] {
    [
        m[0] * p[0] + m[2] * p[1] + m[4],
        m[1] * p[0] + m[3] * p[1] + m[5],
    ]
}

/// Nombres d'une liste SVG (séparés par des blancs, des virgules, ou collés :
/// `1-2`, `.5.5`, `1e-3`).
fn numbers(s: &str) -> Vec<f64> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let start = i;
        if b[i] == b'-' || b[i] == b'+' {
            i += 1;
        }
        let (mut dot, mut digits) = (false, false);
        while i < b.len() && (b[i].is_ascii_digit() || (b[i] == b'.' && !dot)) {
            dot |= b[i] == b'.';
            digits |= b[i].is_ascii_digit();
            i += 1;
        }
        if digits && i < b.len() && (b[i] == b'e' || b[i] == b'E') {
            let save = i;
            i += 1;
            if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
                i += 1;
            }
            if i < b.len() && b[i].is_ascii_digit() {
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            } else {
                i = save;
            }
        }
        if digits {
            if let Ok(v) = s[start..i].parse::<f64>() {
                out.push(v);
            }
        } else {
            i = start + 1;
        }
    }
    out
}

fn transform(s: &str) -> Affine {
    let mut m = ID;
    let mut rest = s;
    while let Some(open) = rest.find('(') {
        let name = rest[..open].trim().trim_start_matches(',').trim();
        let Some(close) = rest[open..].find(')') else {
            break;
        };
        let v = numbers(&rest[open + 1..open + close]);
        let g = |i: usize, d: f64| v.get(i).copied().unwrap_or(d);
        let t: Affine = match name {
            "translate" => [1.0, 0.0, 0.0, 1.0, g(0, 0.0), g(1, 0.0)],
            "scale" => [g(0, 1.0), 0.0, 0.0, g(1, g(0, 1.0)), 0.0, 0.0],
            "matrix" if v.len() == 6 => [v[0], v[1], v[2], v[3], v[4], v[5]],
            "rotate" => {
                let a = g(0, 0.0).to_radians();
                let (c, s) = (a.cos(), a.sin());
                let (cx, cy) = (g(1, 0.0), g(2, 0.0));
                [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy]
            }
            "skewX" => [1.0, 0.0, g(0, 0.0).to_radians().tan(), 1.0, 0.0, 0.0],
            "skewY" => [1.0, g(0, 0.0).to_radians().tan(), 0.0, 1.0, 0.0, 0.0],
            _ => ID,
        };
        m = mul(&m, &t);
        rest = &rest[open + close + 1..];
    }
    m
}

/// Valeur d'un attribut dans le texte d'une balise.
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let mut from = 0;
    while let Some(i) = tag[from..].find(name) {
        let at = from + i;
        let before = tag[..at].chars().last();
        let after = tag[at + name.len()..].trim_start();
        if before.is_some_and(char::is_whitespace) && after.starts_with('=') {
            let v = after[1..].trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let end = v[1..].find(q)?;
                return Some(&v[1..1 + end]);
            }
        }
        from = at + name.len();
    }
    None
}

fn num_attr(tag: &str, name: &str) -> f64 {
    attr(tag, name)
        .and_then(|v| numbers(v).first().copied())
        .unwrap_or(0.0)
}

fn svg_contours(src: &str) -> Result<Vec<Vec<[f64; 2]>>, Err> {
    if !src.contains("<svg") {
        return Err(err("image.invalid", "pas de balise svg"));
    }
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut stack: Vec<(String, Affine)> = vec![(String::new(), ID)];
    let mut hidden = 0usize;
    let mut rest = src;
    while let Some(lt) = rest.find('<') {
        rest = &rest[lt + 1..];
        // Commentaires, déclarations, CDATA : sautés.
        if let Some(r) = rest.strip_prefix("!--") {
            rest = r.find("-->").map_or("", |i| &r[i + 3..]);
            continue;
        }
        let Some(gt) = rest.find('>') else { break };
        let tag = &rest[..gt];
        rest = &rest[gt + 1..];
        if tag.starts_with('!') || tag.starts_with('?') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            let name = name.trim();
            if let Some(pos) = stack.iter().rposition(|(n, _)| n == name) {
                for (n, _) in stack.drain(pos..) {
                    if matches!(
                        n.as_str(),
                        "defs" | "clipPath" | "mask" | "symbol" | "pattern"
                    ) {
                        hidden = hidden.saturating_sub(1);
                    }
                }
                if stack.is_empty() {
                    stack.push((String::new(), ID));
                }
            }
            continue;
        }
        let closed = tag.ends_with('/');
        let name = tag
            .split(|c: char| c.is_whitespace() || c == '/')
            .next()
            .unwrap_or("");
        let parent = stack.last().map_or(ID, |s| s.1);
        let m = attr(tag, "transform").map_or(parent, |t| mul(&parent, &transform(t)));
        let invisible = matches!(name, "defs" | "clipPath" | "mask" | "symbol" | "pattern");
        if !closed {
            if invisible {
                hidden += 1;
            }
            stack.push((name.to_string(), m));
        }
        if hidden > 0 || invisible {
            continue;
        }
        let mut shapes: Vec<Vec<[f64; 2]>> = Vec::new();
        match name {
            "path" => shapes = path_data(attr(tag, "d").unwrap_or(""))?,
            "polygon" | "polyline" => {
                let v = numbers(attr(tag, "points").unwrap_or(""));
                shapes.push(v.chunks_exact(2).map(|p| [p[0], p[1]]).collect());
            }
            "rect" => {
                let (x, y) = (num_attr(tag, "x"), num_attr(tag, "y"));
                let (w, h) = (num_attr(tag, "width"), num_attr(tag, "height"));
                shapes.push(vec![[x, y], [x + w, y], [x + w, y + h], [x, y + h]]);
            }
            "circle" | "ellipse" => {
                let (cx, cy) = (num_attr(tag, "cx"), num_attr(tag, "cy"));
                let (rx, ry) = if name == "circle" {
                    let r = num_attr(tag, "r");
                    (r, r)
                } else {
                    (num_attr(tag, "rx"), num_attr(tag, "ry"))
                };
                let n = 4 * CURVE_STEPS;
                shapes.push(
                    (0..n)
                        .map(|i| {
                            let a = i as f64 / n as f64 * TAU;
                            [cx + rx * a.cos(), cy + ry * a.sin()]
                        })
                        .collect(),
                );
            }
            _ => {}
        }
        for s in shapes {
            // Y du SVG vers le bas : retourné pour le plateau.
            out.push(
                s.into_iter()
                    .map(|p| apply(&m, p))
                    .map(|p| [p[0], -p[1]])
                    .collect(),
            );
            if out.len() > MAX_CONTOURS {
                return Err(err("image.too_large", "contours"));
            }
        }
    }
    Ok(out)
}

/// Sous-chemins d'un attribut `d`, chacun fermé.
fn path_data(d: &str) -> Result<Vec<Vec<[f64; 2]>>, Err> {
    let mut out: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let (mut p, mut start) = ([0.0; 2], [0.0; 2]);
    let mut last_ctrl: Option<[f64; 2]> = None;
    let mut last_quad: Option<[f64; 2]> = None;
    let mut points = 0usize;
    let b = d.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if !c.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let j = (i + 1..b.len())
            .find(|&k| (b[k] as char).is_ascii_alphabetic() && b[k] != b'e' && b[k] != b'E')
            .unwrap_or(b.len());
        let v = numbers(&d[i + 1..j]);
        i = j;
        let rel = c.is_ascii_lowercase();
        let base = |p: [f64; 2]| if rel { p } else { [0.0, 0.0] };
        let flush = |cur: &mut Vec<[f64; 2]>, out: &mut Vec<Vec<[f64; 2]>>| {
            if cur.len() >= 3 {
                out.push(std::mem::take(cur));
            } else {
                cur.clear();
            }
        };
        match c.to_ascii_uppercase() {
            'M' => {
                flush(&mut cur, &mut out);
                for (k, q) in v.chunks_exact(2).enumerate() {
                    let o = base(p);
                    p = [o[0] + q[0], o[1] + q[1]];
                    if k == 0 {
                        start = p;
                    }
                    cur.push(p);
                }
                last_ctrl = None;
                last_quad = None;
            }
            'L' | 'H' | 'V' => {
                let step = if c.eq_ignore_ascii_case(&'L') { 2 } else { 1 };
                for q in v.chunks_exact(step) {
                    let o = base(p);
                    p = match c.to_ascii_uppercase() {
                        'L' => [o[0] + q[0], o[1] + q[1]],
                        'H' => [o[0] + q[0], p[1]],
                        _ => [p[0], o[1] + q[0]],
                    };
                    cur.push(p);
                }
                last_ctrl = None;
                last_quad = None;
            }
            'C' | 'S' => {
                let n = if c.eq_ignore_ascii_case(&'C') { 6 } else { 4 };
                for q in v.chunks_exact(n) {
                    let o = base(p);
                    let (c1, c2, e) = if n == 6 {
                        (
                            [o[0] + q[0], o[1] + q[1]],
                            [o[0] + q[2], o[1] + q[3]],
                            [o[0] + q[4], o[1] + q[5]],
                        )
                    } else {
                        let c1 = last_ctrl.map_or(p, |l| [2.0 * p[0] - l[0], 2.0 * p[1] - l[1]]);
                        (c1, [o[0] + q[0], o[1] + q[1]], [o[0] + q[2], o[1] + q[3]])
                    };
                    for s in 1..=CURVE_STEPS {
                        let t = s as f64 / CURVE_STEPS as f64;
                        let u = 1.0 - t;
                        let w = [u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t];
                        cur.push([
                            w[0] * p[0] + w[1] * c1[0] + w[2] * c2[0] + w[3] * e[0],
                            w[0] * p[1] + w[1] * c1[1] + w[2] * c2[1] + w[3] * e[1],
                        ]);
                    }
                    last_ctrl = Some(c2);
                    p = e;
                }
                last_quad = None;
            }
            'Q' | 'T' => {
                let n = if c.eq_ignore_ascii_case(&'Q') { 4 } else { 2 };
                for q in v.chunks_exact(n) {
                    let o = base(p);
                    let (c1, e) = if n == 4 {
                        ([o[0] + q[0], o[1] + q[1]], [o[0] + q[2], o[1] + q[3]])
                    } else {
                        let c1 = last_quad.map_or(p, |l| [2.0 * p[0] - l[0], 2.0 * p[1] - l[1]]);
                        (c1, [o[0] + q[0], o[1] + q[1]])
                    };
                    for s in 1..=CURVE_STEPS {
                        let t = s as f64 / CURVE_STEPS as f64;
                        let u = 1.0 - t;
                        cur.push([
                            u * u * p[0] + 2.0 * u * t * c1[0] + t * t * e[0],
                            u * u * p[1] + 2.0 * u * t * c1[1] + t * t * e[1],
                        ]);
                    }
                    last_quad = Some(c1);
                    p = e;
                }
                last_ctrl = None;
            }
            'A' => {
                for q in v.chunks_exact(7) {
                    let o = base(p);
                    let e = [o[0] + q[5], o[1] + q[6]];
                    arc(&mut cur, p, e, q[0], q[1], q[2], q[3] != 0.0, q[4] != 0.0);
                    p = e;
                }
                last_ctrl = None;
                last_quad = None;
            }
            'Z' => {
                flush(&mut cur, &mut out);
                p = start;
                last_ctrl = None;
                last_quad = None;
            }
            _ => return Err(err("image.invalid", format!("commande {c}"))),
        }
        points += cur.len();
        if points > MAX_POINTS * 4 {
            return Err(err("image.too_large", "points"));
        }
    }
    if cur.len() >= 3 {
        out.push(cur);
    }
    Ok(out)
}

/// Arc elliptique SVG (paramétrage par les extrémités), aplati.
#[allow(clippy::too_many_arguments)]
fn arc(
    out: &mut Vec<[f64; 2]>,
    p: [f64; 2],
    e: [f64; 2],
    rx: f64,
    ry: f64,
    rot: f64,
    large: bool,
    sweep: bool,
) {
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx < 1e-12 || ry < 1e-12 || (p[0] == e[0] && p[1] == e[1]) {
        out.push(e);
        return;
    }
    let phi = rot.to_radians();
    let (c, s) = (phi.cos(), phi.sin());
    let (dx, dy) = ((p[0] - e[0]) / 2.0, (p[1] - e[1]) / 2.0);
    let (x1, y1) = (c * dx + s * dy, -s * dx + c * dy);
    let l = x1 * x1 / (rx * rx) + y1 * y1 / (ry * ry);
    if l > 1.0 {
        rx *= l.sqrt();
        ry *= l.sqrt();
    }
    let num = (rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1).max(0.0);
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut k = (num / den).sqrt();
    if large == sweep {
        k = -k;
    }
    let (cx1, cy1) = (k * rx * y1 / ry, -k * ry * x1 / rx);
    let (cx, cy) = (
        c * cx1 - s * cy1 + (p[0] + e[0]) / 2.0,
        s * cx1 + c * cy1 + (p[1] + e[1]) / 2.0,
    );
    let ang = |ux: f64, uy: f64| uy.atan2(ux);
    let a1 = ang((x1 - cx1) / rx, (y1 - cy1) / ry);
    let mut da = ang((-x1 - cx1) / rx, (-y1 - cy1) / ry) - a1;
    if sweep && da < 0.0 {
        da += TAU;
    } else if !sweep && da > 0.0 {
        da -= TAU;
    }
    let n = ((da.abs() / TAU * 4.0 * CURVE_STEPS as f64).ceil() as usize).max(2);
    for i in 1..=n {
        let a = a1 + da * i as f64 / n as f64;
        let (x, y) = (rx * a.cos(), ry * a.sin());
        out.push([c * x - s * y + cx, s * x + c * y + cy]);
    }
}

// --- PNG ----------------------------------------------------------------------

fn be32(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

/// Pixels d'encre d'un PNG : (largeur, hauteur, encre par pixel).
fn png_ink(b: &[u8], threshold: f64) -> Result<(usize, usize, Vec<bool>), Err> {
    let bad = |d: &str| err("image.invalid", d);
    if !b.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(bad("signature PNG"));
    }
    let (mut w, mut h, mut depth, mut ctype, mut interlace) = (0u32, 0u32, 0u8, 0u8, 0u8);
    let (mut palette, mut trns, mut idat) = (Vec::new(), Vec::new(), Vec::new());
    let mut i = 8;
    while i + 8 <= b.len() {
        let len = be32(b, i).ok_or_else(|| bad("bloc"))? as usize;
        let kind = &b[i + 4..i + 8];
        let data = b
            .get(i + 8..i + 8 + len)
            .ok_or_else(|| bad("bloc tronqué"))?;
        match kind {
            b"IHDR" if len >= 13 => {
                w = be32(data, 0).unwrap_or(0);
                h = be32(data, 4).unwrap_or(0);
                depth = data[8];
                ctype = data[9];
                interlace = data[12];
            }
            b"PLTE" => palette = data.to_vec(),
            b"tRNS" => trns = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        i += 12 + len;
    }
    if w == 0 || h == 0 {
        return Err(bad("en-tête"));
    }
    if w > MAX_SIDE || h > MAX_SIDE {
        return Err(err("image.too_large", format!("{w} × {h} px")));
    }
    let channels = match ctype {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => return Err(err("image.unsupported", format!("type {ctype}"))),
    };
    if depth != 8 || interlace != 0 {
        return Err(err(
            "image.unsupported",
            format!("{depth} bits, entrelacement {interlace}"),
        ));
    }
    let (w, h) = (w as usize, h as usize);
    let stride = w * channels;
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, (stride + 1) * h)
        .map_err(|_| bad("données compressées"))?;
    if raw.len() < (stride + 1) * h {
        return Err(bad("données tronquées"));
    }
    // Filtres de ligne.
    let mut px = vec![0u8; stride * h];
    for y in 0..h {
        let f = raw[y * (stride + 1)];
        let src = &raw[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        for x in 0..stride {
            let a = if x >= channels {
                px[y * stride + x - channels]
            } else {
                0
            };
            let up = if y > 0 { px[(y - 1) * stride + x] } else { 0 };
            let ul = if y > 0 && x >= channels {
                px[(y - 1) * stride + x - channels]
            } else {
                0
            };
            let pred = match f {
                0 => 0,
                1 => a,
                2 => up,
                3 => ((u16::from(a) + u16::from(up)) / 2) as u8,
                4 => {
                    let p = i16::from(a) + i16::from(up) - i16::from(ul);
                    let (pa, pb, pc) = (
                        (p - i16::from(a)).abs(),
                        (p - i16::from(up)).abs(),
                        (p - i16::from(ul)).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        up
                    } else {
                        ul
                    }
                }
                _ => return Err(bad("filtre")),
            };
            px[y * stride + x] = src[x].wrapping_add(pred);
        }
    }
    // (luminance 0–255, opacité 0–255) par pixel.
    let rgba: Vec<(f64, u8)> = (0..w * h)
        .map(|k| {
            let q = &px[k * channels..(k + 1) * channels];
            let lum = |r: u8, g: u8, b: u8| {
                0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b)
            };
            match ctype {
                0 => (f64::from(q[0]), 255),
                4 => (f64::from(q[0]), q[1]),
                2 => (lum(q[0], q[1], q[2]), 255),
                6 => (lum(q[0], q[1], q[2]), q[3]),
                _ => {
                    let i = usize::from(q[0]);
                    let c = palette.get(3 * i..3 * i + 3).unwrap_or(&[0, 0, 0]);
                    (lum(c[0], c[1], c[2]), trns.get(i).copied().unwrap_or(255))
                }
            }
        })
        .collect();
    let transparent = rgba.iter().any(|p| p.1 < 128);
    let ink = rgba
        .iter()
        .map(|&(l, a)| if transparent { a >= 128 } else { l < threshold })
        .collect();
    Ok((w, h, ink))
}

/// Contours des zones d'encre, le long des bords des pixels, simplifiés.
fn png_contours(b: &[u8], threshold: f64) -> Result<Vec<Vec<[f64; 2]>>, Err> {
    let (w, h, ink) = png_ink(b, threshold)?;
    let at = |x: isize, y: isize| {
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && ink[y as usize * w + x as usize]
    };
    // Arêtes orientées (encre à gauche), de coin à coin : départ → arrivée.
    let mut next: std::collections::BTreeMap<(isize, isize), Vec<(isize, isize)>> =
        Default::default();
    for y in 0..h as isize {
        for x in 0..w as isize {
            if !at(x, y) {
                continue;
            }
            // Coins (x, y) en haut à gauche ; y vers le bas.
            if !at(x, y - 1) {
                next.entry((x + 1, y)).or_default().push((x, y));
            }
            if !at(x, y + 1) {
                next.entry((x, y + 1)).or_default().push((x + 1, y + 1));
            }
            if !at(x - 1, y) {
                next.entry((x, y)).or_default().push((x, y + 1));
            }
            if !at(x + 1, y) {
                next.entry((x + 1, y + 1)).or_default().push((x + 1, y));
            }
        }
    }
    let mut out = Vec::new();
    while let Some((&first, _)) = next.iter().find(|(_, v)| !v.is_empty()) {
        let mut loop_pts = vec![first];
        let mut cur = first;
        loop {
            let Some(v) = next.get_mut(&cur) else { break };
            let Some(n) = v.pop() else { break };
            if n == first {
                break;
            }
            loop_pts.push(n);
            cur = n;
            if loop_pts.len() > MAX_POINTS {
                return Err(err("image.too_large", "points"));
            }
        }
        next.retain(|_, v| !v.is_empty());
        // Milieux des arêtes des pixels : plus près du vrai bord que les coins.
        let n = loop_pts.len();
        let pts: Vec<[f64; 3]> = (0..n)
            .map(|k| {
                let (a, b) = (loop_pts[k], loop_pts[(k + 1) % n]);
                [(a.0 + b.0) as f64 / 2.0, -((a.1 + b.1) as f64) / 2.0, 0.0]
            })
            .collect();
        let mut closed = pts.clone();
        closed.push(pts[0]);
        let simple = crate::points::simplify(&closed, PNG_SIMPLIFY);
        out.push(
            simple[..simple.len().saturating_sub(1)]
                .iter()
                .map(|p| [p[0], p[1]])
                .collect(),
        );
        if out.len() > MAX_CONTOURS {
            return Err(err("image.too_large", "contours"));
        }
    }
    Ok(out)
}

pub(crate) fn register(engine: &mut rhai::Engine) {
    use rhai::Dynamic;
    engine.register_type_with_name::<Drawing>("Drawing");
    engine.register_fn("contours", |d: &mut Drawing| -> rhai::Array {
        d.contours
            .iter()
            .map(|c| {
                let mut pts: Vec<[f64; 3]> = c.iter().map(|p| [p[0], p[1], 0.0]).collect();
                if let Some(&f) = pts.first() {
                    pts.push(f);
                }
                Dynamic::from(crate::points::Points(pts))
            })
            .collect()
    });
    engine.register_get("width", |d: &mut Drawing| d.width);
    engine.register_get("height", |d: &mut Drawing| d.height);
    engine.register_get("empty", |d: &mut Drawing| d.contours.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::base64;

    const SQUARE_HOLE: &str = r#"<?xml version="1.0"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
  <!-- carré percé -->
  <defs><rect x="0" y="0" width="5" height="5"/></defs>
  <g transform="translate(10 10)">
    <path d="M0,0 H80 V80 H0 Z M40 20 A20 20 0 1 0 40 60 A20 20 0 1 0 40 20 Z"/>
  </g>
</svg>"#;

    #[test]
    fn svg_square_with_a_round_hole() {
        let d = read(SQUARE_HOLE, 40.0, 128.0).unwrap();
        assert_eq!(d.contours.len(), 2, "defs ignoré");
        assert!((d.width - 40.0).abs() < 1e-6 && (d.height - 40.0).abs() < 1e-6);
        // Trou : rayon 20 sur 80 → 10 mm sur 40, centré.
        let hole = &d.contours[1];
        let r: Vec<f64> = hole.iter().map(|p| p[0].hypot(p[1])).collect();
        assert!(r.iter().all(|r| (r - 10.0).abs() < 0.05), "{r:?}");
    }

    #[test]
    fn svg_numbers_and_transforms() {
        assert_eq!(numbers("1-2.5.5e1,3"), vec![1.0, -2.5, 5.0, 3.0]);
        let m = transform("translate(10,5) scale(2)");
        assert_eq!(apply(&m, [1.0, 1.0]), [12.0, 7.0]);
        let r = transform("rotate(90)");
        let p = apply(&r, [1.0, 0.0]);
        assert!((p[0]).abs() < 1e-12 && (p[1] - 1.0).abs() < 1e-12);
    }

    /// PNG 8 bits en niveaux de gris, filtre 0, d'après une fonction d'encre.
    fn png(w: usize, h: usize, ink: impl Fn(usize, usize) -> bool) -> Vec<u8> {
        let mut raw = Vec::new();
        for y in 0..h {
            raw.push(0);
            for x in 0..w {
                raw.push(if ink(x, y) { 0 } else { 255 });
            }
        }
        let z = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6);
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut chunk = |kind: &[u8], data: &[u8]| {
            out.extend((data.len() as u32).to_be_bytes());
            out.extend(kind);
            out.extend(data);
            out.extend([0, 0, 0, 0]); // CRC non contrôlé à la lecture
        };
        let mut ihdr = Vec::new();
        ihdr.extend((w as u32).to_be_bytes());
        ihdr.extend((h as u32).to_be_bytes());
        ihdr.extend([8, 0, 0, 0, 0]);
        chunk(b"IHDR", &ihdr);
        chunk(b"IDAT", &z);
        chunk(b"IEND", &[]);
        out
    }

    #[test]
    fn png_square_with_a_hole_matches_the_svg() {
        // Carré de 160 px percé d'un disque de rayon 40, dans 200 × 200 : à
        // 40 mm, un pixel fait 0,25 mm ; le contour suit le dessin à 0,2 mm.
        let img = png(200, 200, |x, y| {
            let (fx, fy) = (x as f64 + 0.5, y as f64 + 0.5);
            (20.0..180.0).contains(&fx)
                && (20.0..180.0).contains(&fy)
                && (fx - 100.0).hypot(fy - 100.0) > 40.0
        });
        let d = read(
            &format!("data:image/png;base64,{}", base64(&img)),
            40.0,
            128.0,
        )
        .unwrap();
        assert_eq!(d.contours.len(), 2);
        assert!((d.width - 40.0).abs() < 1e-6);
        let hole = d
            .contours
            .iter()
            .min_by(|a, b| area(a).abs().total_cmp(&area(b).abs()))
            .unwrap();
        let r: Vec<f64> = hole.iter().map(|p| p[0].hypot(p[1])).collect();
        assert!(r.iter().all(|r| (r - 10.0).abs() < 0.2), "{r:?}");
    }

    #[test]
    fn bad_files_are_refused_with_a_code() {
        let code = |v: &str| read(v, 40.0, 128.0).unwrap_err().0;
        assert_eq!(code("bonjour"), "image.invalid");
        assert_eq!(code("<svg></svg>"), "image.empty");
        assert_eq!(code("<svg><path d=\"M0 0 X 5\"/></svg>"), "image.invalid");
        assert_eq!(code("data:image/png;base64,AAAA"), "image.invalid");
        let blank = png(10, 10, |_, _| false);
        assert_eq!(
            code(&format!("data:image/png;base64,{}", base64(&blank))),
            "image.empty"
        );
        let mut cut = png(20, 20, |x, _| x < 10);
        cut.truncate(cut.len() - 30);
        assert!(read(
            &format!("data:image/png;base64,{}", base64(&cut)),
            40.0,
            128.0
        )
        .is_err());
        assert_eq!(read("", 40.0, 128.0).unwrap(), Drawing::default());
    }
}
