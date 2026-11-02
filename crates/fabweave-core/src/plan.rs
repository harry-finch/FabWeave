//! Physique du parcours : placement sur le plateau, extrusion (`E`), vitesse
//! obtenue sous les plafonds de la machine et du filament, ventilateur,
//! rétractions, temps et matière.
//!
//! Rien n'est corrigé en silence : chaque vitesse réduite est attribuée à son
//! plafond (première couche, machine, débit, axe Z) et comptée ; un parcours
//! hors du volume d'impression (plateau rectangulaire ou rond) ou dans une
//! zone exclue du plateau est une erreur. Le plan est tout de même calculé,
//! les mouvements en cause marqués (`Move::outside`) pour que la vue montre
//! où la pièce dépasse ; l'export, lui, est refusé.
//!
//! Temps estimé : avec les accélérations du profil, chaque mouvement suit un
//! trapèze de vitesse (accélération, palier, freinage) ; la vitesse de
//! passage d'un mouvement au suivant est limitée par le *jerk* dans les
//! angles, et par ce que l'accélération permet d'atteindre ou de freiner
//! (passes avant et arrière, comme le planificateur du firmware). Un
//! déplacement avec rétraction part et s'arrête à l'arrêt.

use serde::Serialize;

use crate::issue::Issue;
use crate::path::{Bounds, MoveKind, Speed, Toolpath};
use crate::profile::{Accelerations, BedShape, FilamentProfile, Origin, PrinterProfile, Zone};

/// Longueur sous laquelle deux points sont confondus (mm).
pub const MIN_SEGMENT: f64 = 1e-4;
/// Avance la plus lente écrite (mm/s) : `F1`, une goutte ne pousse pas le
/// filament plus lentement.
pub const MIN_FEED: f64 = 1.0 / 60.0;

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
    /// Vitesse obtenue (mm/s) ; pour une goutte, celle du filament poussé.
    pub speed: f64,
    /// Vitesse demandée par le modèle (mm/s).
    pub requested: f64,
    /// Filament poussé (mm), 0 pour un déplacement ; négatif pour un
    /// essuyage (rétraction répartie sur le mouvement).
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
    /// Durée estimée du mouvement (s), rétraction et saut compris
    /// (progression affichée par l'imprimante).
    pub time: f64,
    /// Mouvement imprimé hors du volume d'impression ou dans une zone exclue.
    pub outside: bool,
    /// La buse ou la tête y touche la matière déjà posée (garde-buse).
    pub collision: bool,
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
    /// Gouttes (extrusion à l'arrêt).
    pub blobs: usize,
    /// Pauses de l'imprimante (durée non comptée dans le temps estimé).
    pub pauses: usize,
    pub travel_moves: usize,
    pub retractions: usize,
    pub skipped: usize,
    /// Longueur imprimée (mm).
    pub print_length: f64,
    pub travel_length: f64,
    /// Durée estimée (s), avec les accélérations du profil s'il les donne.
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

/// Centre du plateau (repère machine).
pub fn bed_centre(printer: &PrinterProfile) -> [f64; 2] {
    let [bx, by] = printer.bed.size;
    match printer.bed.origin {
        Origin::Corner => [bx / 2.0, by / 2.0],
        Origin::Center => [0.0, 0.0],
    }
}

/// Calcule le plan d'impression, la pièce centrée sur le plateau. `None` si
/// le parcours ne peut pas être imprimé ni montré (les messages disent
/// pourquoi).
pub fn plan(
    path: &Toolpath,
    printer: &PrinterProfile,
    filament: &FilamentProfile,
) -> (Option<Plan>, Vec<Issue>) {
    plan_at(path, printer, filament, None)
}

