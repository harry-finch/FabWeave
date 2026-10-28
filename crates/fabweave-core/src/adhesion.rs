//! Adhésion au plateau générée par le moteur (ENG-18) : bordure (*brim*) et
//! radeau (*raft*), au choix de l'utilisateur et non du modèle.
//!
//! Les deux partent de l'**empreinte de la première couche** : les boudins
//! posés sur le plateau (sous Z min + deux hauteurs de couche, pour qu'un
//! vase en spirale donne son premier tour entier) sont tracés dans une grille (cellules de 0,1 mm, ou
//! plus grandes pour une très grande pièce : 2 millions de cellules au plus),
//! les trous de la pièce bouchés (la bordure ne court qu'à
//! l'extérieur, le radeau est plein), puis chaque cellule reçoit sa distance
//! au bord de la pièce (transformée de distance euclidienne exacte,
//! Felzenszwalb et Huttenlocher). Les contours à distance donnée sont tirés
//! de cette carte par la méthode des carrés marchants, puis simplifiés
//! (Douglas–Peucker).
//!
//! - Bordure : boucles à `gap + (k + ½) × w` du bord, de la plus éloignée à
//!   la plus proche (la dernière finit contre la pièce), à la hauteur de la
//!   première couche.
//! - Radeau : contour à `margin` du bord, rempli en lignes (`fill_lines`) :
//!   couche de base pleine, couches intermédiaires espacées, couche de
//!   contact pleine ; la pièce est rehaussée de son épaisseur plus l'écart
//!   de décollage.
//!
//! Pur et déterministe.

use crate::fill::fill_lines;
use crate::path::{MoveKind, Speed, Toolpath, WayPoint};

/// Côté maximal d'une cellule (mm).
const CELL_MAX: f64 = 0.1;
/// Cellules visées au plus dans la grille (mémoire du WebAssembly).
const CELLS_TARGET: f64 = 2.0e6;
/// Écart toléré à la simplification d'un contour (mm).
const SIMPLIFY: f64 = 0.01;
/// Espacement des lignes des couches intermédiaires du radeau (mm).
const RAFT_SPARSE: f64 = 2.0;

/// Segment imprimé de la première couche : extrémités et largeur (mm).
pub type Bead = ([f64; 2], [f64; 2], f64);

/// Carte des distances au bord de l'empreinte (mm, négatives dedans).
struct Field {
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    dist: Vec<f64>,
}

