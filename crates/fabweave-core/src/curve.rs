//! Paramètre `curve` (format `.weave` version 2) : une silhouette donnée par
//! des points `[rayon, hauteur]` en mm, hauteurs strictement croissantes.
//! Le script la reçoit comme un objet `Curve` : `c.at(z)` rend le rayon à la
//! hauteur `z`, par un raccord lisse (interpolation cubique monotone de
//! Fritsch et Carlson : jamais au-delà des points voisins, donc aucun
//! renflement inventé) ou droit (segments), selon le modèle.
//!
//! Hors de la plage des hauteurs, `at` rend le rayon du point extrême le plus
//! proche : la courbe se prolonge à la verticale, ce que dit la spécification.

use serde::{Serialize, Serializer};

use crate::issue::Issue;

/// Nombre de points au plus, quel que soit le modèle.
pub const MAX_POINTS: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    /// `[rayon, hauteur]`, hauteurs strictement croissantes.
    pub points: Vec<[f64; 2]>,
    /// Raccord lisse (sinon droit).
    pub smooth: bool,
    /// Pentes aux points (raccord lisse), calculées une fois.
    slopes: Vec<f64>,
}

/// Une courbe s'écrit comme sa liste de points (valeurs, projets).
impl Serialize for Curve {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.points.serialize(s)
    }
}

/// Bornes d'une courbe, déclarées par le modèle.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct CurveBounds {
    /// `[rayon, hauteur]` minimaux et maximaux.
    pub min: [f64; 2],
    pub max: [f64; 2],
    /// Nombre de points minimal et maximal.
    pub points: [usize; 2],
}

impl Curve {
    pub fn new(points: Vec<[f64; 2]>, smooth: bool) -> Self {
        let slopes = if smooth {
            monotone_slopes(&points)
        } else {
            Vec::new()
        };
        Self {
            points,
            smooth,
            slopes,
        }
    }

    /// Rayon à la hauteur `z`.
    pub fn at(&self, z: f64) -> f64 {
        let p = &self.points;
        let (first, last) = (p[0], p[p.len() - 1]);
        if p.len() == 1 || z <= first[1] {
            return first[0];
        }
        if z >= last[1] {
            return last[0];
        }
        // Segment qui contient z (hauteurs croissantes : recherche dichotomique).
        let k = p.partition_point(|q| q[1] <= z) - 1;
        let ([r0, z0], [r1, z1]) = (p[k], p[k + 1]);
        let h = z1 - z0;
        let t = (z - z0) / h;
        if !self.smooth {
            return r0 + (r1 - r0) * t;
        }
        // Hermite cubique avec les pentes monotones.
        let (t2, t3) = (t * t, t * t * t);
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * r0 + h10 * h * self.slopes[k] + h01 * r1 + h11 * h * self.slopes[k + 1]
    }

    pub fn min_z(&self) -> f64 {
        self.points[0][1]
    }

    pub fn max_z(&self) -> f64 {
        self.points[self.points.len() - 1][1]
    }

    /// Plus grand rayon atteint (aux points : le raccord lisse ne les dépasse pas).
    pub fn max_r(&self) -> f64 {
        self.points.iter().map(|p| p[0]).fold(f64::MIN, f64::max)
    }
}

/// Pentes de Fritsch et Carlson : la courbe reste monotone entre deux points
/// dont les rayons sont monotones, et ne dépasse jamais leurs valeurs.
fn monotone_slopes(p: &[[f64; 2]]) -> Vec<f64> {
    let n = p.len();
    if n < 2 {
        return vec![0.0; n];
    }
    let d: Vec<f64> = p
        .windows(2)
        .map(|w| (w[1][0] - w[0][0]) / (w[1][1] - w[0][1]))
        .collect();
    let mut m = vec![0.0; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for k in 1..n - 1 {
        m[k] = if d[k - 1] * d[k] <= 0.0 {
            0.0
        } else {
            (d[k - 1] + d[k]) / 2.0
        };
    }
    for k in 0..n - 1 {
        if d[k] == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
            continue;
        }
        let (a, b) = (m[k] / d[k], m[k + 1] / d[k]);
        let s = a * a + b * b;
        if s > 9.0 {
            let tau = 3.0 / s.sqrt();
            m[k] = tau * a * d[k];
            m[k + 1] = tau * b * d[k];
        }
    }
    m
}

/// Points d'une valeur JSON : `[[rayon, hauteur], …]` de nombres finis.
pub fn points_from_json(v: &serde_json::Value) -> Option<Vec<[f64; 2]>> {
    v.as_array()?
        .iter()
        .map(|p| {
            let p = p.as_array()?;
            match p.as_slice() {
                [r, z] => {
                    let (r, z) = (r.as_f64()?, z.as_f64()?);
                    (r.is_finite() && z.is_finite()).then_some([r, z])
                }
                _ => None,
            }
        })
        .collect()
}

