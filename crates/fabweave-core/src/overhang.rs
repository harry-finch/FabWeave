//! Compensation du débit des parois inclinées (R-024, `filament.overhang_flow`).
//!
//! Quand une paroi penche de θ depuis la verticale, le boudin posé sur le
//! précédent n'en recouvre plus qu'une partie : vue perpendiculairement à la
//! paroi, la couche s'amincit (épaisseur × cos θ) et la paroi se creuse. Le
//! débit de chaque point est multiplié par `1 + c · (1 / cos θ − 1)`, `c`
//! étant le réglage du filament (0 : aucune compensation, 1 : section gardée).
//!
//! L'inclinaison se lit sur le parcours lui-même : pour chaque point imprimé,
//! le segment **parallèle** le plus proche de la couche du dessous (entre 0,4
//! et 1,6 hauteur de boudin plus bas) donne le décalage horizontal `d`, et
//! θ = atan(d / écart en hauteur). Les remplissages croisés (segments non
//! parallèles) et les porte-à-faux au-delà de 60° (aucun appui à portée, déjà
//! signalés par les contrôles de qualité) ne sont pas compensés. Le plafond
//! du débit volumétrique reste celui du filament : le planificateur ralentit
//! et le compte, comme ailleurs.

use std::collections::HashMap;

use crate::path::{MoveKind, Toolpath};

/// Inclinaison au-delà de laquelle on ne compense plus (degrés).
const MAX_ANGLE: f64 = 60.0;
/// Inclinaison sous laquelle rien ne change (degrés).
const MIN_ANGLE: f64 = 3.0;
/// Segments parallèles : écart d'orientation au plus (degrés).
const PARALLEL: f64 = 30.0;
/// Côté des cases de la grille de recherche (mm).
const CELL: f64 = 2.0;

/// Ce que la compensation a changé : points compensés, plus grand facteur.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Compensated {
    pub count: usize,
    pub max_factor: f64,
}

fn key(x: f64, y: f64) -> (i64, i64) {
    ((x / CELL).floor() as i64, (y / CELL).floor() as i64)
}

/// Distance horizontale de (x, y) au segment (a, b).
fn distance(x: f64, y: f64, a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((x - a[0]) * dx + (y - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (x - a[0] - t * dx).hypot(y - a[1] - t * dy)
}

/// Multiplie le débit des points des parois inclinées ; `amount` de 0 à 1.
pub fn compensate(path: &mut Toolpath, amount: f64) -> Compensated {
    let mut out = Compensated::default();
    if amount <= 0.0 {
        return out;
    }
    let pts = &mut path.points;
    let tan_max = MAX_ANGLE.to_radians().tan();
    let cos_parallel = PARALLEL.to_radians().cos();
    // Segments imprimés déjà posés, par case de leur milieu.
    let mut grid: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for i in 1..pts.len() {
        let (p, q) = (pts[i - 1], pts[i]);
        if q.kind != MoveKind::Print {
            continue;
        }
        let (dx, dy) = (q.x - p.x, q.y - p.y);
        let len = dx.hypot(dy);
        if len > 1e-9 && q.z > 1.6 * q.h {
            let reach = q.h * 1.6 * tan_max;
            let (lo, hi) = (key(q.x - reach, q.y - reach), key(q.x + reach, q.y + reach));
            let mut best: Option<(f64, f64)> = None;
            for cx in lo.0..=hi.0 {
                for cy in lo.1..=hi.1 {
                    for &k in grid.get(&(cx, cy)).into_iter().flatten() {
                        let (a, b) = (&pts[k - 1], &pts[k]);
                        let dz = q.z - 0.5 * (a.z + b.z);
                        if dz < 0.4 * q.h || dz > 1.6 * q.h {
                            continue;
                        }
                        let (ex, ey) = (b.x - a.x, b.y - a.y);
                        let el = ex.hypot(ey);
                        if el < 1e-9 || (dx * ex + dy * ey).abs() / (len * el) < cos_parallel {
                            continue;
                        }
                        let d = distance(q.x, q.y, [a.x, a.y], [b.x, b.y]);
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, dz));
                        }
                    }
                }
            }
            if let Some((d, dz)) = best {
                let theta = (d / dz).atan();
                if theta > MIN_ANGLE.to_radians() && d / dz <= tan_max {
                    let factor = 1.0 + amount * (1.0 / theta.cos() - 1.0);
                    pts[i].flow *= factor;
                    out.count += 1;
                    out.max_factor = out.max_factor.max(factor);
                }
            }
        }
        let mid = key(0.5 * (p.x + q.x), 0.5 * (p.y + q.y));
        grid.entry(mid).or_default().push(i);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{Speed, WayPoint};

    /// Cône dont la paroi penche de `lean` degrés : `turns` tours de 360
    /// points, hauteur de boudin 0,2.
    fn cone(lean: f64, turns: usize) -> Toolpath {
        let mut path = Toolpath::default();
        let t = lean.to_radians().tan();
        for i in 0..=turns * 360 {
            let a = i as f64 / 360.0 * std::f64::consts::TAU;
            let z = 0.2 + 0.2 * i as f64 / 360.0;
            let r = 20.0 + t * z;
            path.points.push(WayPoint {
                x: r * a.cos(),
                y: r * a.sin(),
                z,
                w: 0.6,
                h: 0.2,
                kind: if i == 0 {
                    MoveKind::Travel
                } else {
                    MoveKind::Print
                },
                speed: Speed::Factor(1.0),
                fan: None,
                flow: 1.0,
                volume: 0.0,
                duration: 0.0,
                temp_offset: 0.0,
                retraction: None,
            });
        }
        path
    }

    #[test]
    fn a_45_degree_wall_gets_the_expected_factor() {
        let mut p = cone(45.0, 6);
        let c = compensate(&mut p, 1.0);
        let expected = 1.0 / 45f64.to_radians().cos();
        let top = p.points.last().unwrap().flow;
        assert!((top - expected).abs() < 0.01, "{top} / {expected}");
        assert!(c.count > 4 * 360);
        let mut half = cone(45.0, 6);
        compensate(&mut half, 0.5);
        assert!((half.points.last().unwrap().flow - (1.0 + 0.5 * (expected - 1.0))).abs() < 0.01);
    }

    #[test]
    fn vertical_walls_and_zero_amount_change_nothing() {
        let mut p = cone(0.0, 4);
        assert_eq!(compensate(&mut p, 1.0).count, 0);
        let mut q = cone(45.0, 4);
        assert_eq!(compensate(&mut q, 0.0), Compensated::default());
        assert!(q.points.iter().all(|w| w.flow == 1.0));
    }

    #[test]
    fn beyond_60_degrees_nothing_is_compensated() {
        let mut p = cone(70.0, 4);
        assert_eq!(compensate(&mut p, 1.0).count, 0);
    }
}