impl Field {
    /// Empreinte des boudins, trous bouchés, avec une marge libre de `reach`
    /// mm autour. `None` si la grille serait trop grande.
    fn new(beads: &[Bead], reach: f64) -> Option<Field> {
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for (a, b, w) in beads {
            for p in [a, b] {
                for k in 0..2 {
                    lo[k] = lo[k].min(p[k] - w / 2.0);
                    hi[k] = hi[k].max(p[k] + w / 2.0);
                }
            }
        }
        if !lo[0].is_finite() {
            return None;
        }
        let w_min = beads.iter().map(|b| b.2).fold(f64::INFINITY, f64::min);
        let pad0 = reach + 1.0;
        let area = (hi[0] - lo[0] + 2.0 * pad0) * (hi[1] - lo[1] + 2.0 * pad0);
        let cell = CELL_MAX.min(w_min / 3.0).max((area / CELLS_TARGET).sqrt());
        let pad = reach + 3.0 * cell;
        let (x0, y0) = (lo[0] - pad, lo[1] - pad);
        let nx = ((hi[0] - lo[0] + 2.0 * pad) / cell).ceil() as usize + 1;
        let ny = ((hi[1] - lo[1] + 2.0 * pad) / cell).ceil() as usize + 1;
        if !(nx > 2 && ny > 2) || nx.saturating_mul(ny) as f64 > 4.0 * CELLS_TARGET {
            return None;
        }
        let mut solid = vec![false; nx * ny];
        // Boudin : disques de rayon w/2 le long du segment, tous les ½ cellule.
        for (a, b, w) in beads {
            let r = w / 2.0;
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            let steps = (len / (cell / 2.0)).ceil().max(1.0) as usize;
            let rc = (r / cell).ceil() as isize;
            for s in 0..=steps {
                let t = s as f64 / steps as f64;
                let (px, py) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                let (ci, cj) = (
                    ((px - x0) / cell).round() as isize,
                    ((py - y0) / cell).round() as isize,
                );
                for dj in -rc..=rc {
                    for di in -rc..=rc {
                        let (i, j) = (ci + di, cj + dj);
                        if i < 0 || j < 0 || i >= nx as isize || j >= ny as isize {
                            continue;
                        }
                        let (cx, cy) = (x0 + i as f64 * cell, y0 + j as f64 * cell);
                        if (cx - px).hypot(cy - py) <= r {
                            solid[j as usize * nx + i as usize] = true;
                        }
                    }
                }
            }
        }
        // Trous bouchés : est extérieur ce que l'on atteint depuis le bord.
        let mut outside = vec![false; nx * ny];
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..nx {
            stack.push(i);
            stack.push((ny - 1) * nx + i);
        }
        for j in 0..ny {
            stack.push(j * nx);
            stack.push(j * nx + nx - 1);
        }
        while let Some(c) = stack.pop() {
            if outside[c] || solid[c] {
                continue;
            }
            outside[c] = true;
            let (i, j) = (c % nx, c / nx);
            if i > 0 {
                stack.push(c - 1);
            }
            if i + 1 < nx {
                stack.push(c + 1);
            }
            if j > 0 {
                stack.push(c - nx);
            }
            if j + 1 < ny {
                stack.push(c + nx);
            }
        }
        let inside: Vec<bool> = outside.iter().map(|o| !o).collect();
        let d_out = edt(&inside, nx, ny);
        let dist = d_out
            .iter()
            .zip(&inside)
            .map(|(d2, &is_in)| {
                if is_in {
                    -cell / 2.0
                } else {
                    // Distance au centre de la cellule pleine la plus proche,
                    // moins une demi-cellule : bord de la pièce.
                    d2.sqrt() * cell - cell / 2.0
                }
            })
            .collect();
        Some(Field {
            x0,
            y0,
            cell,
            nx,
            ny,
            dist,
        })
    }

