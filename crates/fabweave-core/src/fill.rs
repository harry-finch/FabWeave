//! Remplissage d'un contour fermé par des lignes parallèles (API-13) : fonds
//! pleins, couches de dessus, grilles. Calcul natif, déterministe.
//!
//! Le contour est un polygone quelconque dans le plan XY (convexe ou non,
//! ondulé, en étoile ; règle pair-impair pour les croisements). Les lignes,
//! parallèles à la direction `angle` et espacées de `spacing`, sont coupées
//! par le contour puis raccourcies de `inset` (distance perpendiculaire au
//! bord rencontré). Elles sont ensuite reliées en zigzag, d'un bout à
//! l'autre, tant que le lien reste dans la pièce et court ; sinon une
//! nouvelle chaîne commence (le script la rejoint par un déplacement).
//!
//! Résultat : une liste de chaînes, chacune une liste de points à imprimer
//! d'un trait, à la hauteur du premier point du contour.

/// Un lien plus long que `LINK_MAX × spacing` le long des lignes coupe la
/// chaîne : il chevaucherait trop les lignes voisines.
const LINK_MAX: f64 = 3.0;
/// Bord presque parallèle aux lignes : le raccourcissement est plafonné à
/// `inset / SIN_MIN` (sinon une ligne rasante disparaîtrait).
const SIN_MIN: f64 = 0.25;

/// Lignes de remplissage de `contour`, en chaînes à imprimer d'un trait.
pub fn fill_lines(
    contour: &[[f64; 3]],
    spacing: f64,
    angle: f64,
    inset: f64,
) -> Vec<Vec<[f64; 3]>> {
    let mut poly: Vec<[f64; 2]> = Vec::with_capacity(contour.len());
    let (c, s) = (libm::cos(angle), libm::sin(angle));
    for p in contour {
        // Repère tourné : les lignes deviennent horizontales (y' constant).
        let q = [p[0] * c + p[1] * s, -p[0] * s + p[1] * c];
        if poly
            .last()
            .is_none_or(|l: &[f64; 2]| l[0] != q[0] || l[1] != q[1])
        {
            poly.push(q);
        }
    }
    if poly.len() > 1 && poly[0] == poly[poly.len() - 1] {
        poly.pop();
    }
    if poly.len() < 3 || spacing.is_nan() || spacing <= 0.0 {
        return Vec::new();
    }
    let z = contour[0][2];
    let (y0, y1) = poly
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| {
            (a.min(p[1]), b.max(p[1]))
        });
    let extent = y1 - y0;
    if extent.is_nan() || extent <= 0.0 {
        return Vec::new();
    }
    // Lignes centrées dans la hauteur du contour.
    let n = ((extent / spacing).floor() as usize).max(1);
    let first = y0 + (extent - (n - 1) as f64 * spacing) / 2.0;

    let rows: Vec<(f64, Vec<(f64, f64)>)> = (0..n)
        .map(|k| {
            let y = first + k as f64 * spacing;
            (y, row_intervals(&poly, y, inset, spacing * 0.5))
        })
        .collect();

    let mut used: Vec<Vec<bool>> = rows.iter().map(|(_, r)| vec![false; r.len()]).collect();
    let back = |x: f64, y: f64| [x * c - y * s, x * s + y * c, z];
    let mut chains = Vec::new();
    for r0 in 0..rows.len() {
        for i0 in 0..rows[r0].1.len() {
            if used[r0][i0] {
                continue;
            }
            used[r0][i0] = true;
            let (mut r, mut iv, mut right) = (r0, rows[r0].1[i0], true);
            let mut chain = vec![back(iv.0, rows[r].0), back(iv.1, rows[r].0)];
            // Monter de ligne en ligne tant qu'un intervalle voisin se raccorde.
            while r + 1 < rows.len() {
                let end = if right { iv.1 } else { iv.0 };
                let (y, ny) = (rows[r].0, rows[r + 1].0);
                let next = rows[r + 1]
                    .1
                    .iter()
                    .enumerate()
                    .filter(|(j, b)| !used[r + 1][*j] && b.0.max(iv.0) < b.1.min(iv.1))
                    .map(|(j, b)| (j, *b, if right { b.1 } else { b.0 }))
                    .filter(|(_, _, start)| (start - end).abs() <= LINK_MAX * spacing)
                    .min_by(|a, b| (a.2 - end).abs().total_cmp(&(b.2 - end).abs()));
                let Some((j, b, start)) = next else { break };
                if crosses(&poly, [end, y], [start, ny]) {
                    break;
                }
                used[r + 1][j] = true;
                r += 1;
                iv = b;
                right = !right;
                let (a, e) = if right { (b.0, b.1) } else { (b.1, b.0) };
                chain.push(back(a, ny));
                chain.push(back(e, ny));
            }
            chains.push(chain);
        }
    }
    chains
}

