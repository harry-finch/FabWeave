//! Contrôles de qualité du parcours (SAFE-04, SAFE-05) : ce qui s'imprime mal
//! sans rien casser. Rien n'est modifié ni bloqué : chaque contrôle est un
//! message compté, à code stable.
//!
//! - **segments** : trop courts pour la résolution de la machine, ou trop
//!   nombreux par seconde pour son planificateur (la buse « bégaie ») ;
//! - **porte-à-faux** : boudin qui dépasse trop du précédent, vu sur une carte
//!   des hauteurs ; les ponts (`bridge_to`) sont voulus et n'en sont pas ;
//! - **ponts** trop longs, **largeur** et **hauteur** incohérentes avec la
//!   buse, **première couche** trop fine ou décollée du plateau.
//!
//! Les seuils sont des règles d'atelier, sans valeur dans les profils tant
//! qu'aucune machine n'en demande une autre.

use crate::guard::HeightMap;
use crate::issue::Issue;
use crate::path::{MoveKind, Toolpath};
use crate::plan::{Plan, MIN_SEGMENT};
use crate::profile::PrinterProfile;

/// Segment trop court (mm) : quelques pas des moteurs.
const SHORT_SEGMENT: f64 = 0.05;
/// Mouvements par seconde que le planificateur d'un firmware courant tient.
const MAX_SEGMENT_RATE: f64 = 500.0;
/// Durée d'une rafale pour qu'elle compte (s) : le tampon du planificateur
/// absorbe les plus courtes.
const RATE_WINDOW: f64 = 1.0;
/// Angle au-delà duquel un boudin est en porte-à-faux (degrés, depuis la
/// verticale).
const OVERHANG_ANGLE: f64 = 60.0;
/// Tracé en porte-à-faux sous lequel on ne dit rien : mm, et part du tracé
/// imprimé (quelques mm isolés sur un grand vase ne gênent pas).
const OVERHANG_MIN_LENGTH: f64 = 5.0;
const OVERHANG_MIN_SHARE: f64 = 0.001;
/// Pont le plus long sans message (mm).
const LONG_BRIDGE: f64 = 50.0;
/// Largeur admise, en diamètres de buse.
const WIDTH_RANGE: (f64, f64) = (0.5, 2.5);
/// Hauteur la plus grande admise, en diamètres de buse : au-delà, le boudin
/// ne touche plus la buse (des modèles de FullControl vont jusqu'à 1).
const MAX_HEIGHT: f64 = 1.0 + 1e-9;
/// Hauteur de première couche la plus faible, en hauteur du profil.
const MIN_FIRST_LAYER: f64 = 0.5;
/// Écart au plateau toléré sous le premier boudin, en hauteurs de boudin.
const FIRST_LAYER_GAP: f64 = 0.5;

pub fn check(path: &Toolpath, plan: &Plan, printer: &PrinterProfile) -> Vec<Issue> {
    let mut issues = Vec::new();
    segments(plan, &mut issues);
    beads(path, plan, printer, &mut issues);
    bridges(path, &mut issues);
    overhangs(path, plan, &mut issues);
    issues
}

/// Segments trop courts ; débit de mouvements sur une seconde glissante.
fn segments(plan: &Plan, issues: &mut Vec<Issue>) {
    let moves = &plan.moves;
    let short = moves
        .iter()
        .filter(|m| !m.skipped && m.e > 0.0 && m.length >= MIN_SEGMENT && m.length < SHORT_SEGMENT)
        .count();
    if short > 0 {
        issues.push(
            Issue::info("quality.short_segments")
                .count(short)
                .value(SHORT_SEGMENT),
        );
    }
    let (mut start, mut elapsed, mut peak, mut over) = (0, 0.0, 0.0_f64, 0);
    for (i, m) in moves.iter().enumerate() {
        elapsed += m.time;
        while elapsed - moves[start].time >= RATE_WINDOW {
            elapsed -= moves[start].time;
            start += 1;
        }
        if elapsed >= RATE_WINDOW {
            let rate = (i + 1 - start) as f64 / elapsed;
            peak = peak.max(rate);
            if rate > MAX_SEGMENT_RATE {
                over += 1;
            }
        }
    }
    if over > 0 {
        issues.push(
            Issue::warning("quality.segment_rate")
                .count(over)
                .value(peak.round()),
        );
    }
}

