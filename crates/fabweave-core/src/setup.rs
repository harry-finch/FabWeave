//! Préparation de l'impression, choisie par l'utilisateur et non par le
//! modèle : buse montée (diamètre, haut débit, abrasive), placement sur le
//! plateau (position, rotation, copies en grille) et adhésion (bordure ou
//! radeau). Le `.weave` n'en sait rien, sinon par les variables `printer.*`
//! qui suivent la buse choisie.
//!
//! - **Buse** : profils « effectifs » dérivés des profils livrés, jamais
//!   modifiés : diamètre, hauteurs de couche du profil pour cette buse
//!   (interpolées entre les buses connues pour un diamètre libre, et dit),
//!   débit maximal du filament pour cette buse.
//! - **Copies** : imprimées l'une après l'autre (pièce par pièce), en grille,
//!   espacées au moins du rayon de dégagement de la tête, et refusées si la
//!   pièce dépasse la hauteur libre sous le portique.
//! - **Adhésion** : voir `adhesion.rs` ; ajoutée au parcours avant le
//!   placement, si bien que son emprise compte dans les contrôles du plateau.

use serde::{Deserialize, Serialize};

use crate::adhesion;
use crate::issue::Issue;
use crate::path::{MoveKind, Toolpath};
use crate::profile::{same_diameter, FilamentProfile, PrinterProfile};

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Setup {
    pub nozzle: NozzleSetup,
    pub placement: Placement,
    pub adhesion: Adhesion,
}

/// Buse montée ; `None` : celle du profil.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NozzleSetup {
    pub diameter: Option<f64>,
    pub high_flow: Option<bool>,
    pub abrasive: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Placement {
    /// Centre de l'encombrement sur le plateau (repère machine, mm) ;
    /// `None` : centre du plateau.
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// Rotation autour de Z (degrés, sens trigonométrique).
    pub rotation: f64,
    /// Copies en grille : colonnes (X), rangées (Y).
    pub copies: [u32; 2],
    /// Écart entre deux copies (mm) ; `None` : rayon de dégagement de la tête.
    pub spacing: Option<f64>,
}