    /// Contours fermés à la distance `level` du bord (carrés marchants).
    fn contours(&self, level: f64) -> Vec<Vec<[f64; 2]>> {
        let (nx, ny) = (self.nx, self.ny);
        let v = |i: usize, j: usize| self.dist[j * nx + i];
        let pos = |i: usize, j: usize| {
            [
                self.x0 + i as f64 * self.cell,
                self.y0 + j as f64 * self.cell,
            ]
        };
        // Point d'une arête : (i, j, verticale ?) → position interpolée.
        let point = |i: usize, j: usize, vertical: bool| {
            let (a, b) = if vertical {
                ((i, j), (i, j + 1))
            } else {
                ((i, j), (i + 1, j))
            };
            let (va, vb) = (v(a.0, a.1), v(b.0, b.1));
            let t = ((level - va) / (vb - va)).clamp(0.0, 1.0);
            let (pa, pb) = (pos(a.0, a.1), pos(b.0, b.1));
            [pa[0] + (pb[0] - pa[0]) * t, pa[1] + (pb[1] - pa[1]) * t]
        };
        let edge_id =
            |i: usize, j: usize, vertical: bool| (2 * (j * nx + i) + usize::from(vertical)) as u64;
        let mut segs: Vec<[(u64, [f64; 2]); 2]> = Vec::new();
        for j in 0..ny - 1 {
            for i in 0..nx - 1 {
                let c = [v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)];
                let case = c
                    .iter()
                    .enumerate()
                    .fold(0u8, |acc, (k, x)| acc | (u8::from(*x < level) << k));
                if case == 0 || case == 15 {
                    continue;
                }
                // Arêtes : bas, droite, haut, gauche.
                let e = [
                    (i, j, false),
                    (i + 1, j, true),
                    (i, j + 1, false),
                    (i, j, true),
                ];
                let centre_in = (c[0] + c[1] + c[2] + c[3]) / 4.0 < level;
                let pairs: &[(usize, usize)] = match case {
                    1 | 14 => &[(3, 0)],
                    2 | 13 => &[(0, 1)],
                    3 | 12 => &[(3, 1)],
                    4 | 11 => &[(1, 2)],
                    6 | 9 => &[(0, 2)],
                    7 | 8 => &[(3, 2)],
                    5 => {
                        if centre_in {
                            &[(0, 1), (3, 2)]
                        } else {
                            &[(3, 0), (1, 2)]
                        }
                    }
                    _ => {
                        // 10
                        if centre_in {
                            &[(3, 0), (1, 2)]
                        } else {
                            &[(0, 1), (3, 2)]
                        }
                    }
                };
                for &(a, b) in pairs {
                    let (ea, eb) = (e[a], e[b]);
                    segs.push([
                        (edge_id(ea.0, ea.1, ea.2), point(ea.0, ea.1, ea.2)),
                        (edge_id(eb.0, eb.1, eb.2), point(eb.0, eb.1, eb.2)),
                    ]);
                }
            }
        }
        // Chaînage : chaque arête traversée appartient à deux segments.
        let mut by_edge: std::collections::HashMap<u64, Vec<usize>> =
            std::collections::HashMap::new();
        for (k, s) in segs.iter().enumerate() {
            by_edge.entry(s[0].0).or_default().push(k);
            by_edge.entry(s[1].0).or_default().push(k);
        }
        let mut used = vec![false; segs.len()];
        let mut loops = Vec::new();
        for start in 0..segs.len() {
            if used[start] {
                continue;
            }
            used[start] = true;
            let mut lp = vec![segs[start][0].1];
            let first_edge = segs[start][0].0;
            let (mut edge, mut p) = (segs[start][1].0, segs[start][1].1);
            loop {
                if edge == first_edge {
                    break;
                }
                lp.push(p);
                let next = by_edge[&edge].iter().copied().find(|&k| !used[k]);
                let Some(k) = next else { break };
                used[k] = true;
                let s = segs[k];
                (edge, p) = if s[0].0 == edge {
                    (s[1].0, s[1].1)
                } else {
                    (s[0].0, s[0].1)
                };
            }
            if lp.len() >= 3 {
                loops.push(simplify_closed(&lp, SIMPLIFY));
            }
        }
        loops
    }
}

/// Carré de la distance (en cellules) de chaque cellule à la cellule pleine
/// la plus proche ; 0 pour une cellule pleine.
fn edt(solid: &[bool], nx: usize, ny: usize) -> Vec<f64> {
    const INF: f64 = 1e20;
    let mut g: Vec<f64> = solid.iter().map(|&s| if s { 0.0 } else { INF }).collect();
    let mut f = vec![0.0; nx.max(ny)];
    let mut d = vec![0.0; nx.max(ny)];
    for i in 0..nx {
        for j in 0..ny {
            f[j] = g[j * nx + i];
        }
        edt_1d(&f[..ny], &mut d[..ny]);
        for j in 0..ny {
            g[j * nx + i] = d[j];
        }
    }
    for j in 0..ny {
        f[..nx].copy_from_slice(&g[j * nx..(j + 1) * nx]);
        edt_1d(&f[..nx], &mut d[..nx]);
        g[j * nx..(j + 1) * nx].copy_from_slice(&d[..nx]);
    }
    g
}