/// Contrôle des points contre les bornes ; jamais corrigés.
pub fn check(points: &[[f64; 2]], b: &CurveBounds) -> Result<(), Issue> {
    let n = points.len();
    if n < b.points[0] || n > b.points[1] {
        return Err(Issue::error("param.curve_point_count").count(n));
    }
    for (i, p) in points.iter().enumerate() {
        for (axis, (&v, name)) in p.iter().zip(["r", "z"]).enumerate() {
            if v < b.min[axis] || v > b.max[axis] {
                return Err(Issue::error("param.out_of_range")
                    .value(v)
                    .detail(format!("point {} {name}", i + 1)));
            }
        }
    }
    if let Some(i) = points.windows(2).position(|w| w[1][1] <= w[0][1]) {
        return Err(Issue::error("param.curve_not_increasing")
            .value(points[i + 1][1])
            .detail(format!("point {}", i + 2)));
    }
    Ok(())
}

/// Type `Curve` du script : `c.at(z)`, `c.points()`, `c.len()`, `c.min_z`,
/// `c.max_z`, `c.max_r`.
pub(crate) fn register(engine: &mut rhai::Engine) {
    use crate::script::{num, R};
    use rhai::{Dynamic, FLOAT, INT};
    engine.register_type_with_name::<Curve>("Curve");
    engine.register_fn("at", |c: &mut Curve, z: Dynamic| -> R<FLOAT> {
        Ok(c.at(num(&z, "z")?))
    });
    engine.register_fn("len", |c: &mut Curve| c.points.len() as INT);
    engine.register_fn("points", |c: &mut Curve| -> rhai::Array {
        c.points
            .iter()
            .map(|p| {
                Dynamic::from_array(vec![Dynamic::from_float(p[0]), Dynamic::from_float(p[1])])
            })
            .collect()
    });
    engine.register_get("min_z", |c: &mut Curve| c.min_z());
    engine.register_get("max_z", |c: &mut Curve| c.max_z());
    engine.register_get("max_r", |c: &mut Curve| c.max_r());
}

#[cfg(test)]
mod tests {
    use super::*;

    const B: CurveBounds = CurveBounds {
        min: [0.0, 0.0],
        max: [100.0, 200.0],
        points: [2, 8],
    };

    #[test]
    fn straight_and_smooth_pass_through_the_points() {
        let pts = vec![[30.0, 0.0], [50.0, 40.0], [20.0, 100.0]];
        for smooth in [false, true] {
            let c = Curve::new(pts.clone(), smooth);
            for p in &pts {
                assert!((c.at(p[1]) - p[0]).abs() < 1e-9);
            }
            assert_eq!(c.at(-5.0), 30.0, "prolongée à la verticale");
            assert_eq!(c.at(150.0), 20.0);
        }
        assert!((Curve::new(pts, false).at(20.0) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn smooth_curve_never_overshoots() {
        let c = Curve::new(
            vec![
                [30.0, 0.0],
                [30.0, 10.0],
                [60.0, 12.0],
                [60.0, 50.0],
                [10.0, 60.0],
            ],
            true,
        );
        for i in 0..=600 {
            let r = c.at(i as f64 / 10.0);
            assert!(
                (10.0 - 1e-9..=60.0 + 1e-9).contains(&r),
                "z = {} : r = {r}",
                i as f64 / 10.0
            );
        }
        // Plat entre deux points égaux.
        assert!((c.at(5.0) - 30.0).abs() < 1e-9);
        assert!((c.at(30.0) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn values_are_checked_not_corrected() {
        let code = |p: &[[f64; 2]]| check(p, &B).err().map(|i| i.code);
        assert_eq!(code(&[[10.0, 0.0], [20.0, 50.0]]), None);
        assert_eq!(code(&[[10.0, 0.0]]), Some("param.curve_point_count"));
        assert_eq!(
            code(&[[10.0, 0.0], [120.0, 50.0]]),
            Some("param.out_of_range")
        );
        assert_eq!(
            code(&[[10.0, 50.0], [20.0, 50.0]]),
            Some("param.curve_not_increasing")
        );
        let json = serde_json::json!([[1, 2], [3.5, 4]]);
        assert_eq!(points_from_json(&json), Some(vec![[1.0, 2.0], [3.5, 4.0]]));
        assert_eq!(points_from_json(&serde_json::json!([[1, 2, 3]])), None);
        assert_eq!(points_from_json(&serde_json::json!(5)), None);
    }
}
