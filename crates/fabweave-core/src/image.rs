//! Vignettes du parcours, rendues par le moteur : paquet Bambu Lab
//! (`package.rs`) et vignettes intégrées au G-code (écrans Prusa, Klipper).
//! Encodeurs PNG et QOI, base64 ; pur et déterministe.

use crate::path::{MoveKind, Toolpath};
use crate::plan::Plan;
use crate::profile::{ImageFormat, ThumbnailSpec};

/// Vignette PNG `size` × `size` (paquet Bambu Lab).
pub fn thumbnail(path: &Toolpath, plan: &Plan, size: usize) -> Vec<u8> {
    png(&render(path, plan, size, size), size, size)
}

/// Vignette au format demandé par le profil.
pub fn encode(path: &Toolpath, plan: &Plan, spec: ThumbnailSpec) -> Vec<u8> {
    let rgba = render(path, plan, spec.width, spec.height);
    match spec.format {
        ImageFormat::Png => png(&rgba, spec.width, spec.height),
        ImageFormat::Qoi => qoi(&rgba, spec.width, spec.height),
    }
}

/// Parcours imprimé en RVBA `width` × `height`, fond transparent, en vue
/// oblique (comme les vignettes du catalogue), teinte claire dégradée selon
/// la hauteur ; traits semi-transparents, dont le recouvrement dessine le
/// relief. Centré, marge de 8 % du plus petit côté.
pub fn render(path: &Toolpath, plan: &Plan, width: usize, height: usize) -> Vec<u8> {
    let pts = &path.points;
    let proj = |p: &crate::path::WayPoint| {
        (
            p.x * 0.91 + p.y * 0.41,
            -p.z * 0.8 + (p.y * 0.91 - p.x * 0.41) * 0.45,
        )
    };
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in pts {
        let (u, v) = proj(p);
        for (k, c) in [u, v, p.z].into_iter().enumerate() {
            lo[k] = lo[k].min(c);
            hi[k] = hi[k].max(c);
        }
    }
    let mut img = Raster {
        rgba: vec![0u8; width * height * 4],
        width,
        height,
    };
    if pts.len() >= 2 && lo[0].is_finite() {
        let (w, h) = (width as f64, height as f64);
        let pad = 0.08 * w.min(h);
        let (su, sv) = ((hi[0] - lo[0]).max(1e-6), (hi[1] - lo[1]).max(1e-6));
        let k = ((w - 2.0 * pad) / su).min((h - 2.0 * pad) / sv);
        let ou = (w - su * k) / 2.0 - lo[0] * k;
        let ov = (h - sv * k) / 2.0 - lo[1] * k;
        let zspan = (hi[2] - lo[2]).max(1e-6);
        for (seg, m) in pts.windows(2).zip(&plan.moves[1..]) {
            let (a, b) = (&seg[0], &seg[1]);
            if !b.kind.deposits() || m.skipped {
                continue;
            }
            let (ua, va) = proj(a);
            let (ub, vb) = proj(b);
            let t = ((b.z - lo[2]) / zspan).clamp(0.0, 1.0);
            let shade = (125.0 + 117.0 * t) as u8; // gris moyen en bas, presque blanc en haut
            if b.kind == MoveKind::Blob {
                img.disc((ou + ub * k, ov + vb * k), (b.w * k / 2.0).max(0.7), shade);
                continue;
            }
            img.segment(
                (ou + ua * k, ov + va * k),
                (ou + ub * k, ov + vb * k),
                shade,
            );
        }
    }
    img.rgba
}

struct Raster {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
}

impl Raster {
    /// Segment en pas d'un pixel, mélangé à 45 % d'opacité.
    fn segment(&mut self, a: (f64, f64), b: (f64, f64), shade: u8) {
        let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as usize;
        for s in 0..=steps {
            let t = s as f64 / steps as f64;
            self.blend(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, shade);
        }
    }

