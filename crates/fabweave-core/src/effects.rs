//! Effets de parcours (R-021) : textures tracées par la buse elle-même, sur
//! une liste de points qui tourne autour de l'axe Z (spirale d'un vase,
//! `revolve`). Calcul natif ; l'effet ne parle que de géométrie, de section
//! de matière et de facteur de vitesse : jamais d'`E` ni de température.
//!
//! `effect(kind, opts)` crée l'effet ; `pts.with_effect(e)` rend les points
//! déplacés ; `path.print_effect(pts, e, w, h)` les imprime, avec la vitesse
//! de crête de la dentelle et la section du gros boudin.
//!
//! Repère : angle `a` déroulé le long de la liste (un tour = 2π, sans saut),
//! phase `p = a · repeats / 2π` (un motif par unité), tour `a / 2π`.
//! - `waves` (vagues) : la buse monte et descend, `z += I · sin(2πp)` ;
//! - `weave` (trame) : `r += I · sin(2πp + π · tour)`, décalé d'un demi-motif
//!   à chaque tour, la paroi se tisse ;
//! - `columns` (colonnes) : `r += I · (½ + ½ cos 2πp)`, alignées d'un tour à
//!   l'autre ;
//! - `spikes` (picots) : `r += I · (1 − |2 frac(p) − 1|)³`, pointes vers
//!   l'extérieur ;
//! - `grain` : `r += I · bruit(…)`, relief irrégulier de `repeats` grains par
//!   tour environ ;
//! - `lace` (dentelle) : arches de hauteur `I` (`z += I · arche`), décalées
//!   d'un demi-motif à chaque tour : chaque fil ne touche la rangée du
//!   dessous qu'à son pied ; crête ralentie (`crest_speed`), pied rentré de
//!   `foot_shift` mm ;
//! - `bead` (gros boudin) : aucune géométrie, largeur et hauteur du boudin
//!   (`bead_width`, `bead_height`) indépendantes de la buse.
//!
//! Plage : `from`, `to` (hauteurs), `transition` (mm) pour une entrée douce.

use std::f64::consts::{PI, TAU};

use rhai::{Dynamic, Engine};

