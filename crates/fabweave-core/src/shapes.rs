//! Formes natives (R-023) : sections vues de dessus, torsion rythmée, cage
//! de déformation et révolution d'une silhouette en spirale continue.
//!
//! - `section(kind, sides, depth)` : objet `Section` ; `s.at(angle)` rend le
//!   facteur du rayon (1 au plus : la silhouette est l'enveloppe, la section
//!   creuse vers l'intérieur). Types : `round`, `faceted`, `petal`, `star`,
//!   `fluted`, `flower`, `teeth`, `drop` ; `depth` de 0 à 0,9.
//! - `twist_angle(kind, degrees, t)` : rotation (radians) à la part `t` de la
//!   hauteur (0 en bas, 1 en haut) ; `even` (régulière), `s` (en S),
//!   `back_and_forth` (aller-retour : revient à 0 en haut).
//! - `cage(rings, per_ring, offsets, softness)` : objet `Cage` ; anneaux
//!   répartis du bas (`t` = 0) au haut (`t` = 1), `offsets` en mm (anneau du
//!   bas d'abord, angle 0 d'abord) ; `c.at(angle, t)` rend le déplacement
//!   radial. `softness` 0 : la paroi passe par les points (Catmull-Rom) ;
//!   1 : plus douce, en deçà des points (B-spline) ; entre les deux, mélange.
//! - `revolve(opts)` et `revolve_ring(opts, z)` : spirale (ou un tour fermé à
//!   la hauteur `z`) qui suit la silhouette `opts.profile` (`Curve`), avec
//!   `section`, `twist`, `twist_kind`, `taper`, `cage` facultatifs ;
//!   `start`, `pitch` (montée par tour) et `segments` (par tour) ; `end` : par
//!   défaut, le haut de la silhouette.
//!
//! Angles en radians dans le script, sauf la torsion donnée en degrés comme
//! un paramètre. Valeur hors domaine : erreur à code stable, jamais corrigée.

use std::f64::consts::{PI, TAU};

use rhai::{Dynamic, Engine, FLOAT, INT};

use crate::curve::Curve;
use crate::points::{Builder, Points};
use crate::script::{fault, num, R};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SectionKind {
    Round,
    Faceted,
    Petal,
    Star,
    Fluted,
    Flower,
    Teeth,
    Drop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    pub kind: SectionKind,
    pub sides: usize,
    pub depth: f64,
}

/// Partie fractionnaire positive.
fn frac(x: f64) -> f64 {
    x - libm::floor(x)
}

impl Section {
    pub fn new(kind: &str, sides: usize, depth: f64) -> Result<Self, (&'static str, String)> {
        let kind = match kind {
            "round" => SectionKind::Round,
            "faceted" => SectionKind::Faceted,
            "petal" => SectionKind::Petal,
            "star" => SectionKind::Star,
            "fluted" => SectionKind::Fluted,
            "flower" => SectionKind::Flower,
            "teeth" => SectionKind::Teeth,
            "drop" => SectionKind::Drop,
            other => return Err(("shape.unknown_section", other.to_string())),
        };
        if !(3..=60).contains(&sides) {
            return Err(("points.invalid_count", format!("sides : {sides}")));
        }
        if !(0.0..=0.9).contains(&depth) {
            return Err(("points.invalid_value", format!("depth : {depth}")));
        }
        Ok(Self { kind, sides, depth })
    }

    /// Facteur du rayon à l'angle `a` (radians), entre 1 − depth et 1.
    pub fn at(&self, a: f64) -> f64 {
        let n = self.sides as f64;
        let d = self.depth;
        // Part du secteur (0 au bord d'un lobe, 0,5 au milieu).
        let f = frac(a * n / TAU);
        let w = (2.0 * f - 1.0).abs(); // 1 aux arêtes, 0 au milieu du secteur
        let shape = match self.kind {
            SectionKind::Round => 1.0,
            SectionKind::Faceted => {
                // Polygone inscrit dans le cercle, sommets aux arêtes.
                let phi = (f - 0.5) * TAU / n;
                libm::cos(PI / n) / libm::cos(phi)
            }
            SectionKind::Petal => libm::sqrt(libm::fabs(libm::cos(a * n / 2.0))),
            SectionKind::Star => w,
            SectionKind::Fluted => 1.0 - libm::sqrt((1.0 - w * w).max(0.0)),
            SectionKind::Flower => 0.5 + 0.5 * libm::cos(a * n),
            SectionKind::Teeth => ((w - 0.35) / 0.3).clamp(0.0, 1.0),
            SectionKind::Drop => {
                let b = libm::atan2(libm::sin(a), libm::cos(a)); // dans [-π, π]
                libm::pow(1.0 - b.abs() / PI, 4.0)
            }
        };
        1.0 - d * (1.0 - shape)
    }
}