/// Intervalles intérieurs de la ligne horizontale `y`, raccourcis de `inset`,
/// triés ; ceux plus courts que `min_len` sont écartés.
fn row_intervals(poly: &[[f64; 2]], y: f64, inset: f64, min_len: f64) -> Vec<(f64, f64)> {
    let mut hits: Vec<(f64, f64)> = Vec::new();
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a[1] > y) != (b[1] > y) {
            let x = a[0] + (y - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            let sin = ((b[1] - a[1]) / len).abs().max(SIN_MIN);
            hits.push((x, inset / sin));
        }
    }
    hits.sort_by(|p, q| p.0.total_cmp(&q.0));
    hits.chunks_exact(2)
        .map(|h| (h[0].0 + h[0].1, h[1].0 - h[1].1))
        .filter(|(a, b)| b - a >= min_len)
        .collect()
}

/// Le segment `p`–`q` coupe-t-il un bord du polygone ?
fn crosses(poly: &[[f64; 2]], p: [f64; 2], q: [f64; 2]) -> bool {
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    (0..poly.len()).any(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let (d1, d2) = (cross(a, b, p), cross(a, b, q));
        let (d3, d4) = (cross(p, q, a), cross(p, q, b));
        (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(side: f64) -> Vec<[f64; 3]> {
        let h = side / 2.0;
        vec![
            [-h, -h, 0.2],
            [h, -h, 0.2],
            [h, h, 0.2],
            [-h, h, 0.2],
            [-h, -h, 0.2],
        ]
    }

    #[test]
    fn a_square_is_one_zigzag_inside_the_inset() {
        let chains = fill_lines(&square(20.0), 0.5, 0.0, 0.25);
        assert_eq!(chains.len(), 1);
        let c = &chains[0];
        // 40 lignes de 2 points, centrées, à 0,25 mm des bords.
        assert_eq!(c.len(), 80);
        for p in c {
            assert!(
                p[0].abs() <= 9.75 + 1e-9 && p[1].abs() <= 9.75 + 1e-9,
                "{p:?}"
            );
            assert_eq!(p[2], 0.2);
        }
        assert!((c[0][1] + 9.75).abs() < 1e-9);
        // Zigzag : chaque ligne repart du côté où la précédente s'arrête.
        assert!(c[1][0] > 0.0 && c[2][0] > 0.0 && c[3][0] < 0.0);
    }

    #[test]
    fn the_angle_turns_the_lines() {
        let chains = fill_lines(&square(20.0), 0.5, std::f64::consts::FRAC_PI_2, 0.25);
        let c = &chains[0];
        // Lignes parallèles à Y : x constant sur chaque ligne.
        assert!((c[0][0] - c[1][0]).abs() < 1e-9);
        assert!((c[0][1] - c[1][1]).abs() > 19.0);
    }

    #[test]
    fn a_notch_splits_the_fill_into_chains() {
        // « U » : deux branches reliées par le bas.
        let u = vec![
            [0.0, 0.0, 0.0],
            [30.0, 0.0, 0.0],
            [30.0, 30.0, 0.0],
            [20.0, 30.0, 0.0],
            [20.0, 10.0, 0.0],
            [10.0, 10.0, 0.0],
            [10.0, 30.0, 0.0],
            [0.0, 30.0, 0.0],
        ];
        let chains = fill_lines(&u, 1.0, 0.0, 0.5);
        assert!(chains.len() >= 2, "{}", chains.len());
        // Aucun lien ne traverse l'encoche (10 < x < 20 au-dessus de y = 10).
        for c in &chains {
            for w in c.windows(2) {
                let (a, b) = (w[0], w[1]);
                let above = a[1] > 10.5 && b[1] > 10.5;
                assert!(
                    !(above && a[0].min(b[0]) < 15.0 && a[0].max(b[0]) > 15.0),
                    "{a:?} → {b:?}"
                );
            }
        }
        let total: usize = chains.iter().map(Vec::len).sum();
        assert_eq!(total % 2, 0);
    }

    #[test]
    fn degenerate_contours_give_nothing() {
        assert!(fill_lines(&[], 0.5, 0.0, 0.0).is_empty());
        assert!(fill_lines(&[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], 0.5, 0.0, 0.0).is_empty());
        assert!(fill_lines(
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]],
            0.5,
            0.0,
            0.0
        )
        .is_empty());
        assert!(fill_lines(&square(10.0), 0.0, 0.0, 0.0).is_empty());
        // Contour plus petit que le retrait : rien.
        assert!(fill_lines(&square(1.0), 0.4, 0.0, 1.0).is_empty());
    }

    #[test]
    fn deterministic_on_a_rippled_contour() {
        let ripple: Vec<[f64; 3]> = (0..=400)
            .map(|i| {
                let a = std::f64::consts::TAU * i as f64 / 400.0;
                let r = 15.0
                    + 0.5 * libm::cos(50.0 * a)
                    + 5.0 * libm::pow(0.5 - 0.5 * libm::cos(4.0 * a), 1.5);
                [r * libm::cos(a), r * libm::sin(a), 0.2]
            })
            .collect();
        let a = fill_lines(&ripple, 0.45, 0.7, 0.6);
        let b = fill_lines(&ripple, 0.45, 0.7, 0.6);
        assert_eq!(a, b);
        assert!(!a.is_empty());
        for p in a.iter().flatten() {
            assert!(p[0].hypot(p[1]) < 20.6, "{p:?}");
        }
    }
}