impl Default for Placement {
    fn default() -> Self {
        Self {
            x: None,
            y: None,
            rotation: 0.0,
            copies: [1, 1],
            spacing: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Adhesion {
    #[default]
    None,
    Brim {
        /// Largeur de la bordure (mm).
        #[serde(default = "brim_width")]
        width: f64,
        /// Écart entre la bordure et la pièce (mm).
        #[serde(default = "brim_gap")]
        gap: f64,
    },
    Raft {
        /// Nombre de couches (base, intermédiaires, contact).
        #[serde(default = "raft_layers")]
        layers: u32,
        /// Écart en Z entre le radeau et la pièce, pour le décollage (mm).
        #[serde(default = "raft_gap")]
        gap: f64,
        /// Débord autour de l'empreinte (mm).
        #[serde(default = "raft_margin")]
        margin: f64,
    },
}

fn brim_width() -> f64 {
    5.0
}
fn brim_gap() -> f64 {
    0.1
}
fn raft_layers() -> u32 {
    3
}
fn raft_gap() -> f64 {
    0.2
}
fn raft_margin() -> f64 {
    3.0
}

/// Copies au plus (produit des deux nombres).
pub const MAX_COPIES: u32 = 100;

fn range(out: &mut Vec<Issue>, field: &str, v: f64, lo: f64, hi: f64) {
    if !(v.is_finite() && v >= lo && v <= hi) {
        out.push(
            Issue::error("setup.out_of_range")
                .field(field.to_string())
                .value(v),
        );
    }
}

impl Setup {
    /// Contrôle des valeurs (jamais corrigées).
    pub fn check(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        if let Some(d) = self.nozzle.diameter {
            range(&mut out, "nozzle.diameter", d, 0.1, 2.0);
        }
        let p = &self.placement;
        for (f, v) in [("placement.x", p.x), ("placement.y", p.y)] {
            if let Some(v) = v {
                range(&mut out, f, v, -10_000.0, 10_000.0);
            }
        }
        range(&mut out, "placement.rotation", p.rotation, -360.0, 360.0);
        for (k, n) in p.copies.iter().enumerate() {
            range(
                &mut out,
                ["placement.copies.x", "placement.copies.y"][k],
                f64::from(*n),
                1.0,
                f64::from(MAX_COPIES),
            );
        }
        if p.copies[0].saturating_mul(p.copies[1]) > MAX_COPIES {
            out.push(
                Issue::error("setup.too_many_copies")
                    .field("placement.copies")
                    .value(f64::from(MAX_COPIES)),
            );
        }
        if let Some(s) = p.spacing {
            range(&mut out, "placement.spacing", s, 0.0, 500.0);
        }
        match self.adhesion {
            Adhesion::None => {}
            Adhesion::Brim { width, gap } => {
                range(&mut out, "adhesion.width", width, 0.1, 50.0);
                range(&mut out, "adhesion.gap", gap, 0.0, 5.0);
            }
            Adhesion::Raft {
                layers,
                gap,
                margin,
            } => {
                range(&mut out, "adhesion.layers", f64::from(layers), 1.0, 10.0);
                range(&mut out, "adhesion.gap", gap, 0.0, 1.0);
                range(&mut out, "adhesion.margin", margin, 0.0, 20.0);
            }
        }
        out
    }

    /// Profils tels que la buse choisie les rend : jamais les profils livrés
    /// eux-mêmes. Messages : buse hors des buses du profil (couches
    /// interpolées), haut débit ou buse abrasive non proposés, filament
    /// abrasif dans une buse qui ne l'est pas.
    pub fn effective(
        &self,
        printer: &PrinterProfile,
        filament: &FilamentProfile,
    ) -> (PrinterProfile, FilamentProfile, Vec<Issue>) {
        let mut issues = Vec::new();
        let mut p = printer.clone();
        let mut f = filament.clone();
        let n = &self.nozzle;
        let base = printer.nozzle.diameter;
        let d = n.diameter.unwrap_or(base);
        if let Some(hf) = n.high_flow {
            if hf && !printer.nozzle.high_flow_available && !printer.nozzle.high_flow {
                issues.push(Issue::error("setup.high_flow_unavailable").field("nozzle.high_flow"));
            }
            p.nozzle.high_flow = hf;
        }
        if let Some(ab) = n.abrasive {
            if ab && !printer.nozzle.abrasive_available && !printer.nozzle.abrasive {
                issues.push(Issue::error("setup.abrasive_unavailable").field("nozzle.abrasive"));
            }
            p.nozzle.abrasive = ab;
        }
        if !same_diameter(d, base) {
            p.nozzle.diameter = d;
            let l = &printer.layers;
            // Buses connues : celle du profil et ses variantes.
            let mut known: Vec<(f64, f64, f64)> =
                vec![(base, l.layer_height, l.first_layer_height)];
            for e in &l.by_nozzle {
                known.push((
                    e.diameter,
                    e.layer_height,
                    e.first_layer_height.unwrap_or(l.first_layer_height),
                ));
            }
            known.sort_by(|a, b| a.0.total_cmp(&b.0));
            let (lh, flh) = match known.iter().find(|k| same_diameter(k.0, d)) {
                Some(k) => (k.1, k.2),
                None => {
                    issues.push(
                        Issue::info("nozzle.custom")
                            .field("printer.nozzle")
                            .value(d),
                    );
                    interpolate(&known, d)
                }
            };
            p.layers.layer_height = lh;
            p.layers.first_layer_height = flh;
        }
        f.speed.max_volumetric = filament.speed.max_volumetric_for(d, p.nozzle.high_flow);
        if f.filament.abrasive && !p.nozzle.abrasive {
            issues.push(Issue::warning("nozzle.not_abrasive").field("printer.nozzle"));
        }
        (p, f, issues)
    }
}

/// Hauteurs pour un diamètre libre : interpolation linéaire entre les deux
/// buses connues qui l'encadrent, proportionnelle au-delà.
fn interpolate(known: &[(f64, f64, f64)], d: f64) -> (f64, f64) {
    let (first, last) = (known[0], known[known.len() - 1]);
    if d <= first.0 {
        return (
            first.1 * d / first.0,
            first.2.min(first.2 * d / first.0).max(0.1 * d),
        );
    }
    if d >= last.0 {
        return (last.1 * d / last.0, last.2);
    }
    let k = known
        .windows(2)
        .find(|w| d >= w[0].0 && d <= w[1].0)
        .expect("encadré");
    let t = (d - k[0].0) / (k[1].0 - k[0].0);
    (
        k[0].1 + (k[1].1 - k[0].1) * t,
        k[0].2 + (k[1].2 - k[0].2) * t,
    )
}

/// Parcours prêt à placer : rotation, copies, adhésion. Messages des copies
/// (dégagement de la tête) et de l'adhésion.
pub fn arrange(path: &Toolpath, setup: &Setup, printer: &PrinterProfile) -> (Toolpath, Vec<Issue>) {
    let mut issues = Vec::new();
    let mut out = path.clone();
    let p = &setup.placement;

    // Rotation autour du centre de l'encombrement.
    if p.rotation != 0.0 {
        if let Some(b) = path.bounds() {
            let c = b.center();
            let (s, co) = libm::sincos(p.rotation.to_radians());
            let turn = |x: f64, y: f64| {
                let (dx, dy) = (x - c[0], y - c[1]);
                (c[0] + dx * co - dy * s, c[1] + dx * s + dy * co)
            };
            for w in &mut out.points {
                (w.x, w.y) = turn(w.x, w.y);
            }
            for n in &mut out.notes {
                (n.x, n.y) = turn(n.x, n.y);
            }
        }
    }

    // Copies en grille, pièce par pièce, rangée après rangée en serpentin.
    let [nx, ny] = p.copies;
    if nx * ny > 1 {
        if let Some(b) = out.bounds() {
            let size = b.size();
            let n = &printer.nozzle;
            let spacing = match (p.spacing, n.clearance_radius) {
                (Some(s), Some(r)) if s < r => {
                    issues.push(
                        Issue::error("copies.too_close")
                            .field("placement.spacing")
                            .value(r),
                    );
                    s
                }
                (Some(s), _) => s,
                (None, Some(r)) => r,
                (None, None) => 10.0,
            };
            match n.clearance_height {
                Some(h) if size[2] + b.min[2] > h => issues.push(
                    Issue::error("copies.too_tall")
                        .field("printer.nozzle.clearance_height")
                        .value(h),
                ),
                None => {
                    issues.push(Issue::error("copies.clearance_unknown").field("printer.nozzle"))
                }
                _ => {}
            }
            let pitch = [size[0] + spacing, size[1] + spacing];
            let one = out.clone();
            out.points.clear();
            out.notes.clear();
            out.starts.clear();
            for j in 0..ny {
                for k in 0..nx {
                    let i = if j % 2 == 0 { k } else { nx - 1 - k };
                    let (dx, dy) = (
                        (i as f64 - (nx - 1) as f64 / 2.0) * pitch[0],
                        (j as f64 - (ny - 1) as f64 / 2.0) * pitch[1],
                    );
                    out.starts.push(out.points.len());
                    out.points.extend(one.points.iter().map(|w| {
                        let mut w = *w;
                        w.x += dx;
                        w.y += dy;
                        w
                    }));
                    if out.notes.is_empty() {
                        out.notes.extend(one.notes.iter().map(|n| {
                            let mut n = n.clone();
                            n.x += dx;
                            n.y += dy;
                            n
                        }));
                    }
                }
            }
            // Chaque copie commence par un déplacement.
            for &s in &out.starts {
                out.points[s].kind = MoveKind::Travel;
            }
        }
    }

    // Adhésion : empreinte de la première couche de toutes les copies.
    if setup.adhesion != Adhesion::None {
        let min_z = out
            .points
            .iter()
            .skip(1)
            .filter(|w| w.kind != MoveKind::Travel)
            .map(|w| w.z)
            .fold(f64::INFINITY, f64::min);
        if min_z.is_finite() {
            let flh = printer.layers.first_layer_height;
            // Comme le plan : première couche sous Z min + sa hauteur (à 1 %
            // près, pour exclure la couche suivante posée pile au-dessus).
            let beads = adhesion::first_layer_beads(&out, min_z + flh * 0.99);
            let w = printer.nozzle.diameter * 1.125;
            let added = match setup.adhesion {
                Adhesion::Brim { width, gap } => {
                    adhesion::brim(&beads, width, gap, w, min_z).map(|b| (b, 0.0))
                }
                Adhesion::Raft {
                    layers,
                    gap,
                    margin,
                } => {
                    let mut heights = vec![flh];
                    heights.extend(std::iter::repeat_n(
                        printer.layers.layer_height,
                        layers as usize - 1,
                    ));
                    adhesion::raft(&beads, margin, printer.nozzle.diameter * 1.25, &heights)
                        .map(|(r, top)| (r, top + gap))
                }
                Adhesion::None => unreachable!(),
            };
            match added {
                Some((mut pts, lift)) if !pts.is_empty() => {
                    for w in &mut out.points {
                        w.z += lift;
                    }
                    for n in &mut out.notes {
                        n.z += lift;
                    }
                    let shift = pts.len();
                    for s in &mut out.starts {
                        *s += shift;
                    }
                    if out.starts.is_empty() {
                        out.starts.push(shift);
                    }
                    out.points[0].kind = MoveKind::Travel;
                    pts.append(&mut out.points);
                    out.points = pts;
                }
                _ => issues.push(Issue::warning("adhesion.skipped")),
            }
        }
    }
    (out, issues)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::{Speed, WayPoint};
    use crate::testing::{filament, printer};

    fn wp(x: f64, y: f64, z: f64, kind: MoveKind) -> WayPoint {
        WayPoint {
            x,
            y,
            z,
            w: 0.45,
            h: 0.2,
            kind,
            speed: Speed::Factor(1.0),
            fan: None,
            flow: 1.0,
        }
    }

    /// Carré de 20 mm sur `layers` couches.
    fn square(layers: usize) -> Toolpath {
        let mut points = vec![wp(-10.0, -10.0, 0.2, MoveKind::Travel)];
        for k in 0..layers {
            let z = 0.2 * (k + 1) as f64;
            for (x, y) in [(10.0, -10.0), (10.0, 10.0), (-10.0, 10.0), (-10.0, -10.0)] {
                points.push(wp(x, y, z, MoveKind::Print));
            }
        }
        Toolpath {
            points,
            ..Default::default()
        }
    }

    #[test]
    fn nozzle_choice_changes_layers_and_flow() {
        let (pr, fi) = (printer(), filament());
        let mut s = Setup::default();
        let (p, f, issues) = s.effective(&pr, &fi);
        assert!(issues.is_empty());
        assert_eq!(
            (
                p.nozzle.diameter,
                p.layers.layer_height,
                f.speed.max_volumetric
            ),
            (0.4, 0.2, 15.0)
        );

        // Buse du profil : couches de PrusaSlicer, débit du filament pour 0,8 mm.
        s.nozzle.diameter = Some(0.8);
        let (p, f, issues) = s.effective(&pr, &fi);
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!((p.nozzle.diameter, p.layers.layer_height), (0.8, 0.4));
        assert_eq!(f.speed.max_volumetric, 19.0);
        // Haut débit 0,6 : entrée la plus précise du filament.
        s.nozzle.diameter = Some(0.6);
        s.nozzle.high_flow = Some(true);
        let (p, f, issues) = s.effective(&pr, &fi);
        assert!(issues.is_empty(), "{issues:?}");
        assert!(p.nozzle.high_flow);
        assert_eq!(f.speed.max_volumetric, 28.0);
        // Diamètre libre : couches interpolées, et dit.
        s.nozzle = NozzleSetup {
            diameter: Some(0.7),
            ..Default::default()
        };
        let (p, _, issues) = s.effective(&pr, &fi);
        assert_eq!(issues[0].code, "nozzle.custom");
        assert!(
            (p.layers.layer_height - 0.325).abs() < 1e-9,
            "{}",
            p.layers.layer_height
        );
        // Les profils livrés ne changent pas.
        assert_eq!(pr.nozzle.diameter, 0.4);
    }

    #[test]
    fn rotation_turns_about_the_footprint_centre() {
        let s = Setup {
            placement: Placement {
                rotation: 90.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let (out, issues) = arrange(&square(1), &s, &printer());
        assert!(issues.is_empty());
        let p = out.points[1];
        assert!(
            (p.x - 10.0).abs() < 1e-9 && (p.y - 10.0).abs() < 1e-9,
            "{p:?}"
        );
    }

    #[test]
    fn copies_are_spaced_by_the_head_clearance_and_limited_in_height() {
        let pr = printer(); // MK4S : rayon 45 mm, hauteur 13 mm
        let mut s = Setup::default();
        s.placement.copies = [2, 1];
        let (out, issues) = arrange(&square(5), &s, &pr);
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(out.points.len(), 2 * square(5).points.len());
        assert_eq!(out.starts, [0, square(5).points.len()]);
        // Pas de 20 + 45 mm, centrés.
        assert_eq!(out.points[0].x, -10.0 - 32.5);
        assert_eq!(out.points[out.starts[1]].x, -10.0 + 32.5);
        assert_eq!(out.points[out.starts[1]].kind, MoveKind::Travel);
        // Trop près, trop haut.
        s.placement.spacing = Some(10.0);
        assert_eq!(arrange(&square(5), &s, &pr).1[0].code, "copies.too_close");
        s.placement.spacing = None;
        assert_eq!(arrange(&square(70), &s, &pr).1[0].code, "copies.too_tall");
    }

    #[test]
    fn a_raft_lifts_the_part_and_a_brim_comes_first() {
        let pr = printer();
        let mut s = Setup {
            adhesion: Adhesion::Raft {
                layers: 3,
                gap: 0.2,
                margin: 3.0,
            },
            ..Default::default()
        };
        let part = square(2);
        let (out, issues) = arrange(&part, &s, &pr);
        assert!(issues.is_empty(), "{issues:?}");
        let start = out.starts[0];
        // Radeau : 0,2 + 0,2 + 0,2 mm, écart 0,2 mm → pièce rehaussée de 0,8 mm.
        assert!(
            (out.points[start].z - (0.2 + 0.8)).abs() < 1e-9,
            "{}",
            out.points[start].z
        );
        assert!(out.points[..start].iter().all(|p| p.z <= 0.6 + 1e-9));
        assert_eq!(out.points[start].kind, MoveKind::Travel);

        s.adhesion = Adhesion::Brim {
            width: 2.0,
            gap: 0.1,
        };
        let (out, _) = arrange(&part, &s, &pr);
        let start = out.starts[0];
        assert!(start > 0);
        assert!(out.points[..start].iter().all(|p| p.z == 0.2));
        assert_eq!(out.points[start].z, 0.2, "pièce non rehaussée");
        // Bordure autour du carré : au-delà de son bord, à 2,5 mm au plus.
        for p in &out.points[..start] {
            let d = p.x.abs().max(p.y.abs());
            assert!(d > 10.0 && d < 10.225 + 2.6, "{p:?}");
        }
    }

    #[test]
    fn values_are_checked_not_clamped() {
        let s: Setup = serde_json::from_value(serde_json::json!({
            "nozzle": { "diameter": 5.0 },
            "placement": { "copies": [20, 20], "rotation": 30 },
            "adhesion": { "type": "brim", "width": 0 }
        }))
        .unwrap();
        let codes: Vec<_> = s.check().iter().map(|i| i.code).collect();
        assert_eq!(
            codes,
            [
                "setup.out_of_range",
                "setup.too_many_copies",
                "setup.out_of_range"
            ]
        );
        // Valeurs par défaut des variantes.
        let s: Setup = serde_json::from_str(r#"{"adhesion": {"type": "raft"}}"#).unwrap();
        assert_eq!(
            s.adhesion,
            Adhesion::Raft {
                layers: 3,
                gap: 0.2,
                margin: 3.0
            }
        );
        assert!(serde_json::from_str::<Setup>(r#"{"placement": {"copise": [2, 1]}}"#).is_err());
    }
}
