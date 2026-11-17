//! Formes natives (R-023) : torsion mesurée sur le parcours, cage au repos
//! sans effet, refus à code stable.

use std::path::Path;

use fabweave_core::{Toolpath, Workbench};
use serde_json::Map;

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

/// Modèle d'essai : une colonne de rayon 20, la spirale par `revolve` avec
/// les réglages `extra` (table Rhai sans accolades).
fn model(extra: &str) -> String {
    format!(
        r#"version = 2
[meta]
name = "t"
category = "t"
[params.profile]
type = "curve"
label = "p"
default = [[20, 0], [20, 40]]
min = [5, 0]
max = [50, 100]
---
let shape = #{{ profile: params.profile, start: 0.2, pitch: 0.3, segments: 360{extra} }};
let wall = revolve(shape);
let path = new_path();
path.travel_to(wall);
path.print_points(wall, 0.6, 0.3);
path
"#
    )
}

fn run(src: &str) -> Result<Toolpath, Vec<&'static str>> {
    let mut w = Workbench::default();
    w.set_printer(&read("../../profiles/printers/prusa_mk4s.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_pla.toml"))
        .unwrap();
    w.load_model(src).unwrap();
    let ev = w.evaluate(&Map::new());
    let errors: Vec<_> = ev
        .issues
        .iter()
        .filter(|i| i.is_error())
        .map(|i| i.code)
        .collect();
    if errors.is_empty() {
        Ok(ev.path.unwrap())
    } else {
        Err(errors)
    }
}

/// Angle (degrés) du point de rayon maximal du dernier tour, un lobe d'une
/// section `drop` marquant l'orientation de la section.
fn last_turn_tip(p: &Toolpath) -> f64 {
    let pts = &p.points[p.points.len() - 360..];
    let tip = pts
        .iter()
        .max_by(|a, b| a.x.hypot(a.y).total_cmp(&b.x.hypot(b.y)))
        .unwrap();
    tip.y.atan2(tip.x).to_degrees()
}

#[test]
fn a_90_degree_twist_turns_the_top_by_90_degrees() {
    let plain = run(&model(", section: section(\"drop\", 3, 0.5)")).unwrap();
    let twisted = run(&model(", section: section(\"drop\", 3, 0.5), twist: 90")).unwrap();
    let d = (last_turn_tip(&twisted) - last_turn_tip(&plain) + 540.0).rem_euclid(360.0) - 180.0;
    // Une section par degré : la mesure se fait au point près.
    assert!((d - 90.0).abs() <= 1.0, "{d}");
}

#[test]
fn a_cage_at_rest_changes_nothing() {
    let without = run(&model(", section: section(\"star\", 5, 0.2)")).unwrap();
    let with = run(&model(
        ", section: section(\"star\", 5, 0.2), cage: cage(3, 6, [0,0,0,0,0,0, 0,0,0,0,0,0, 0,0,0,0,0,0], 0.6)",
    ))
    .unwrap();
    assert_eq!(without.points, with.points);
}

#[test]
fn bad_shapes_are_refused_with_a_code() {
    assert_eq!(
        run(&model(", section: section(\"blob\", 5, 0.2)")).unwrap_err(),
        ["shape.unknown_section"]
    );
    assert_eq!(
        run(&model(", twist: 90, twist_kind: \"spin\"")).unwrap_err(),
        ["shape.unknown_twist"]
    );
    assert_eq!(
        run(&model(", cage: cage(3, 6, [1, 2], 0.5)")).unwrap_err(),
        ["shape.cage_size"]
    );
    assert_eq!(
        run(&model(
            ", cage: cage(3, 3, [-30,-30,-30, -30,-30,-30, -30,-30,-30], 0)"
        ))
        .unwrap_err(),
        ["shape.radius_too_small"]
    );
}