/// Section du boudin par rapport à la buse, première couche.
fn beads(path: &Toolpath, plan: &Plan, printer: &PrinterProfile, issues: &mut Vec<Issue>) {
    let d = printer.nozzle.diameter;
    let first = printer.layers.first_layer_height;
    let (mut narrow, mut wide, mut tall, mut thin) = (0, 0, 0, 0);
    let mut lowest: Option<(f64, f64)> = None;
    for (p, m) in path.points.iter().zip(&plan.moves).skip(1) {
        if !matches!(p.kind, MoveKind::Print | MoveKind::Bridge) || m.skipped {
            continue;
        }
        narrow += usize::from(p.w < WIDTH_RANGE.0 * d);
        wide += usize::from(p.w > WIDTH_RANGE.1 * d);
        tall += usize::from(p.h > MAX_HEIGHT * d);
        if m.first_layer {
            thin += usize::from(p.h < MIN_FIRST_LAYER * first);
        }
        if lowest.is_none_or(|(z, _)| p.z < z) {
            lowest = Some((p.z, p.h));
        }
    }
    for (code, n) in [
        ("quality.width_narrow", narrow),
        ("quality.width_wide", wide),
        ("quality.height_tall", tall),
    ] {
        if n > 0 {
            issues.push(
                Issue::warning(code)
                    .field("printer.nozzle")
                    .count(n)
                    .value(d),
            );
        }
    }
    if thin > 0 {
        issues.push(
            Issue::warning("quality.first_layer_thin")
                .count(thin)
                .value(MIN_FIRST_LAYER * first),
        );
    }
    if let Some((z, h)) = lowest {
        if z > h * (1.0 + FIRST_LAYER_GAP) {
            issues.push(Issue::warning("quality.first_layer_gap").value(z - h));
        }
    }
}

/// Ponts plus longs que ce qu'un filament tient d'ordinaire : portée en ligne
/// droite depuis le dernier appui, pas la somme de ponts qui se suivent.
fn bridges(path: &Toolpath, issues: &mut Vec<Issue>) {
    let (mut anchor, mut span, mut longest, mut count) = (None, 0.0_f64, 0.0_f64, 0);
    let mut close = |span: &mut f64| {
        if *span > LONG_BRIDGE {
            count += 1;
            longest = longest.max(*span);
        }
        *span = 0.0;
    };
    for w in path.points.windows(2) {
        if w[1].kind != MoveKind::Bridge {
            close(&mut span);
            anchor = None;
            continue;
        }
        let (x, y) = *anchor.get_or_insert((w[0].x, w[0].y));
        span = span.max((w[1].x - x).hypot(w[1].y - y));
    }
    close(&mut span);
    if count > 0 {
        issues.push(
            Issue::warning("quality.long_bridge")
                .count(count)
                .value(longest),
        );
    }
}

