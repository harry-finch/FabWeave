//! Physique du parcours : placement sur le plateau, extrusion (`E`), vitesse
//! obtenue sous les plafonds de la machine et du filament, ventilateur,
//! rétractions, temps et matière.
//!
//! Rien n'est corrigé en silence : chaque vitesse réduite est attribuée à son
//! plafond (première couche, machine, débit, axe Z) et comptée ; un parcours
//! hors du volume d'impression est une erreur.

use serde::Serialize;

use crate::issue::Issue;
use crate::path::{Bounds, MoveKind, Speed, Toolpath};
use crate::profile::{FilamentProfile, Origin, PrinterProfile};

/// Longueur sous laquelle deux points sont confondus (mm).
pub const MIN_SEGMENT: f64 = 1e-4;

/// Plafond qui a fixé la vitesse d'un mouvement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SpeedLimit {
    /// Vitesse demandée par le modèle.
    None = 0,
    FirstLayer = 1,
    Machine = 2,
    /// Débit volumétrique maximal du filament.
    Flow = 3,
    /// Vitesse maximale de l'axe Z.
    ZAxis = 4,
}

/// Mouvement vers le point `i` depuis le point `i - 1` (le mouvement 0 est
/// le déplacement initial vers le premier point).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Move {
    /// Longueur 3D (mm).
    pub length: f64,
    /// Vitesse obtenue (mm/s).
    pub speed: f64,
    /// Vitesse demandée par le modèle (mm/s).
    pub requested: f64,
    /// Filament poussé (mm), 0 pour un déplacement.
    pub e: f64,
    /// Débit volumétrique obtenu (mm³/s).
    pub flow: f64,
    /// Ventilateur (%).
    pub fan: u8,
    pub limit: SpeedLimit,
    /// Déplacement avec rétraction et saut en Z.
    pub retract: bool,
    /// Point confondu avec le précédent : rien n'est écrit.
    pub skipped: bool,
    /// Sur la première couche.
    pub first_layer: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct LimitCounts {
    pub first_layer: usize,
    pub machine: usize,
    pub flow: usize,
    pub z_axis: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Stats {
    pub points: usize,
    pub print_moves: usize,
    pub travel_moves: usize,
    pub retractions: usize,
    pub skipped: usize,
    /// Longueur imprimée (mm).
    pub print_length: f64,
    pub travel_length: f64,
    /// Durée estimée sans accélérations (s).
    pub time: f64,
    /// Filament (mm, cm³, g).
    pub filament_length: f64,
    pub filament_volume: f64,
    pub filament_mass: f64,
    /// Débit volumétrique maximal obtenu et demandé (mm³/s).
    pub max_flow: f64,
    pub max_requested_flow: f64,
    pub max_speed: f64,
    /// Encombrement sur le plateau (repère machine).
    pub bounds: Bounds,
    /// Encombrement de la première couche (repère machine).
    pub first_layer_bounds: Bounds,
    pub limited: LimitCounts,
}

pub struct Plan {
    /// Translation du repère du modèle vers celui de la machine.
    pub offset: [f64; 3],
    pub moves: Vec<Move>,
    pub stats: Stats,
    pub issues: Vec<Issue>,
    /// Z sous lequel un point appartient à la première couche (repère machine).
    pub first_layer_top: f64,
}

/// Section d'un boudin W × H : stade (bords arrondis), symétrique en W et H.
pub fn bead_area(w: f64, h: f64) -> f64 {
    let (a, b) = if w < h { (w, h) } else { (h, w) };
    (b - a) * a + std::f64::consts::PI * a * a / 4.0
}

fn grow(b: &mut Option<Bounds>, p: [f64; 3]) {
    match b {
        None => *b = Some(Bounds { min: p, max: p }),
        Some(b) => {
            for ((lo, hi), v) in b.min.iter_mut().zip(b.max.iter_mut()).zip(p) {
                *lo = lo.min(v);
                *hi = hi.max(v);
            }
        }
    }
}

/// Calcule le plan d'impression. `None` si le parcours ne peut pas être
/// imprimé (les messages disent pourquoi).
pub fn plan(
    path: &Toolpath,
    printer: &PrinterProfile,
    filament: &FilamentProfile,
) -> (Option<Plan>, Vec<Issue>) {
    let mut issues = Vec::new();
    let Some(model_bounds) = path.bounds().filter(|_| path.extruding_moves() > 0) else {
        return (None, vec![Issue::error("path.empty")]);
    };

    // Placement : centre de l'encombrement au centre du plateau.
    let c = model_bounds.center();
    let [bx, by] = printer.bed.size;
    let offset = match printer.bed.origin {
        Origin::Corner => [bx / 2.0 - c[0], by / 2.0 - c[1], 0.0],
        Origin::Center => [-c[0], -c[1], 0.0],
    };
    let (lo, hi) = match printer.bed.origin {
        Origin::Corner => ([0.0, 0.0], [bx, by]),
        Origin::Center => ([-bx / 2.0, -by / 2.0], [bx / 2.0, by / 2.0]),
    };
    let overflow = (0..2)
        .map(|i| {
            let a = lo[i] - (model_bounds.min[i] + offset[i]);
            let b = model_bounds.max[i] + offset[i] - hi[i];
            a.max(b)
        })
        .fold(0.0_f64, f64::max);
    if overflow > 1e-6 {
        issues.push(
            Issue::error("path.out_of_bed")
                .field("printer.bed")
                .value(overflow),
        );
    }
    if model_bounds.min[2] < -1e-9 {
        issues.push(Issue::error("path.below_bed").value(model_bounds.min[2]));
    }
    if model_bounds.max[2] > printer.bed.max_height {
        issues.push(
            Issue::error("path.too_high")
                .field("printer.bed")
                .value(model_bounds.max[2]),
        );
    }
    if crate::issue::has_error(&issues) {
        return (None, issues);
    }

    let pts = &path.points;
    let min_print_z = pts
        .iter()
        .skip(1)
        .filter(|p| p.kind != MoveKind::Travel)
        .map(|p| p.z)
        .fold(f64::INFINITY, f64::min);
    let first_layer_top = min_print_z + printer.layers.first_layer_height;

    let f_area = filament.area();
    let flow_mult = filament.filament.flow_multiplier;
    let lim = &printer.limits;
    let sp = &filament.speed;
    let r = &filament.retraction;

    let mut moves = Vec::with_capacity(pts.len());
    let mut limited = LimitCounts::default();
    let (mut print_moves, mut travel_moves, mut retractions, mut skipped) = (0, 0, 0, 0);
    let (mut print_length, mut travel_length, mut time, mut e_total) = (0.0, 0.0, 0.0, 0.0);
    let (mut max_flow, mut max_req_flow, mut max_speed) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut fl_bounds: Option<Bounds> = None;
    let retract_time = if r.length > 0.0 {
        2.0 * r.length / r.speed
    } else {
        0.0
    };
    let hop_time = 2.0 * r.z_hop / lim.max_z_speed;

    // Déplacement initial vers le premier point (toujours avec rétraction).
    moves.push(Move {
        length: 0.0,
        speed: lim.travel_speed,
        requested: lim.travel_speed,
        e: 0.0,
        flow: 0.0,
        fan: 0,
        limit: SpeedLimit::None,
        retract: true,
        skipped: false,
        first_layer: pts[0].z < first_layer_top,
    });

    for i in 1..pts.len() {
        let (a, b) = (&pts[i - 1], &pts[i]);
        let (dx, dy, dz) = (b.x - a.x, b.y - a.y, b.z - a.z);
        let length = (dx * dx + dy * dy + dz * dz).sqrt();
        let first_layer = b.z < first_layer_top;
        let mut m = Move {
            length,
            speed: 0.0,
            requested: 0.0,
            e: 0.0,
            flow: 0.0,
            fan: 0,
            limit: SpeedLimit::None,
            retract: false,
            skipped: false,
            first_layer,
        };
        if length < MIN_SEGMENT {
            m.skipped = true;
            skipped += 1;
            moves.push(m);
            continue;
        }
        if b.kind == MoveKind::Travel {
            let xy = dx.hypot(dy);
            m.retract = xy >= r.min_travel && (r.length > 0.0 || r.z_hop > 0.0);
            m.speed = lim.travel_speed;
            m.requested = lim.travel_speed;
            travel_moves += 1;
            travel_length += length;
            time += xy / lim.travel_speed + dz.abs() / lim.max_z_speed;
            if m.retract {
                retractions += 1;
                time += retract_time + hop_time;
            }
            moves.push(m);
            continue;
        }

        let area = bead_area(b.w, b.h);
        let mult = flow_mult * b.flow;
        let requested = match b.speed {
            Speed::Absolute(v) => v,
            Speed::Factor(f) => sp.nominal * f,
        };
        let mut v = requested;
        let mut limit = SpeedLimit::None;
        if first_layer && v > sp.first_layer {
            v = sp.first_layer;
            limit = SpeedLimit::FirstLayer;
        }
        if v > lim.max_print_speed {
            v = lim.max_print_speed;
            limit = SpeedLimit::Machine;
        }
        let req_flow = area * mult * requested;
        max_req_flow = max_req_flow.max(req_flow);
        if area * mult * v > sp.max_volumetric {
            v = sp.max_volumetric / (area * mult);
            limit = SpeedLimit::Flow;
        }
        let z_ratio = dz.abs() / length;
        if v * z_ratio > lim.max_z_speed {
            v = lim.max_z_speed / z_ratio;
            limit = SpeedLimit::ZAxis;
        }
        match limit {
            SpeedLimit::None => {}
            SpeedLimit::FirstLayer => limited.first_layer += 1,
            SpeedLimit::Machine => limited.machine += 1,
            SpeedLimit::Flow => limited.flow += 1,
            SpeedLimit::ZAxis => limited.z_axis += 1,
        }

        let default_fan = if first_layer {
            filament.cooling.first_layer_fan
        } else {
            filament.cooling.fan
        };
        let mut fan = b.fan.map_or(default_fan, f64::from);
        if b.kind == MoveKind::Bridge {
            fan = fan.max(filament.cooling.bridge_fan);
        }

        m.e = area * length * mult / f_area;
        m.speed = v;
        m.requested = requested;
        m.flow = area * mult * v;
        m.fan = fan.round().clamp(0.0, 100.0) as u8;
        m.limit = limit;
        print_moves += 1;
        print_length += length;
        e_total += m.e;
        time += length / v;
        max_flow = max_flow.max(m.flow);
        max_speed = max_speed.max(v);
        if first_layer {
            grow(&mut fl_bounds, [a.x + offset[0], a.y + offset[1], a.z]);
            grow(&mut fl_bounds, [b.x + offset[0], b.y + offset[1], b.z]);
        }
        moves.push(m);
    }

    let bed = |v: [f64; 3]| [v[0] + offset[0], v[1] + offset[1], v[2] + offset[2]];
    let bounds = Bounds {
        min: bed(model_bounds.min),
        max: bed(model_bounds.max),
    };
    let filament_volume = e_total * f_area / 1000.0;
    let stats = Stats {
        points: pts.len(),
        print_moves,
        travel_moves,
        retractions,
        skipped,
        print_length,
        travel_length,
        time,
        filament_length: e_total,
        filament_volume,
        filament_mass: filament_volume * filament.filament.density,
        max_flow,
        max_requested_flow: max_req_flow,
        max_speed,
        bounds,
        first_layer_bounds: fl_bounds.unwrap_or(bounds),
        limited,
    };

    if print_moves == 0 {
        // Tous les points imprimés étaient confondus (division entière dans le
        // script, pas trop petit…).
        issues.push(Issue::error("path.empty").count(skipped));
        return (None, issues);
    }

    let l = &stats.limited;
    if l.first_layer > 0 {
        issues.push(
            Issue::info("speed.limited_first_layer")
                .count(l.first_layer)
                .value(sp.first_layer),
        );
    }
    if l.machine > 0 {
        issues.push(
            Issue::info("speed.limited_machine")
                .count(l.machine)
                .value(lim.max_print_speed),
        );
    }
    if l.flow > 0 {
        issues.push(
            Issue::info("speed.limited_flow")
                .count(l.flow)
                .value(stats.max_requested_flow),
        );
    }
    if l.z_axis > 0 {
        issues.push(
            Issue::info("speed.limited_z")
                .count(l.z_axis)
                .value(lim.max_z_speed),
        );
    }
    if skipped > 0 {
        // Au-delà d'un point sur dix, c'est probablement une erreur du script.
        let issue = if skipped * 10 >= pts.len() {
            Issue::warning("path.duplicates_skipped")
        } else {
            Issue::info("path.duplicates_skipped")
        };
        issues.push(issue.count(skipped));
    }
    (
        Some(Plan {
            offset,
            moves,
            stats,
            issues: issues.clone(),
            first_layer_top,
        }),
        issues,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::WayPoint;
    use crate::testing::{filament, printer};

    fn wp(x: f64, y: f64, z: f64, kind: MoveKind, speed: Speed) -> WayPoint {
        WayPoint {
            x,
            y,
            z,
            w: 0.45,
            h: 0.2,
            kind,
            speed,
            fan: None,
            flow: 1.0,
        }
    }

    #[test]
    fn stadium_area() {
        let a = bead_area(0.45, 0.2);
        assert!((a - (0.25 * 0.2 + std::f64::consts::PI * 0.01)).abs() < 1e-12);
        assert_eq!(bead_area(0.2, 0.45), a);
    }

    #[test]
    fn centres_on_the_bed_and_computes_extrusion() {
        let p = Toolpath {
            points: vec![
                wp(-10.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
                wp(10.0, 0.0, 0.2, MoveKind::Print, Speed::Absolute(20.0)),
            ],
        };
        let (plan, issues) = plan(&p, &printer(), &filament());
        let plan = plan.unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(plan.offset, [125.0, 105.0, 0.0]);
        let m = plan.moves[1];
        let expected_e = bead_area(0.45, 0.2) * 20.0 / filament().area();
        assert!((m.e - expected_e).abs() < 1e-12);
        assert_eq!(m.speed, 20.0);
        assert!((plan.stats.time - 1.0).abs() < 1e-9);
    }

    #[test]
    fn speed_limits_are_counted_not_hidden() {
        let pts = vec![
            wp(0.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
            // Première couche : plafonnée à 20 mm/s.
            wp(10.0, 0.0, 0.2, MoveKind::Print, Speed::Absolute(100.0)),
            wp(10.0, 0.0, 1.0, MoveKind::Travel, Speed::Factor(1.0)),
            // Débit : 0,0814 mm² × 500 mm/s > 15 mm³/s.
            wp(20.0, 0.0, 1.0, MoveKind::Print, Speed::Absolute(500.0)),
            // Axe Z : montée presque verticale.
            wp(20.0, 0.1, 6.0, MoveKind::Print, Speed::Absolute(50.0)),
        ];
        let (plan, issues) = plan(&Toolpath { points: pts }, &printer(), &filament());
        let plan = plan.unwrap();
        assert_eq!(plan.moves[1].limit, SpeedLimit::FirstLayer);
        assert!(!plan.moves[2].retract);
        assert_eq!(plan.moves[3].limit, SpeedLimit::Flow);
        assert!((plan.moves[3].flow - 15.0).abs() < 1e-9);
        assert_eq!(plan.moves[4].limit, SpeedLimit::ZAxis);
        let codes: Vec<_> = issues.iter().map(|i| i.code).collect();
        assert_eq!(
            codes,
            [
                "speed.limited_first_layer",
                "speed.limited_flow",
                "speed.limited_z"
            ]
        );
    }

    #[test]
    fn out_of_bed_and_empty_paths_are_errors() {
        let big = Toolpath {
            points: vec![
                wp(-200.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
                wp(200.0, 0.0, 0.2, MoveKind::Print, Speed::Factor(1.0)),
            ],
        };
        let (plan_, issues) = plan(&big, &printer(), &filament());
        assert!(plan_.is_none());
        assert_eq!(issues[0].code, "path.out_of_bed");
        assert!((issues[0].value.unwrap() - 75.0).abs() < 1e-9);

        let same = wp(1.0, 1.0, 0.2, MoveKind::Print, Speed::Factor(1.0));
        let (none, issues) = plan(
            &Toolpath {
                points: vec![same, same, same],
            },
            &printer(),
            &filament(),
        );
        assert!(none.is_none());
        assert_eq!(issues[0].code, "path.empty");
        assert_eq!(issues[0].count, Some(2));

        let travel_only = Toolpath {
            points: vec![wp(0.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0))],
        };
        assert_eq!(
            plan(&travel_only, &printer(), &filament()).1[0].code,
            "path.empty"
        );
    }
}