use crate::points::Points;
use crate::script::{fault, num, ScriptPath, R};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EffectKind {
    Waves,
    Weave,
    Columns,
    Spikes,
    Grain,
    Lace,
    Bead,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Effect {
    pub kind: EffectKind,
    /// Amplitude (mm).
    pub intensity: f64,
    /// Motifs par tour.
    pub repeats: f64,
    pub from: f64,
    pub to: f64,
    pub transition: f64,
    /// Facteur de vitesse à la crête (dentelle), de 0,1 à 1.
    pub crest_speed: f64,
    /// Recul du pied vers l'intérieur (dentelle, mm).
    pub foot_shift: f64,
    pub bead_width: Option<f64>,
    pub bead_height: Option<f64>,
}

fn frac(x: f64) -> f64 {
    x - libm::floor(x)
}

impl Effect {
    pub fn new(kind: &str, opts: &rhai::Map) -> R<Self> {
        let kind = match kind {
            "waves" => EffectKind::Waves,
            "weave" => EffectKind::Weave,
            "columns" => EffectKind::Columns,
            "spikes" => EffectKind::Spikes,
            "grain" => EffectKind::Grain,
            "lace" => EffectKind::Lace,
            "bead" => EffectKind::Bead,
            other => return Err(fault("effect.unknown", other.to_string())),
        };
        let get = |key: &str, default: f64| -> R<f64> {
            opts.get(key).map_or(Ok(default), |v| num(v, key))
        };
        let bad = |key: &str, v: f64| fault("points.invalid_value", format!("{key} : {v}"));
        let e = Self {
            kind,
            intensity: get("intensity", 1.0)?,
            repeats: get("repeats", 12.0)?,
            from: get("from", f64::NEG_INFINITY)?,
            to: get("to", f64::INFINITY)?,
            transition: get("transition", 0.0)?,
            crest_speed: get("crest_speed", 1.0)?,
            foot_shift: get("foot_shift", 0.0)?,
            bead_width: opts
                .get("bead_width")
                .map(|v| num(v, "bead_width"))
                .transpose()?,
            bead_height: opts
                .get("bead_height")
                .map(|v| num(v, "bead_height"))
                .transpose()?,
        };
        let checks = [
            (
                "intensity",
                e.intensity,
                e.intensity >= 0.0 && e.intensity <= 30.0,
            ),
            ("repeats", e.repeats, e.repeats > 0.0 && e.repeats <= 500.0),
            (
                "transition",
                e.transition,
                e.transition >= 0.0 && e.transition.is_finite(),
            ),
            (
                "crest_speed",
                e.crest_speed,
                (0.1..=1.0).contains(&e.crest_speed),
            ),
            (
                "foot_shift",
                e.foot_shift,
                (0.0..=3.0).contains(&e.foot_shift),
            ),
            ("from", e.from, !e.from.is_nan() && e.from <= e.to),
        ];
        if let Some((key, v, _)) = checks.iter().find(|c| !c.2) {
            return Err(bad(key, *v));
        }
        for (key, v) in [("bead_width", e.bead_width), ("bead_height", e.bead_height)] {
            if let Some(v) = v.filter(|v| !(*v > 0.0 && *v <= 10.0)) {
                return Err(bad(key, v));
            }
        }
        Ok(e)
    }

    /// Part de l'effet à la hauteur `z` (0 hors plage, montée sur `transition`).
    fn weight(&self, z: f64) -> f64 {
        if z < self.from || z > self.to {
            return 0.0;
        }
        if self.transition <= 0.0 {
            return 1.0;
        }
        let g = ((z - self.from).min(self.to - z) / self.transition).clamp(0.0, 1.0);
        g * g * (3.0 - 2.0 * g)
    }

    /// Points déplacés et facteur de vitesse de chacun.
    pub fn apply(&self, pts: &[[f64; 3]]) -> R<Vec<([f64; 3], f64)>> {
        let mut out = Vec::with_capacity(pts.len());
        let mut unwrapped = 0.0;
        let mut prev: Option<f64> = None;
        for &[x, y, z] in pts {
            let r = x.hypot(y);
            let a = libm::atan2(y, x);
            unwrapped += prev.map_or(0.0, |p| {
                let d = a - p;
                d - TAU * libm::round(d / TAU)
            });
            prev = Some(a);
            let w = self.weight(z);
            let (i, n) = (self.intensity * w, self.repeats);
            let p = unwrapped * n / TAU;
            let turn = unwrapped / TAU;
            let (mut dr, mut dz, mut speed) = (0.0, 0.0, 1.0);
            match self.kind {
                EffectKind::Waves => dz = i * libm::sin(TAU * p),
                EffectKind::Weave => dr = i * libm::sin(TAU * p + PI * turn),
                EffectKind::Columns => dr = i * (0.5 + 0.5 * libm::cos(TAU * p)),
                EffectKind::Spikes => dr = i * (1.0 - (2.0 * frac(p) - 1.0).abs()).powi(3),
                EffectKind::Grain => {
                    // Bruit sur le cercle (sans couture) et la hauteur, à
                    // l'échelle d'un grain : un tour fait `n` grains.
                    let k = n / TAU;
                    let size = TAU * r.max(1.0) / n;
                    dr = i * crate::noise::noise3(libm::cos(a) * k, libm::sin(a) * k, z / size);
                }
                EffectKind::Lace => {
                    let arch = libm::sin(PI * frac(p + turn / 2.0));
                    dz = i * arch;
                    dr = -self.foot_shift * w * (1.0 - arch);
                    speed = 1.0 - (1.0 - self.crest_speed) * w * arch;
                }
                EffectKind::Bead => {}
            }
            let r2 = r + dr;
            if dr != 0.0 && !(r2 >= 0.5 && r > 1e-9) {
                return Err(fault(
                    "shape.radius_too_small",
                    format!("z = {z:.2}, r = {r2:.2}"),
                ));
            }
            let s = if dr != 0.0 { r2 / r } else { 1.0 };
            out.push(([x * s, y * s, z + dz], speed));
        }
        Ok(out)
    }
}

pub(crate) fn register(engine: &mut Engine) {
    engine.register_type_with_name::<Effect>("Effect");
    engine.register_fn("effect", |kind: &str| Effect::new(kind, &rhai::Map::new()));
    engine.register_fn("effect", |kind: &str, opts: rhai::Map| {
        Effect::new(kind, &opts)
    });
    engine.register_fn("with_effect", |p: &mut Points, e: Effect| -> R<Points> {
        Ok(Points(e.apply(&p.0)?.into_iter().map(|(q, _)| q).collect()))
    });
    engine.register_fn(
        "print_effect",
        |path: &mut ScriptPath, p: Points, e: Effect, w: Dynamic, h: Dynamic| -> R<()> {
            let w = e.bead_width.map_or_else(|| num(&w, "w"), Ok)?;
            let h = e.bead_height.map_or_else(|| num(&h, "h"), Ok)?;
            let moved = e.apply(&p.0)?;
            let skip = usize::from(path.at(moved.first().map(|m| &m.0)));
            for (q, speed) in moved.into_iter().skip(skip) {
                path.push_scaled(q, w, h, speed)?;
            }
            Ok(())
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cylindre de rayon 20 : `turns` tours de 360 points, pas de 0,4 mm.
    fn cylinder(turns: usize) -> Vec<[f64; 3]> {
        (0..=turns * 360)
            .map(|i| {
                let a = i as f64 / 360.0 * TAU;
                [
                    20.0 * libm::cos(a),
                    20.0 * libm::sin(a),
                    1.0 + 0.4 * i as f64 / 360.0,
                ]
            })
            .collect()
    }

    fn fx(kind: &str, pairs: &[(&str, f64)]) -> Effect {
        let mut m = rhai::Map::new();
        for (k, v) in pairs {
            m.insert((*k).into(), Dynamic::from_float(*v));
        }
        Effect::new(kind, &m).unwrap()
    }

    /// Maxima locaux d'une suite sur un tour (motifs).
    fn peaks(v: &[f64]) -> usize {
        let n = v.len();
        (0..n)
            .filter(|&i| v[i] > v[(i + n - 1) % n] && v[i] >= v[(i + 1) % n])
            .count()
    }

    #[test]
    fn amplitude_and_repeats_are_those_asked() {
        let base = cylinder(3);
        for kind in ["weave", "columns", "spikes"] {
            let out = fx(kind, &[("intensity", 1.5), ("repeats", 12.0)])
                .apply(&base)
                .unwrap();
            let r: Vec<f64> = out.iter().map(|(q, _)| q[0].hypot(q[1]) - 20.0).collect();
            let hi = r.iter().cloned().fold(f64::MIN, f64::max);
            assert!((hi - 1.5).abs() < 0.01, "{kind} : {hi}");
            if kind != "weave" {
                assert_eq!(peaks(&r[360..720]), 12, "{kind}");
            }
        }
        let out = fx("waves", &[("intensity", 0.8), ("repeats", 6.0)])
            .apply(&base)
            .unwrap();
        let dz: Vec<f64> = out
            .iter()
            .zip(&base)
            .map(|((q, _), b)| q[2] - b[2])
            .collect();
        assert!((dz.iter().cloned().fold(f64::MIN, f64::max) - 0.8).abs() < 0.01);
        assert_eq!(peaks(&dz[360..720]), 6);
    }

    #[test]
    fn weave_shifts_by_half_a_motif_each_turn() {
        let base = cylinder(2);
        let out = fx("weave", &[("intensity", 1.0), ("repeats", 8.0)])
            .apply(&base)
            .unwrap();
        let r = |i: usize| out[i].0[0].hypot(out[i].0[1]) - 20.0;
        // Même angle, tour suivant : signe opposé.
        assert!((r(11) + r(371)).abs() < 0.05, "{} {}", r(11), r(371));
    }

    #[test]
    fn lace_slows_at_the_crest_and_range_fades() {
        let base = cylinder(2);
        let out = fx(
            "lace",
            &[("intensity", 3.0), ("repeats", 10.0), ("crest_speed", 0.4)],
        )
        .apply(&base)
        .unwrap();
        let slowest = out.iter().map(|o| o.1).fold(f64::MAX, f64::min);
        assert!((slowest - 0.4).abs() < 0.01);
        let ranged = fx("columns", &[("intensity", 2.0), ("from", 1.5), ("to", 1.6)])
            .apply(&base)
            .unwrap();
        assert_eq!(ranged[0].0, base[0], "hors plage");
    }

    #[test]
    fn bad_settings_are_refused() {
        let e = |k: &str, pairs: &[(&str, f64)]| {
            let mut m = rhai::Map::new();
            for (k, v) in pairs {
                m.insert((*k).into(), Dynamic::from_float(*v));
            }
            Effect::new(k, &m).is_err()
        };
        assert!(e("swirl", &[]));
        assert!(e("waves", &[("intensity", -1.0)]));
        assert!(e("waves", &[("repeats", 0.0)]));
        assert!(e("lace", &[("crest_speed", 0.0)]));
        assert!(e("bead", &[("bead_width", 0.0)]));
    }
}