/// Tracé imprimé sans matière sous lui, à la pente près.
fn overhangs(path: &Toolpath, plan: &Plan, issues: &mut Vec<Issue>) {
    let pts = &path.points;
    let Some(b) = path.bounds() else {
        return;
    };
    let mut map = HeightMap::new([b.min[0], b.min[1]], [b.max[0], b.max[1]]);
    let tan = OVERHANG_ANGLE.to_radians().tan();
    let mut length = 0.0;
    let (mut pending, mut run) = (0_usize, 0.0_f64);
    let mut laid = vec![0.0_f64; pts.len()];
    for i in 1..pts.len() {
        let (p, q) = (&pts[i - 1], &pts[i]);
        let d = (q.x - p.x).hypot(q.y - p.y).hypot(q.z - p.z);
        run += if q.kind.deposits() { d } else { 0.0 };
        laid[i] = run;
        let keep = if q.kind.deposits() { 2.0 * q.w } else { 0.0 };
        while pending + 1 < i && (!q.kind.deposits() || laid[i - 1] - laid[pending + 1] > keep) {
            map.stamp_segment(&pts[pending], &pts[pending + 1]);
            pending += 1;
        }
        if q.kind != MoveKind::Print || plan.moves[i].first_layer || plan.moves[i].skipped {
            continue;
        }
        // Le bas du boudin touche-t-il de la matière à moins de la portée ?
        let floor = q.z - 2.0 * q.h;
        if floor <= 0.0 {
            continue;
        }
        let reach = q.h * tan + map.cell();
        let mut held = false;
        map.scan(q.x, q.y, reach, floor, |_, _| held = true);
        if !held {
            length += d;
        }
    }
    if length >= OVERHANG_MIN_LENGTH.max(OVERHANG_MIN_SHARE * plan.stats.print_length) {
        issues.push(
            Issue::warning("quality.overhang")
                .count(length.round() as usize)
                .value(OVERHANG_ANGLE),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::{MK4S, PLA};
    use crate::Workbench;
    use serde_json::Map;

    fn codes(body: &str) -> Vec<&'static str> {
        let mut w = Workbench::default();
        w.set_printer(MK4S).unwrap();
        w.set_filament(PLA).unwrap();
        w.load_model(&format!(
            "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet path = new_path();\n{body}\npath\n"
        ))
        .unwrap();
        w.evaluate(&Map::new())
            .issues
            .iter()
            .filter(|i| i.code.starts_with("quality."))
            .map(|i| i.code)
            .collect()
    }

    /// Murs de `n` tours de 20 mm de rayon, chaque tour décalé de `lean` mm.
    fn cone(n: usize, lean: f64) -> String {
        format!(
            "path.travel_to(20.0, 0.0, 0.2);\n\
             for k in 0..{n} {{\n\
                 let r = 20.0 + k * {lean};\n\
                 path.print_points(circle(0, 0, 0.2 + k * 0.2, r, 0, 120), 0.45, 0.2);\n\
             }}\n"
        )
    }

    #[test]
    fn a_straight_wall_says_nothing() {
        assert!(codes(&cone(20, 0.0)).is_empty());
    }

    #[test]
    fn a_wall_leaning_too_far_is_an_overhang() {
        // 45° (0,2 mm par couche) passe ; 72° (0,6 mm) non.
        assert!(codes(&cone(20, 0.2)).is_empty());
        assert_eq!(codes(&cone(20, 0.6)), ["quality.overhang"]);
    }

    #[test]
    fn the_bead_is_compared_to_the_nozzle() {
        let line = |w: f64, h: f64| {
            format!("path.travel_to(0.0, 0.0, 0.2);\npath.print_to(20.0, 0.0, 0.2, {w}, {h});\n")
        };
        assert_eq!(codes(&line(0.1, 0.2)), ["quality.width_narrow"]);
        assert_eq!(codes(&line(1.5, 0.2)), ["quality.width_wide"]);
        assert!(codes(&line(0.45, 0.6)).contains(&"quality.height_tall"));
    }

    #[test]
    fn the_first_layer_must_be_thick_enough_and_on_the_bed() {
        let thin = "path.travel_to(0.0, 0.0, 0.05);\npath.print_to(20.0, 0.0, 0.05, 0.45, 0.05);";
        assert_eq!(codes(thin), ["quality.first_layer_thin"]);
        let high = "path.travel_to(0.0, 0.0, 1.0);\npath.print_to(20.0, 0.0, 1.0, 0.45, 0.2);";
        assert_eq!(codes(high), ["quality.first_layer_gap"]);
    }

    #[test]
    fn a_long_bridge_is_flagged() {
        let bridge = |l: f64| {
            format!(
                "path.travel_to(0.0, 0.0, 0.2);\n\
                 path.print_to(1.0, 0.0, 0.2, 0.45, 0.2);\n\
                 path.bridge_to({l}, 0.0, 0.2, 0.45, 0.2, 1.0);\n\
                 path.print_to({l} + 1.0, 0.0, 0.2, 0.45, 0.2);\n"
            )
        };
        assert!(codes(&bridge(30.0)).is_empty());
        assert_eq!(codes(&bridge(80.0)), ["quality.long_bridge"]);
    }

    #[test]
    fn many_tiny_segments_saturate_the_planner() {
        // 0,02 mm à 40 mm/s (première couche ralentie à 20) : 1 000 mouvements
        // par seconde pendant 2 s, trop courts aussi.
        let tiny = "path.travel_to(0.0, 0.0, 0.2);\n\
                    for k in 1..3000 { path.print_to(k * 0.02, 0.0, 0.2, 0.45, 0.2); }";
        let c = codes(tiny);
        assert!(c.contains(&"quality.short_segments"), "{c:?}");
        assert!(c.contains(&"quality.segment_rate"), "{c:?}");
    }
}