/// Comme [`plan`], le centre de l'encombrement posé en `target` (repère
/// machine) plutôt qu'au centre du plateau.
pub fn plan_at(
    path: &Toolpath,
    printer: &PrinterProfile,
    filament: &FilamentProfile,
    target: Option<[f64; 2]>,
) -> (Option<Plan>, Vec<Issue>) {
    let mut issues = Vec::new();
    let Some(model_bounds) = path.bounds().filter(|_| path.extruding_moves() > 0) else {
        return (None, vec![Issue::error("path.empty")]);
    };

    // Placement : centre de l'encombrement au point demandé.
    let c = model_bounds.center();
    let centre = bed_centre(printer);
    let t = target.unwrap_or(centre);
    let offset = [t[0] - c[0], t[1] - c[1], 0.0];
    let [bx, by] = printer.bed.size;
    let (lo, hi) = (
        [centre[0] - bx / 2.0, centre[1] - by / 2.0],
        [centre[0] + bx / 2.0, centre[1] + by / 2.0],
    );
    let round = printer.bed.shape == BedShape::Circle;
    // Dépassement d'un point du plateau (mm, négatif dedans).
    let beyond = |x: f64, y: f64| {
        if round {
            (x - centre[0]).hypot(y - centre[1]) - bx / 2.0
        } else {
            (lo[0] - x).max(x - hi[0]).max(lo[1] - y).max(y - hi[1])
        }
    };
    let overflow = if round {
        path.points
            .iter()
            .map(|p| beyond(p.x + offset[0], p.y + offset[1]))
            .fold(0.0_f64, f64::max)
    } else {
        (0..2)
            .map(|i| {
                let a = lo[i] - (model_bounds.min[i] + offset[i]);
                let b = model_bounds.max[i] + offset[i] - hi[i];
                a.max(b)
            })
            .fold(0.0_f64, f64::max)
    };
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
    excluded_zones(path, offset, &printer.bed.exclude, &mut issues);
    let max_height = printer.bed.max_height;
    let zones: Vec<([f64; 2], [f64; 2])> = printer
        .bed
        .exclude
        .iter()
        .map(|z| {
            (
                [z.min[0] + 1e-6, z.min[1] + 1e-6],
                [z.max[0] - 1e-6, z.max[1] - 1e-6],
            )
        })
        .collect();
    // Mouvement imprimé hors du volume ou dans une zone (repère machine).
    let off_limits = |a: &crate::path::WayPoint, b: &crate::path::WayPoint| {
        let (pa, pb) = (
            [a.x + offset[0], a.y + offset[1]],
            [b.x + offset[0], b.y + offset[1]],
        );
        beyond(pa[0], pa[1]) > 1e-6
            || beyond(pb[0], pb[1]) > 1e-6
            || a.z.min(b.z) < -1e-9
            || a.z.max(b.z) > max_height
            || zones
                .iter()
                .any(|(lo, hi)| segment_hits_box(pa, pb, *lo, *hi))
    };

    let pts = &path.points;
    let min_print_z = pts
        .iter()
        .skip(1)
        .filter(|p| p.kind.deposits())
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
    let (mut blobs, mut slowed_blobs, mut pauses) = (0, 0, 0);
    // Longueur de chaque suite d'essuyages, pour y répartir la rétraction.
    let mut wipe_run = vec![0.0_f64; pts.len()];
    let mut i = 1;
    while i < pts.len() {
        if pts[i].kind != MoveKind::Wipe {
            i += 1;
            continue;
        }
        let first = i;
        let mut total = 0.0;
        while i < pts.len() && pts[i].kind == MoveKind::Wipe {
            let (a, b) = (&pts[i - 1], &pts[i]);
            total += (b.x - a.x).hypot(b.y - a.y);
            i += 1;
        }
        wipe_run[first..i].fill(total);
    }
    let (mut print_length, mut travel_length, mut time, mut e_total) = (0.0, 0.0, 0.0, 0.0);
    let (mut max_flow, mut max_req_flow, mut max_speed) = (0.0_f64, 0.0_f64, 0.0_f64);
    let mut fl_bounds: Option<Bounds> = None;
    // Rétraction d'un déplacement : celle d'essai du point (modèle de
    // calibration) ou celle du filament ; durée de la rétraction et du saut.
    let retraction_at = |i: usize| crate::path::Retraction::resolve(pts[i].retraction, r);
    let stop_time = |i: usize| {
        let (len, speed, hop) = retraction_at(i);
        let retract = if len > 0.0 { 2.0 * len / speed } else { 0.0 };
        retract + 2.0 * hop / lim.max_z_speed
    };

    // Sauts en Z au-dessus du volume (déplacement initial compris).
    let mut hop_over = f64::NEG_INFINITY;
    let first_hop = pts[0].z + r.z_hop.max(0.2);
    if first_hop > max_height {
        hop_over = first_hop;
    }
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
        time: 0.0,
        outside: first_hop > max_height,
        collision: false,
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
            time: 0.0,
            outside: false,
            collision: false,
        };
        if b.kind == MoveKind::Pause {
            // Pause : rien ne bouge, durée inconnue (non comptée).
            pauses += 1;
            moves.push(m);
            continue;
        }
        if b.kind == MoveKind::Wipe && length >= MIN_SEGMENT {
            // Essuyage : sur le dernier tracé, sans extruder ; la rétraction
            // du filament est répartie sur toute la suite d'essuyages, et la
            // vitesse limitée pour que le filament ne recule pas plus vite
            // que la vitesse de rétraction.
            let xy = dx.hypot(dy);
            let run = wipe_run[i];
            let mut v = lim.travel_speed;
            let (len, speed, _) = retraction_at(i);
            if len > 0.0 && run > 0.0 {
                v = v.min(speed * run / len);
                m.e = -len * xy / run;
            }
            m.speed = v;
            m.requested = v;
            m.time = length / v;
            travel_moves += 1;
            travel_length += length;
            time += m.time;
            moves.push(m);
            continue;
        }
        if b.kind == MoveKind::Blob {
            // Goutte : `volume` en `duration`, débit plafonné par le filament
            // (la goutte dure alors plus longtemps), filament poussé à F1 au
            // moins (résolution de la commande).
            let mult = flow_mult * b.flow;
            let req_flow = b.volume * mult / b.duration;
            max_req_flow = max_req_flow.max(req_flow);
            let mut duration = b.duration;
            if req_flow > sp.max_volumetric {
                duration = b.volume * mult / sp.max_volumetric;
                m.limit = SpeedLimit::Flow;
                limited.flow += 1;
            }
            m.e = b.volume * mult / f_area;
            m.speed = m.e / duration;
            if m.speed < MIN_FEED {
                m.speed = MIN_FEED;
                duration = m.e / MIN_FEED;
                slowed_blobs += 1;
            }
            m.requested = m.e / b.duration;
            m.flow = b.volume * mult / duration;
            let default_fan = if first_layer {
                filament.cooling.first_layer_fan
            } else {
                filament.cooling.fan
            };
            m.fan = b
                .fan
                .map_or(default_fan, f64::from)
                .round()
                .clamp(0.0, 100.0) as u8;
            m.outside = off_limits(a, b);
            m.time = duration;
            time += duration;
            e_total += m.e;
            max_flow = max_flow.max(m.flow);
            blobs += 1;
            if first_layer {
                grow(&mut fl_bounds, [b.x + offset[0], b.y + offset[1], b.z]);
            }
            moves.push(m);
            continue;
        }
        if length < MIN_SEGMENT {
            m.skipped = true;
            skipped += 1;
            moves.push(m);
            continue;
        }
        if b.kind == MoveKind::Travel {
            let xy = dx.hypot(dy);
            let (len, _, hop) = retraction_at(i);
            m.retract = xy >= r.min_travel && (len > 0.0 || hop > 0.0);
            // Le saut monte au-dessus des deux bouts (comme le G-code).
            let top = a.z.max(b.z) + if m.retract { hop } else { 0.0 };
            if top > max_height {
                m.outside = true;
                hop_over = hop_over.max(top);
            }
            m.speed = lim.travel_speed;
            m.requested = lim.travel_speed;
            travel_moves += 1;
            travel_length += length;
            m.time = xy / lim.travel_speed + dz.abs() / lim.max_z_speed;
            if m.retract {
                retractions += 1;
                m.time += stop_time(i);
            }
            time += m.time;
            moves.push(m);
            continue;
        }

        let area = bead_area(b.w, b.h);
        let bridge = if b.kind == MoveKind::Bridge {
            filament.filament.bridge_flow
        } else {
            1.0
        };
        let mult = flow_mult * b.flow * bridge;
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

        m.outside = off_limits(a, b);
        m.e = area * length * mult / f_area;
        m.speed = v;
        m.requested = requested;
        m.flow = area * mult * v;
        m.fan = fan.round().clamp(0.0, 100.0) as u8;
        m.limit = limit;
        print_moves += 1;
        print_length += length;
        e_total += m.e;
        m.time = length / v;
        time += m.time;
        max_flow = max_flow.max(m.flow);
        max_speed = max_speed.max(v);
        if first_layer {
            grow(&mut fl_bounds, [a.x + offset[0], a.y + offset[1], a.z]);
            grow(&mut fl_bounds, [b.x + offset[0], b.y + offset[1], b.z]);
        }
        moves.push(m);
    }

    if let Some(acc) = printer.limits.accelerations() {
        time = accelerated_times(pts, &mut moves, &acc, &stop_time, lim.max_z_speed);
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
        blobs,
        pauses,
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

    temperatures(path, &moves, printer, filament, &mut issues);
    if hop_over > max_height {
        issues.push(
            Issue::error("path.hop_too_high")
                .field("printer.bed")
                .value(hop_over),
        );
    }

    if print_moves + blobs == 0 {
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
    if slowed_blobs > 0 {
        issues.push(
            Issue::warning("blob.too_slow")
                .count(slowed_blobs)
                .value(MIN_FEED * 60.0),
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

/// Consignes de buse et de plateau sous les plafonds de l'imprimante :
/// chaque température du filament (gabarits compris), plus le décalage
/// d'essai des modèles d'étalonnage.
fn temperatures(
    path: &Toolpath,
    moves: &[Move],
    printer: &PrinterProfile,
    filament: &FilamentProfile,
    issues: &mut Vec<Issue>,
) {
    let t = &filament.temperature;
    let [nozzle_cap, bed_cap] = printer.limits.max_temperatures();
    let mut nozzle = [t.first_layer, t.nozzle, t.probe, t.idle.unwrap_or(0.0)]
        .into_iter()
        .fold(f64::NEG_INFINITY, f64::max);
    for (p, m) in path.points.iter().zip(moves) {
        if p.kind.deposits() {
            let base = if m.first_layer {
                t.first_layer
            } else {
                t.nozzle
            };
            nozzle = nozzle.max(base + p.temp_offset);
        }
    }
    let bed = t.bed.max(t.bed_first_layer);
    if nozzle > nozzle_cap {
        issues.push(
            Issue::error("temperature.nozzle_above_machine")
                .field("printer.limits.max_nozzle_temperature")
                .value(nozzle),
        );
    }
    if bed > bed_cap {
        issues.push(
            Issue::error("temperature.bed_above_machine")
                .field("printer.limits.max_bed_temperature")
                .value(bed),
        );
    }
    let l = &printer.limits;
    if l.max_nozzle_temperature.is_none() || l.max_bed_temperature.is_none() {
        issues.push(Issue::info("temperature.limit_unknown").field("printer.limits"));
    }
}

/// Erreur `path.excluded_zone` pour chaque zone du plateau traversée par un
/// mouvement imprimé (repère machine, bords de la zone tolérés).
fn excluded_zones(path: &Toolpath, offset: [f64; 3], zones: &[Zone], issues: &mut Vec<Issue>) {
    for (i, z) in zones.iter().enumerate() {
        let (lo, hi) = (
            [z.min[0] + 1e-6, z.min[1] + 1e-6],
            [z.max[0] - 1e-6, z.max[1] - 1e-6],
        );
        let hits = path
            .points
            .windows(2)
            .filter(|w| w[1].kind.deposits())
            .filter(|w| {
                let a = [w[0].x + offset[0], w[0].y + offset[1]];
                let b = [w[1].x + offset[0], w[1].y + offset[1]];
                segment_hits_box(a, b, lo, hi)
            })
            .count();
        if hits > 0 {
            issues.push(
                Issue::error("path.excluded_zone")
                    .field(format!("printer.bed.exclude[{i}]"))
                    .count(hits)
                    .name(z.name.clone()),
            );
        }
    }
}

/// Le segment `a`–`b` passe-t-il dans le rectangle ? (Liang–Barsky.)
fn segment_hits_box(a: [f64; 2], b: [f64; 2], lo: [f64; 2], hi: [f64; 2]) -> bool {
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for k in 0..2 {
        let d = b[k] - a[k];
        if d.abs() < 1e-12 {
            if a[k] < lo[k] || a[k] > hi[k] {
                return false;
            }
            continue;
        }
        let (mut u, mut v) = ((lo[k] - a[k]) / d, (hi[k] - a[k]) / d);
        if u > v {
            std::mem::swap(&mut u, &mut v);
        }
        t0 = t0.max(u);
        t1 = t1.min(v);
        if t0 > t1 {
            return false;
        }
    }
    true
}

/// Temps des mouvements avec accélérations (voir l'en-tête du module) ;
/// renvoie la durée totale. `stop_time` : rétraction et saut en Z d'un
/// déplacement rétracté.
fn accelerated_times(
    pts: &[crate::path::WayPoint],
    moves: &mut [Move],
    acc: &Accelerations,
    stop_time: &dyn Fn(usize) -> f64,
    max_z_speed: f64,
) -> f64 {
    let n = moves.len();
    // Mouvements « fluides » : ni ignorés, ni rétractés (ceux-ci s'arrêtent).
    let accel = |i: usize| {
        let m = &moves[i];
        if pts[i].kind == MoveKind::Travel {
            acc.travel
        } else if m.first_layer {
            acc.first_layer
        } else {
            acc.print
        }
    };
    let dir = |i: usize| {
        let (a, b) = (&pts[i - 1], &pts[i]);
        let l = moves[i].length;
        [(b.x - a.x) / l, (b.y - a.y) / l, (b.z - a.z) / l]
    };
    // Arrêt complet : déplacement rétracté ou goutte.
    let stops = |i: usize| {
        moves[i].retract
            || matches!(
                pts[i].kind,
                MoveKind::Blob | MoveKind::Wipe | MoveKind::Pause
            )
    };
    let flows = |i: usize| i > 0 && !moves[i].skipped && !stops(i);
    // Vitesse de fin de chaque mouvement (= début du suivant fluide).
    let mut exit = vec![0.0_f64; n];
    let mut prev: Option<usize> = None;
    for i in 1..n {
        if !flows(i) {
            if stops(i) {
                if let Some(p) = prev {
                    exit[p] = 0.0;
                }
                prev = None;
            }
            continue;
        }
        if let Some(p) = prev {
            let (u, w) = (dir(p), dir(i));
            let v = moves[p].speed.min(moves[i].speed);
            let dv =
                v * ((u[0] - w[0]).powi(2) + (u[1] - w[1]).powi(2) + (u[2] - w[2]).powi(2)).sqrt();
            exit[p] = if dv > acc.jerk { v * acc.jerk / dv } else { v };
        }
        prev = Some(i);
    }
    if let Some(p) = prev {
        exit[p] = 0.0;
    }
    // Entrée de chaque mouvement fluide = sortie du précédent fluide.
    let mut entry = vec![0.0_f64; n];
    let mut last: Option<usize> = None;
    for (i, e) in entry.iter_mut().enumerate().skip(1) {
        if stops(i) {
            last = None;
        } else if flows(i) {
            *e = last.map_or(0.0, |p| exit[p]);
            last = Some(i);
        }
    }
    // Passe arrière : freiner à temps ; passe avant : accélérer au plus.
    let mut next_entry = 0.0;
    for i in (1..n).rev() {
        if stops(i) {
            next_entry = 0.0;
        } else if flows(i) {
            exit[i] = exit[i].min(next_entry);
            entry[i] = entry[i].min((exit[i].powi(2) + 2.0 * accel(i) * moves[i].length).sqrt());
            next_entry = entry[i];
        }
    }
    let mut prev_exit = 0.0;
    for i in 1..n {
        if stops(i) {
            prev_exit = 0.0;
        } else if flows(i) {
            entry[i] = entry[i].min(prev_exit);
            exit[i] = exit[i].min((entry[i].powi(2) + 2.0 * accel(i) * moves[i].length).sqrt());
            prev_exit = exit[i];
        }
    }
    let mut times = vec![0.0_f64; n];
    for i in 1..n {
        let m = &moves[i];
        if m.skipped {
            continue;
        }
        times[i] = if matches!(
            pts[i].kind,
            MoveKind::Blob | MoveKind::Wipe | MoveKind::Pause
        ) {
            m.time
        } else if m.retract {
            // Déplacement à l'arrêt aux deux bouts, puis rétraction et saut.
            let (a, b) = (&pts[i - 1], &pts[i]);
            let xy = (b.x - a.x).hypot(b.y - a.y);
            trapezoid(xy, 0.0, 0.0, m.speed, acc.travel)
                + (b.z - a.z).abs() / max_z_speed
                + stop_time(i)
        } else {
            trapezoid(m.length, entry[i], exit[i], m.speed, accel(i))
        };
    }
    for (m, t) in moves.iter_mut().zip(&times).skip(1) {
        m.time = *t;
    }
    times.iter().sum()
}

/// Durée d'un mouvement de longueur `l` entré à `ve`, sorti à `vx`, de
/// vitesse de croisière `v` et d'accélération `a`.
fn trapezoid(l: f64, ve: f64, vx: f64, v: f64, a: f64) -> f64 {
    if l <= 0.0 {
        return 0.0;
    }
    let (ve, vx) = (ve.min(v), vx.min(v));
    let (da, dd) = ((v * v - ve * ve) / (2.0 * a), (v * v - vx * vx) / (2.0 * a));
    if da + dd <= l {
        (v - ve) / a + (v - vx) / a + (l - da - dd) / v
    } else {
        // Pas de palier : pic de vitesse atteint en accélérant puis freinant.
        let vp = ((2.0 * a * l + ve * ve + vx * vx) / 2.0).sqrt();
        (vp - ve) / a + (vp - vx) / a
    }
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
            volume: 0.0,
            duration: 0.0,
            temp_offset: 0.0,
            retraction: None,
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
            ..Default::default()
        };
        let (plan, issues) = plan(&p, &printer(), &filament());
        let plan = plan.unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert_eq!(plan.offset, [125.0, 105.0, 0.0]);
        let m = plan.moves[1];
        let expected_e = bead_area(0.45, 0.2) * 20.0 / filament().area();
        assert!((m.e - expected_e).abs() < 1e-12);
        assert_eq!(m.speed, 20.0);
        // 20 mm à 20 mm/s, première couche à 500 mm/s² : 0,4 mm pour
        // accélérer, autant pour freiner (0,04 s chacun), 19,2 mm de palier.
        assert!((plan.stats.time - 1.04).abs() < 1e-9, "{}", plan.stats.time);
        // Sans accélérations dans le profil : longueur / vitesse.
        let mut pr = printer();
        pr.limits.acceleration = None;
        pr.limits.travel_acceleration = None;
        pr.limits.first_layer_acceleration = None;
        let (plan, _) = super::plan(&p, &pr, &filament());
        assert!((plan.unwrap().stats.time - 1.0).abs() < 1e-9);
    }

    #[test]
    fn trapezoids_and_corners() {
        // Palier atteint, ou pic de vitesse sur un mouvement court.
        assert!((trapezoid(100.0, 0.0, 0.0, 100.0, 1000.0) - 1.1).abs() < 1e-12);
        let short = trapezoid(1.0, 0.0, 0.0, 100.0, 1000.0);
        assert!((short - 2.0 * (1.0_f64 / 1000.0).sqrt()).abs() < 1e-12);
        assert_eq!(trapezoid(10.0, 50.0, 50.0, 50.0, 1000.0), 0.2);

        // Ligne droite découpée en 100 segments : aussi rapide qu'un seul trait ;
        // zigzag à angles droits : ralenti à chaque angle.
        let line: Vec<WayPoint> =
            std::iter::once(wp(0.0, 0.0, 0.4, MoveKind::Travel, Speed::Factor(1.0)))
                .chain(
                    (1..=100)
                        .map(|i| wp(i as f64, 0.0, 0.4, MoveKind::Print, Speed::Absolute(40.0))),
                )
                .collect();
        let zig: Vec<WayPoint> =
            std::iter::once(wp(0.0, 0.0, 0.4, MoveKind::Travel, Speed::Factor(1.0)))
                .chain((1..=100).map(|i| {
                    let y = if i % 2 == 0 { 0.0 } else { 1.0 };
                    wp(
                        ((i + 1) / 2) as f64,
                        y,
                        0.4,
                        MoveKind::Print,
                        Speed::Absolute(40.0),
                    )
                }))
                .collect();
        // Première couche : 20 mm/s, 500 mm/s².
        let pr = printer();
        let time = |pts: Vec<WayPoint>| {
            let t = Toolpath {
                points: pts,
                ..Default::default()
            };
            super::plan(&t, &pr, &filament()).0.unwrap().stats.time
        };
        let (t_line, t_zig) = (time(line), time(zig));
        assert!(
            (t_line - (100.0 / 20.0 + 20.0 / 500.0)).abs() < 1e-9,
            "{t_line}"
        );
        // Zigzag : 50 mm d'avance, ~120 mm de trajet, chaque angle à 8 mm/s ou moins.
        let length = 50.0 + 50.0 * 2.0_f64.sqrt();
        assert!(t_zig > length / 20.0 * 1.2, "{t_zig}");
    }

    #[test]
    fn excluded_zones_stop_the_print() {
        let mut pr = printer();
        // Zone de 10 mm au centre du plateau (le carré y est centré).
        pr.bed.exclude = vec![Zone {
            name: crate::model::Text::Plain("pince".into()),
            min: [120.0, 100.0],
            max: [130.0, 110.0],
        }];
        let square = |half: f64| Toolpath {
            points: vec![
                wp(-half, -half, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
                wp(half, -half, 0.2, MoveKind::Print, Speed::Factor(1.0)),
                wp(half, half, 0.2, MoveKind::Print, Speed::Factor(1.0)),
                wp(-half, half, 0.2, MoveKind::Print, Speed::Factor(1.0)),
                wp(-half, -half, 0.2, MoveKind::Print, Speed::Factor(1.0)),
            ],
            ..Default::default()
        };
        // Contour de 30 mm autour de la zone : il ne la touche pas.
        assert!(super::plan(&square(15.0), &pr, &filament()).0.is_some());
        // Contour de 8 mm : ses quatre côtés sont dans la zone.
        let (shown, issues) = super::plan(&square(4.0), &pr, &filament());
        // Plan gardé pour la vue, mouvements en cause marqués.
        assert!(shown.unwrap().moves[1..].iter().all(|m| m.outside));
        assert_eq!(issues[0].code, "path.excluded_zone");
        assert_eq!(issues[0].count, Some(4));
        assert_eq!(issues[0].field.as_deref(), Some("printer.bed.exclude[0]"));
        // Un trait qui la traverse sans s'y arrêter compte aussi ; un bord
        // effleuré non.
        assert!(segment_hits_box(
            [0.0, 105.0],
            [200.0, 105.0],
            [120.0, 100.0],
            [130.0, 110.0]
        ));
        assert!(!segment_hits_box(
            [0.0, 110.0],
            [200.0, 110.0],
            [120.0, 100.0],
            [129.999, 109.999]
        ));
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
        let (plan, issues) = plan(
            &Toolpath {
                points: pts,
                ..Default::default()
            },
            &printer(),
            &filament(),
        );
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
            ..Default::default()
        };
        let (shown, issues) = plan(&big, &printer(), &filament());
        assert!(shown.unwrap().moves[1].outside);
        assert_eq!(issues[0].code, "path.out_of_bed");
        assert!((issues[0].value.unwrap() - 75.0).abs() < 1e-9);

        // Placement demandé, plateau rond.
        let small = Toolpath {
            points: vec![
                wp(-10.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
                wp(10.0, 0.0, 0.2, MoveKind::Print, Speed::Factor(1.0)),
            ],
            ..Default::default()
        };
        let (at, issues) = plan_at(&small, &printer(), &filament(), Some([20.0, 30.0]));
        assert!(!crate::issue::has_error(&issues));
        assert_eq!(at.unwrap().offset, [20.0, 30.0, 0.0]);
        let (edge, issues) = plan_at(&small, &printer(), &filament(), Some([5.0, 30.0]));
        assert_eq!(issues[0].code, "path.out_of_bed");
        assert!(edge.unwrap().moves[1].outside);
        let mut delta = printer();
        delta.bed.shape = crate::profile::BedShape::Circle;
        delta.bed.size = [200.0, 200.0];
        delta.bed.origin = Origin::Center;
        // Diagonale de ±75 mm : ses bouts sont hors du disque de rayon 100
        // (75 √2 ≈ 106), alors qu'elle tiendrait dans un plateau carré.
        let corner = Toolpath {
            points: vec![
                wp(-75.0, -75.0, 0.2, MoveKind::Travel, Speed::Factor(1.0)),
                wp(75.0, 75.0, 0.2, MoveKind::Print, Speed::Factor(1.0)),
            ],
            ..Default::default()
        };
        let (_, issues) = plan(&corner, &delta, &filament());
        assert_eq!(issues[0].code, "path.out_of_bed");
        assert!((issues[0].value.unwrap() - (75.0 * 2.0_f64.sqrt() - 100.0)).abs() < 1e-9);
        assert!(!crate::issue::has_error(
            &plan(&small, &delta, &filament()).1
        ));

        let same = wp(1.0, 1.0, 0.2, MoveKind::Print, Speed::Factor(1.0));
        let (none, issues) = plan(
            &Toolpath {
                points: vec![same, same, same],
                ..Default::default()
            },
            &printer(),
            &filament(),
        );
        assert!(none.is_none());
        assert_eq!(issues[0].code, "path.empty");
        assert_eq!(issues[0].count, Some(2));

        let travel_only = Toolpath {
            points: vec![wp(0.0, 0.0, 0.2, MoveKind::Travel, Speed::Factor(1.0))],
            ..Default::default()
        };
        assert_eq!(
            plan(&travel_only, &printer(), &filament()).1[0].code,
            "path.empty"
        );
    }
}