/// Transformée 1D : enveloppe inférieure des paraboles (Felzenszwalb).
fn edt_1d(f: &[f64], d: &mut [f64]) {
    let n = f.len();
    let mut v = vec![0usize; n];
    let mut z = vec![0.0f64; n + 1];
    let mut k = 0;
    z[0] = f64::NEG_INFINITY;
    z[1] = f64::INFINITY;
    for q in 1..n {
        loop {
            let p = v[k];
            let s =
                ((f[q] + (q * q) as f64) - (f[p] + (p * p) as f64)) / (2.0 * (q as f64 - p as f64));
            if s <= z[k] && k > 0 {
                k -= 1;
                continue;
            }
            if s <= z[k] {
                // k == 0 : la parabole q remplace la première.
                v[0] = q;
                z[1] = f64::INFINITY;
                break;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = f64::INFINITY;
            break;
        }
    }
    k = 0;
    for (q, out) in d.iter_mut().enumerate() {
        while z[k + 1] < q as f64 {
            k += 1;
        }
        let p = v[k];
        *out = (q as f64 - p as f64).powi(2) + f[p];
    }
}

/// Douglas–Peucker sur une boucle fermée (coupée en ses deux points les plus
/// éloignés du premier).
fn simplify_closed(lp: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    let far = (1..lp.len())
        .max_by(|&a, &b| {
            let da = (lp[a][0] - lp[0][0]).hypot(lp[a][1] - lp[0][1]);
            let db = (lp[b][0] - lp[0][0]).hypot(lp[b][1] - lp[0][1]);
            da.total_cmp(&db)
        })
        .unwrap_or(0);
    let mut a = simplify(&lp[..=far], tol);
    let mut back: Vec<[f64; 2]> = lp[far..].to_vec();
    back.push(lp[0]);
    let b = simplify(&back, tol);
    a.pop();
    a.extend_from_slice(&b[..b.len() - 1]);
    a
}

fn simplify(pts: &[[f64; 2]], tol: f64) -> Vec<[f64; 2]> {
    if pts.len() <= 2 {
        return pts.to_vec();
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let len = dx.hypot(dy);
    let (mut worst, mut at) = (0.0, 0);
    for (k, p) in pts.iter().enumerate().take(pts.len() - 1).skip(1) {
        let d = if len < 1e-12 {
            (p[0] - a[0]).hypot(p[1] - a[1])
        } else {
            ((p[0] - a[0]) * dy - (p[1] - a[1]) * dx).abs() / len
        };
        if d > worst {
            worst = d;
            at = k;
        }
    }
    if worst <= tol {
        return vec![a, b];
    }
    let mut left = simplify(&pts[..=at], tol);
    let right = simplify(&pts[at..], tol);
    left.pop();
    left.extend(right);
    left
}

/// Boucle tournée pour commencer au point le plus proche de `from`.
fn start_near(lp: &[[f64; 2]], from: [f64; 2]) -> Vec<[f64; 2]> {
    let k = (0..lp.len())
        .min_by(|&a, &b| {
            let da = (lp[a][0] - from[0]).hypot(lp[a][1] - from[1]);
            let db = (lp[b][0] - from[0]).hypot(lp[b][1] - from[1]);
            da.total_cmp(&db)
        })
        .unwrap_or(0);
    let mut out = lp[k..].to_vec();
    out.extend_from_slice(&lp[..k]);
    out
}

fn point(p: [f64; 2], z: f64, w: f64, h: f64, kind: MoveKind) -> WayPoint {
    WayPoint {
        x: p[0],
        y: p[1],
        z,
        w,
        h,
        kind,
        speed: Speed::Factor(1.0),
        fan: None,
        flow: 1.0,
        volume: 0.0,
        duration: 0.0,
    }
}

/// Boucles de la bordure, de la plus éloignée à la plus proche de la pièce,
/// chacune fermée, à la hauteur `z` (dessus du boudin). `None` si la grille
/// serait trop grande.
pub fn brim(beads: &[Bead], width: f64, gap: f64, w: f64, z: f64) -> Option<Vec<WayPoint>> {
    let loops_n = ((width / w).round() as usize).max(1);
    let reach = gap + loops_n as f64 * w;
    let field = Field::new(beads, reach)?;
    let mut out: Vec<WayPoint> = Vec::new();
    let mut here = [field.x0, field.y0];
    for k in (0..loops_n).rev() {
        let level = gap + (k as f64 + 0.5) * w;
        let mut loops = field.contours(level);
        while !loops.is_empty() {
            // Boucle la plus proche de la position courante.
            let near = (0..loops.len())
                .min_by(|&a, &b| {
                    let d = |l: &Vec<[f64; 2]>| {
                        l.iter()
                            .map(|p| (p[0] - here[0]).hypot(p[1] - here[1]))
                            .fold(f64::INFINITY, f64::min)
                    };
                    d(&loops[a]).total_cmp(&d(&loops[b]))
                })
                .unwrap_or(0);
            let lp = start_near(&loops.remove(near), here);
            out.push(point(lp[0], z, w, z, MoveKind::Travel));
            for p in lp.iter().skip(1).chain(std::iter::once(&lp[0])) {
                out.push(point(*p, z, w, z, MoveKind::Print));
            }
            here = lp[0];
        }
    }
    Some(out)
}

/// Couches du radeau et son épaisseur (dessus de la couche de contact).
/// `heights` : hauteur de chaque couche, de la base au contact.
pub fn raft(beads: &[Bead], margin: f64, w: f64, heights: &[f64]) -> Option<(Vec<WayPoint>, f64)> {
    let field = Field::new(beads, margin)?;
    let outlines = field.contours(margin);
    let mut out: Vec<WayPoint> = Vec::new();
    let mut z = 0.0;
    let n = heights.len();
    for (k, h) in heights.iter().enumerate() {
        z += h;
        // Base et contact pleins, intermédiaires espacés ; sens alterné.
        let spacing = if k == 0 || k + 1 == n { w } else { RAFT_SPARSE };
        let angle = if k % 2 == 0 {
            0.0
        } else {
            std::f64::consts::FRAC_PI_2
        };
        for lp in &outlines {
            let contour: Vec<[f64; 3]> = lp.iter().map(|p| [p[0], p[1], z]).collect();
            for chain in fill_lines(&contour, spacing, angle, w / 2.0) {
                out.push(point(
                    [chain[0][0], chain[0][1]],
                    z,
                    w,
                    *h,
                    MoveKind::Travel,
                ));
                for p in chain.iter().skip(1) {
                    out.push(point([p[0], p[1]], z, w, *h, MoveKind::Print));
                }
            }
        }
    }
    Some((out, z))
}

/// Segments imprimés de la première couche d'un parcours : mouvements dont
/// les deux bouts sont sous `top` (choisi par l'appelant).
pub fn first_layer_beads(path: &Toolpath, top: f64) -> Vec<Bead> {
    path.points
        .windows(2)
        .filter(|w| w[1].kind != MoveKind::Travel && w[0].z < top && w[1].z < top)
        .map(|w| ([w[0].x, w[0].y], [w[1].x, w[1].y], w[1].w))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ring(r: f64, n: usize) -> Vec<Bead> {
        (0..n)
            .map(|i| {
                let (a, b) = (
                    std::f64::consts::TAU * i as f64 / n as f64,
                    std::f64::consts::TAU * (i + 1) as f64 / n as f64,
                );
                ([r * a.cos(), r * a.sin()], [r * b.cos(), r * b.sin()], 0.45)
            })
            .collect()
    }

    #[test]
    fn edt_matches_brute_force() {
        let (nx, ny) = (13, 9);
        let solid: Vec<bool> = (0..nx * ny)
            .map(|c| c == 20 || c == 77 || c == 101)
            .collect();
        let d = edt(&solid, nx, ny);
        for (c, dc) in d.iter().enumerate() {
            let (i, j) = ((c % nx) as f64, (c / nx) as f64);
            let best = (0..nx * ny)
                .filter(|&s| solid[s])
                .map(|s| ((s % nx) as f64 - i).powi(2) + ((s / nx) as f64 - j).powi(2))
                .fold(f64::INFINITY, f64::min);
            assert_eq!(*dc, best, "cellule {c}");
        }
    }

    #[test]
    fn brim_loops_surround_a_ring_at_the_right_distance() {
        // Anneau de rayon 10 mm (boudin 0,45) : bord extérieur à 10,225 mm.
        let beads = ring(10.0, 120);
        let pts = brim(&beads, 2.0, 0.1, 0.45, 0.2).unwrap();
        let travels: Vec<usize> = (0..pts.len())
            .filter(|&i| pts[i].kind == MoveKind::Travel)
            .collect();
        // 2 mm / 0,45 → 4 boucles, extérieures seulement (le trou est bouché).
        assert_eq!(travels.len(), 4);
        for (n, &t) in travels.iter().enumerate() {
            let k = 3 - n; // de la plus éloignée à la plus proche
            let expected = 10.225 + 0.1 + (k as f64 + 0.5) * 0.45;
            let end = travels.get(n + 1).copied().unwrap_or(pts.len());
            for p in &pts[t..end] {
                let r = p.x.hypot(p.y);
                assert!(
                    (r - expected).abs() < 0.08,
                    "boucle {k} : r = {r}, attendu {expected}"
                );
            }
            // Boucle fermée.
            assert_eq!((pts[t].x, pts[t].y), (pts[end - 1].x, pts[end - 1].y));
        }
        assert!(pts.iter().all(|p| p.z == 0.2 && p.h == 0.2));
        assert_eq!(
            pts,
            brim(&beads, 2.0, 0.1, 0.45, 0.2).unwrap(),
            "déterministe"
        );
    }

    #[test]
    fn two_islands_get_their_own_brims_when_far_apart() {
        let mut beads = ring(5.0, 60);
        beads.extend(
            ring(5.0, 60)
                .into_iter()
                .map(|(a, b, w)| ([a[0] + 30.0, a[1]], [b[0] + 30.0, b[1]], w)),
        );
        let pts = brim(&beads, 0.45, 0.0, 0.45, 0.2).unwrap();
        assert_eq!(pts.iter().filter(|p| p.kind == MoveKind::Travel).count(), 2);
        // Proches : une seule boucle les entoure toutes deux.
        let mut close = ring(5.0, 60);
        close.extend(
            ring(5.0, 60)
                .into_iter()
                .map(|(a, b, w)| ([a[0] + 10.6, a[1]], [b[0] + 10.6, b[1]], w)),
        );
        let pts = brim(&close, 0.45, 0.0, 0.45, 0.2).unwrap();
        assert_eq!(pts.iter().filter(|p| p.kind == MoveKind::Travel).count(), 1);
    }

    #[test]
    fn raft_covers_the_footprint_plus_its_margin() {
        let beads = ring(10.0, 120);
        let (pts, top) = raft(&beads, 3.0, 0.5, &[0.3, 0.2, 0.2]).unwrap();
        assert!((top - 0.7).abs() < 1e-12);
        let zs: Vec<f64> = pts.iter().map(|p| p.z).collect();
        assert!(zs.windows(2).all(|w| w[0] <= w[1]), "couche après couche");
        // Lignes dans le disque de 10,225 + 3 mm, et jusqu'à près de son bord.
        let r_max = pts
            .iter()
            .filter(|p| p.kind == MoveKind::Print)
            .map(|p| p.x.hypot(p.y))
            .fold(0.0, f64::max);
        assert!(r_max < 13.25 && r_max > 12.5, "{r_max}");
        // Couche intermédiaire espacée de 2 mm : bien moins de lignes.
        let count = |z: f64| {
            pts.iter()
                .filter(|p| p.kind == MoveKind::Print && (p.z - z).abs() < 1e-9)
                .count()
        };
        assert!(
            count(0.5) * 2 < count(0.3),
            "{} / {}",
            count(0.5),
            count(0.3)
        );
    }

    #[test]
    fn simplification_keeps_corners() {
        let mut sq = Vec::new();
        for i in 0..40 {
            sq.push([i as f64 * 0.25, 0.0]);
        }
        for i in 0..40 {
            sq.push([10.0, i as f64 * 0.25]);
        }
        for i in 0..40 {
            sq.push([10.0 - i as f64 * 0.25, 10.0]);
        }
        for i in 0..40 {
            sq.push([0.0, 10.0 - i as f64 * 0.25]);
        }
        let s = simplify_closed(&sq, 0.01);
        assert_eq!(s.len(), 4, "{s:?}");
    }
}