/// Rotation (radians) à la part `t` de la hauteur, pour `degrees` en tout.
pub fn twist_angle(kind: &str, degrees: f64, t: f64) -> Result<f64, (&'static str, String)> {
    let total = degrees.to_radians();
    Ok(match kind {
        "even" => total * t,
        "s" => total * t * t * (3.0 - 2.0 * t),
        "back_and_forth" => total * libm::sin(PI * t),
        other => return Err(("shape.unknown_twist", other.to_string())),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cage {
    rings: usize,
    per_ring: usize,
    offsets: Vec<f64>,
    softness: f64,
}

/// Mélange Catmull-Rom (passe par les points) et B-spline (plus douce).
fn blend(p: [f64; 4], f: f64, softness: f64) -> f64 {
    let (f2, f3) = (f * f, f * f * f);
    let cr = 0.5
        * (2.0 * p[1]
            + (p[2] - p[0]) * f
            + (2.0 * p[0] - 5.0 * p[1] + 4.0 * p[2] - p[3]) * f2
            + (3.0 * p[1] - p[0] - 3.0 * p[2] + p[3]) * f3);
    let bs = ((1.0 - f).powi(3) * p[0]
        + (3.0 * f3 - 6.0 * f2 + 4.0) * p[1]
        + (-3.0 * f3 + 3.0 * f2 + 3.0 * f + 1.0) * p[2]
        + f3 * p[3])
        / 6.0;
    cr + (bs - cr) * softness
}

impl Cage {
    pub fn new(
        rings: usize,
        per_ring: usize,
        offsets: Vec<f64>,
        softness: f64,
    ) -> Result<Self, (&'static str, String)> {
        if !(2..=8).contains(&rings) {
            return Err(("points.invalid_count", format!("rings : {rings}")));
        }
        if !(3..=24).contains(&per_ring) {
            return Err(("points.invalid_count", format!("per_ring : {per_ring}")));
        }
        if offsets.len() != rings * per_ring {
            return Err((
                "shape.cage_size",
                format!("{} / {}", offsets.len(), rings * per_ring),
            ));
        }
        if offsets.iter().any(|o| !o.is_finite()) || !(0.0..=1.0).contains(&softness) {
            return Err(("points.invalid_value", "cage".to_string()));
        }
        Ok(Self {
            rings,
            per_ring,
            offsets,
            softness,
        })
    }

    /// Déplacement de l'anneau `j` à l'angle `a` (autour : périodique).
    fn ring(&self, j: usize, a: f64) -> f64 {
        let n = self.per_ring;
        let u = frac(a / TAU) * n as f64;
        let i = u as usize % n;
        let f = u - libm::floor(u);
        let p = |k: isize| self.offsets[j * n + (k.rem_euclid(n as isize)) as usize];
        let i = i as isize;
        blend([p(i - 1), p(i), p(i + 1), p(i + 2)], f, self.softness)
    }

    /// Déplacement radial (mm) à l'angle `a`, à la part `t` de la hauteur.
    pub fn at(&self, a: f64, t: f64) -> f64 {
        let m = self.rings;
        let v = t.clamp(0.0, 1.0) * (m - 1) as f64;
        let j = (v as usize).min(m - 2);
        let f = v - j as f64;
        // Au-delà des anneaux extrêmes : prolongés tels quels.
        let r = |k: isize| self.ring(k.clamp(0, m as isize - 1) as usize, a);
        let j = j as isize;
        blend([r(j - 1), r(j), r(j + 1), r(j + 2)], f, self.softness)
    }
}

/// Réglages d'une révolution, lus dans la table `opts` du script.
struct Revolve {
    profile: Curve,
    section: Option<Section>,
    twist: f64,
    twist_kind: String,
    taper: f64,
    cage: Option<Cage>,
    start: f64,
    pitch: f64,
    segments: usize,
    end: f64,
}

fn opt_num(m: &rhai::Map, key: &str) -> R<Option<f64>> {
    m.get(key).map(|v| num(v, key)).transpose()
}

impl Revolve {
    fn read(m: &rhai::Map) -> R<Self> {
        let bad = |key: &str| fault("points.invalid_value", key.to_string());
        let profile = m
            .get("profile")
            .and_then(|v| v.clone().try_cast::<Curve>())
            .ok_or_else(|| bad("profile"))?;
        let section = match m.get("section") {
            None => None,
            Some(v) => Some(
                v.clone()
                    .try_cast::<Section>()
                    .ok_or_else(|| bad("section"))?,
            ),
        };
        let cage = match m.get("cage") {
            None => None,
            Some(v) => Some(v.clone().try_cast::<Cage>().ok_or_else(|| bad("cage"))?),
        };
        let twist_kind = match m.get("twist_kind") {
            None => "even".to_string(),
            Some(v) => v.clone().into_string().map_err(|_| bad("twist_kind"))?,
        };
        let start = opt_num(m, "start")?.ok_or_else(|| bad("start"))?;
        let pitch = opt_num(m, "pitch")?.ok_or_else(|| bad("pitch"))?;
        if !(pitch > 0.0 && pitch.is_finite()) {
            return Err(bad("pitch"));
        }
        let segments = opt_num(m, "segments")?.unwrap_or(120.0);
        if segments.fract() != 0.0 || !(3.0..=10_000.0).contains(&segments) {
            return Err(fault(
                "points.invalid_count",
                format!("segments : {segments}"),
            ));
        }
        let taper = opt_num(m, "taper")?.unwrap_or(0.0);
        if !(-0.9..=0.9).contains(&taper) {
            return Err(bad("taper"));
        }
        let end = opt_num(m, "end")?.unwrap_or(profile.max_z());
        Ok(Self {
            twist: opt_num(m, "twist")?.unwrap_or(0.0),
            twist_kind,
            taper,
            section,
            cage,
            start,
            pitch,
            segments: segments as usize,
            end,
            profile,
        })
    }

    /// Point à l'angle de spirale `theta`, à la hauteur `z`.
    fn point(&self, theta: f64, z: f64) -> R<[f64; 3]> {
        let (z0, z1) = (self.profile.min_z(), self.profile.max_z());
        let t = if z1 > z0 {
            ((z - z0) / (z1 - z0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let tw =
            twist_angle(&self.twist_kind, self.twist, t).map_err(|(code, d)| fault(code, d))?;
        let local = theta - tw;
        let mut r = self.profile.at(z) * (1.0 - self.taper * t);
        if let Some(s) = &self.section {
            r *= s.at(local);
        }
        if let Some(c) = &self.cage {
            r += c.at(local, t);
        }
        if r.is_nan() || r < 0.5 {
            return Err(fault(
                "shape.radius_too_small",
                format!("z = {z:.2}, r = {r:.2}"),
            ));
        }
        Ok([r * libm::cos(theta), r * libm::sin(theta), z])
    }
}

/// Départ des tours : devant, au milieu (comme les vases du catalogue).
const START: f64 = -PI / 2.0;

pub(crate) fn register(engine: &mut Engine, limit: usize) {
    let err = |(code, d): (&'static str, String)| fault(code, d);
    engine.register_type_with_name::<Section>("Section");
    engine.register_type_with_name::<Cage>("Cage");
    engine.register_fn(
        "section",
        move |kind: &str, sides: Dynamic, depth: Dynamic| -> R<Section> {
            let sides = num(&sides, "sides")?;
            if sides.fract() != 0.0 || sides < 0.0 {
                return Err(fault("points.invalid_count", format!("sides : {sides}")));
            }
            Section::new(kind, sides as usize, num(&depth, "depth")?).map_err(err)
        },
    );
    engine.register_fn("at", |s: &mut Section, a: Dynamic| -> R<FLOAT> {
        Ok(s.at(num(&a, "angle")?))
    });
    engine.register_fn(
        "twist_angle",
        move |kind: &str, deg: Dynamic, t: Dynamic| -> R<FLOAT> {
            twist_angle(kind, num(&deg, "degrees")?, num(&t, "t")?).map_err(err)
        },
    );
    engine.register_fn(
        "cage",
        move |rings: INT, per_ring: INT, offsets: rhai::Array, soft: Dynamic| -> R<Cage> {
            let offsets = offsets
                .iter()
                .map(|o| num(o, "offsets"))
                .collect::<R<Vec<f64>>>()?;
            let (rings, per_ring) = (rings.max(0) as usize, per_ring.max(0) as usize);
            Cage::new(rings, per_ring, offsets, num(&soft, "softness")?).map_err(err)
        },
    );
    engine.register_fn("at", |c: &mut Cage, a: Dynamic, t: Dynamic| -> R<FLOAT> {
        Ok(c.at(num(&a, "angle")?, num(&t, "t")?))
    });
    engine.register_fn("revolve", move |opts: rhai::Map| -> R<Points> {
        let rv = Revolve::read(&opts)?;
        let turns = (rv.end - rv.start) / rv.pitch;
        if turns.is_nan() || turns <= 0.0 {
            return Err(fault(
                "shape.no_turn",
                format!("{:.2} → {:.2}", rv.start, rv.end),
            ));
        }
        let n = (turns * rv.segments as f64).floor() as usize;
        let mut out = Builder::new(limit, n + 1)?;
        for i in 0..=n {
            let tv = i as f64 / rv.segments as f64;
            out.push(rv.point(START + tv * TAU, rv.start + tv * rv.pitch)?)?;
        }
        Ok(out.done())
    });
    engine.register_fn(
        "revolve_ring",
        move |opts: rhai::Map, z: Dynamic| -> R<Points> {
            let rv = Revolve::read(&opts)?;
            let z = num(&z, "z")?;
            let mut out = Builder::new(limit, rv.segments + 1)?;
            for i in 0..=rv.segments {
                let a = START + i as f64 / rv.segments as f64 * TAU;
                out.push(rv.point(a, z)?)?;
            }
            Ok(out.done())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nombre de minima du rayon sur un tour (creux entre lobes).
    fn valleys(s: &Section) -> usize {
        let n = 3600;
        let r: Vec<f64> = (0..n).map(|i| s.at(i as f64 / n as f64 * TAU)).collect();
        (0..n)
            .filter(|&i| {
                let (a, b, c) = (r[(i + n - 1) % n], r[i], r[(i + 1) % n]);
                b < a && b <= c
            })
            .count()
    }

    #[test]
    fn sections_have_their_lobes_and_depth() {
        for kind in ["faceted", "petal", "star", "fluted", "flower", "teeth"] {
            for sides in [3, 5, 12] {
                let s = Section::new(kind, sides, 0.3).unwrap();
                assert_eq!(valleys(&s), sides, "{kind} {sides}");
                let r: Vec<f64> = (0..3600).map(|i| s.at(i as f64 / 3600.0 * TAU)).collect();
                let (lo, hi) = r
                    .iter()
                    .fold((9.0f64, 0.0f64), |(a, b), &x| (a.min(x), b.max(x)));
                assert!((hi - 1.0).abs() < 1e-3, "{kind} {sides} : max {hi}");
                if kind != "faceted" {
                    assert!((lo - 0.7).abs() < 1e-3, "{kind} {sides} : min {lo}");
                }
            }
        }
        let drop = Section::new("drop", 3, 0.5).unwrap();
        assert_eq!(valleys(&drop), 1);
        assert_eq!(drop.at(0.0), 1.0);
        assert_eq!(Section::new("round", 3, 0.5).unwrap().at(1.0), 1.0);
        assert!(Section::new("blob", 3, 0.5).is_err());
        assert!(Section::new("star", 2, 0.5).is_err());
        assert!(Section::new("star", 5, 0.95).is_err());
    }

    #[test]
    fn twist_rhythms() {
        let a = |k: &str, t: f64| twist_angle(k, 90.0, t).unwrap().to_degrees();
        assert!((a("even", 1.0) - 90.0).abs() < 1e-9);
        assert!((a("even", 0.5) - 45.0).abs() < 1e-9);
        assert!((a("s", 1.0) - 90.0).abs() < 1e-9);
        assert!(a("s", 0.1) < a("even", 0.1));
        assert!(a("back_and_forth", 1.0).abs() < 1e-9);
        assert!((a("back_and_forth", 0.5) - 90.0).abs() < 1e-9);
        assert!(twist_angle("spin", 90.0, 0.5).is_err());
    }

    #[test]
    fn cage_at_rest_does_nothing_and_softness_matters() {
        let rest = Cage::new(3, 6, vec![0.0; 18], 0.5).unwrap();
        for i in 0..100 {
            assert_eq!(rest.at(i as f64 * 0.1, i as f64 / 99.0), 0.0);
        }
        let mut o = vec![0.0; 18];
        o[6] = 6.0; // anneau du milieu, angle 0
        let sharp = Cage::new(3, 6, o.clone(), 0.0).unwrap();
        let soft = Cage::new(3, 6, o, 1.0).unwrap();
        assert!(
            (sharp.at(0.0, 0.5) - 6.0).abs() < 1e-9,
            "passe par le point"
        );
        assert!(soft.at(0.0, 0.5) < 6.0 && soft.at(0.0, 0.5) > 1.0);
        assert!(sharp.at(PI, 0.5).abs() < 1e-9, "loin du point");
        assert!(Cage::new(3, 6, vec![0.0; 17], 0.5).is_err());
        assert!(Cage::new(1, 6, vec![0.0; 6], 0.5).is_err());
    }
}