    /// Disque plein (goutte), mélangé comme les segments.
    fn disc(&mut self, c: (f64, f64), r: f64, shade: u8) {
        let n = r.ceil() as i64;
        for dy in -n..=n {
            for dx in -n..=n {
                let (fx, fy) = (dx as f64, dy as f64);
                if fx * fx + fy * fy <= r * r {
                    self.blend(c.0 + fx, c.1 + fy, shade);
                }
            }
        }
    }

    /// Pixel mélangé à 45 % d'opacité (hors image : ignoré).
    fn blend(&mut self, x: f64, y: f64, shade: u8) {
        if x < 0.0 || y < 0.0 || x >= self.width as f64 || y >= self.height as f64 {
            return;
        }
        let p = ((y as usize) * self.width + x as usize) * 4;
        let rgba = &mut self.rgba;
        let alpha = 0.45;
        let old_a = f64::from(rgba[p + 3]) / 255.0;
        let new_a = alpha + old_a * (1.0 - alpha);
        for c in 0..3 {
            let old = f64::from(rgba[p + c]) * old_a * (1.0 - alpha);
            rgba[p + c] = ((f64::from(shade) * alpha + old) / new_a).round() as u8;
        }
        rgba[p + 3] = (new_a * 255.0).round() as u8;
    }
}

/// PNG RVBA 8 bits, sans filtre de lignes, compressé (zlib).
pub fn png(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height);
    for row in rgba.chunks(width * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend((width as u32).to_be_bytes());
    ihdr.extend((height as u32).to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(
        &mut out,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

/// CRC-32 (polynôme 0xEDB88320) du PNG et du ZIP.
pub(crate) fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// QOI (« Quite OK Image », qoiformat.org) RVBA : format des vignettes
/// lues par le firmware des Prusa, plus rapide à décoder que le PNG.
pub fn qoi(rgba: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(14 + rgba.len() / 2 + 8);
    out.extend_from_slice(b"qoif");
    out.extend((width as u32).to_be_bytes());
    out.extend((height as u32).to_be_bytes());
    out.extend([4, 0]); // RVBA, sRGB
    let mut index = [[0u8; 4]; 64];
    let mut prev = [0u8, 0, 0, 255];
    let mut run = 0u8;
    let pixels: Vec<[u8; 4]> = rgba
        .chunks_exact(4)
        .map(|c| [c[0], c[1], c[2], c[3]])
        .collect();
    for (i, &px) in pixels.iter().enumerate() {
        if px == prev {
            run += 1;
            if run == 62 || i + 1 == pixels.len() {
                out.push(0xc0 | (run - 1));
                run = 0;
            }
            continue;
        }
        if run > 0 {
            out.push(0xc0 | (run - 1));
            run = 0;
        }
        let h = (usize::from(px[0]) * 3
            + usize::from(px[1]) * 5
            + usize::from(px[2]) * 7
            + usize::from(px[3]) * 11)
            % 64;
        if index[h] == px {
            out.push(h as u8);
        } else {
            index[h] = px;
            if px[3] == prev[3] {
                let d = |k: usize| px[k].wrapping_sub(prev[k]) as i8;
                let (dr, dg, db) = (d(0), d(1), d(2));
                let (dr_dg, db_dg) = (dr.wrapping_sub(dg), db.wrapping_sub(dg));
                if (-2..=1).contains(&dr) && (-2..=1).contains(&dg) && (-2..=1).contains(&db) {
                    out.push(0x40 | ((dr + 2) as u8) << 4 | ((dg + 2) as u8) << 2 | (db + 2) as u8);
                } else if (-32..=31).contains(&dg)
                    && (-8..=7).contains(&dr_dg)
                    && (-8..=7).contains(&db_dg)
                {
                    out.push(0x80 | (dg + 32) as u8);
                    out.push(((dr_dg + 8) as u8) << 4 | (db_dg + 8) as u8);
                } else {
                    out.extend([0xfe, px[0], px[1], px[2]]);
                }
            } else {
                out.extend([0xff, px[0], px[1], px[2], px[3]]);
            }
        }
        prev = px;
    }
    out.extend([0, 0, 0, 0, 0, 0, 0, 1]);
    out
}

/// Base64 standard (RFC 4648), avec remplissage `=`.
pub fn base64(data: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = u32::from(c[0]) << 16
            | u32::from(*c.get(1).unwrap_or(&0)) << 8
            | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            if k <= c.len() {
                out.push(ABC[(n >> (18 - 6 * k) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Décodeur QOI de référence (spécification), pour l'aller-retour.
    fn unqoi(q: &[u8]) -> (usize, usize, Vec<u8>) {
        assert_eq!(&q[..4], b"qoif");
        let w = u32::from_be_bytes([q[4], q[5], q[6], q[7]]) as usize;
        let h = u32::from_be_bytes([q[8], q[9], q[10], q[11]]) as usize;
        assert_eq!(&q[q.len() - 8..], &[0, 0, 0, 0, 0, 0, 0, 1]);
        let mut index = [[0u8; 4]; 64];
        let mut px = [0u8, 0, 0, 255];
        let mut out = Vec::with_capacity(w * h * 4);
        let mut p = 14;
        while out.len() < w * h * 4 {
            let b = q[p];
            p += 1;
            let mut run = 1;
            match b {
                0xfe => {
                    px[..3].copy_from_slice(&q[p..p + 3]);
                    p += 3;
                }
                0xff => {
                    px.copy_from_slice(&q[p..p + 4]);
                    p += 4;
                }
                _ => match b >> 6 {
                    0 => px = index[b as usize],
                    1 => {
                        for (k, s) in [4, 2, 0].into_iter().enumerate() {
                            px[k] = px[k].wrapping_add((b >> s & 3).wrapping_sub(2));
                        }
                    }
                    2 => {
                        let dg = (b & 63).wrapping_sub(32);
                        let n = q[p];
                        p += 1;
                        px[0] = px[0].wrapping_add(dg.wrapping_add((n >> 4).wrapping_sub(8)));
                        px[1] = px[1].wrapping_add(dg);
                        px[2] = px[2].wrapping_add(dg.wrapping_add((n & 15).wrapping_sub(8)));
                    }
                    _ => run = usize::from(b & 63) + 1,
                },
            }
            let h = (usize::from(px[0]) * 3
                + usize::from(px[1]) * 5
                + usize::from(px[2]) * 7
                + usize::from(px[3]) * 11)
                % 64;
            index[h] = px;
            for _ in 0..run {
                out.extend(px);
            }
        }
        assert_eq!(p, q.len() - 8, "fin du flux");
        (w, h, out)
    }

    #[test]
    fn qoi_round_trips() {
        // Dégradés, aplats (séries), couleurs répétées (index), transparences.
        let (w, h) = (37, 23);
        let mut img = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = (x * 7 + y * 3) as u8;
                img.extend(match (x / 5 + y / 4) % 4 {
                    0 => [v, v.wrapping_add(1), v, 255],
                    1 => [0, 0, 0, 0],
                    2 => [200, 10, (x * 40) as u8, 128],
                    _ => [12, 250, 3, 255],
                });
            }
        }
        let q = qoi(&img, w, h);
        assert!(q.len() < img.len());
        assert_eq!(unqoi(&q), (w, h, img));
    }

    #[test]
    fn png_has_the_expected_header() {
        let p = png(&[255u8; 5 * 3 * 4], 5, 3);
        assert_eq!(&p[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&p[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([p[16], p[17], p[18], p[19]]), 5);
        assert_eq!(u32::from_be_bytes([p[20], p[21], p[22], p[23]]), 3);
        assert_eq!(&p[p.len() - 8..p.len() - 4], b"IEND");
    }

    #[test]
    fn crc32_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn base64_matches_rfc_4648() {
        for (i, o) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(i.as_bytes()), o);
        }
    }
}
