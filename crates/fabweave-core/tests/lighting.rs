//! Luminaires (R-022) : trous de douille aux cotes de l'utilisateur (E27 :
//! 40 mm, E14 : 30 mm) et emboîtements qui diffèrent exactement du jeu.

use std::path::Path;

use fabweave_core::{MoveKind, Toolpath, Workbench};
use serde_json::{json, Map, Value};

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

fn run(model: &str, values: Value) -> Toolpath {
    let mut w = Workbench::default();
    w.set_printer(&read("../../profiles/printers/prusa_core_one.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_petg.toml"))
        .unwrap();
    w.load_model(&read(&format!("../../models/luminaires/{model}.weave")))
        .unwrap();
    let values: Map<String, Value> = values.as_object().unwrap().clone();
    let ev = w.evaluate(&values);
    assert!(
        !ev.issues.iter().any(|i| i.is_error()),
        "{model} : {:?}",
        ev.issues
    );
    w.gcode(&ev).unwrap();
    ev.path.unwrap()
}

/// Bord de matière le plus proche du centre, sur la première couche :
/// centre du boudin moins sa demi-largeur.
fn inner_edge(p: &Toolpath) -> f64 {
    let z0 = p
        .points
        .iter()
        .find(|q| q.kind == MoveKind::Print)
        .unwrap()
        .z;
    p.points
        .iter()
        .filter(|q| q.kind == MoveKind::Print && (q.z - z0).abs() < 1e-9)
        .map(|q| q.x.hypot(q.y) - q.w / 2.0)
        .fold(f64::INFINITY, f64::min)
}

/// Bord extérieur de la paroi à la hauteur `z` (centre plus demi-largeur).
fn outer_edge_at(p: &Toolpath, z: f64) -> f64 {
    p.points
        .iter()
        .filter(|q| q.kind == MoveKind::Print && (q.z - z).abs() < 0.3)
        .map(|q| q.x.hypot(q.y) + q.w / 2.0)
        .fold(0.0, f64::max)
}

#[test]
fn socket_holes_have_the_given_diameters() {
    for (socket, d) in [("e27", 40.0), ("e14", 30.0)] {
        for (model, part) in [
            ("pendant", json!({})),
            ("table_lamp", json!({ "part": "base" })),
            ("wall_sconce", json!({ "part": "plate" })),
        ] {
            let mut v = part.clone();
            v["socket"] = json!(socket);
            if model != "pendant" && socket == "e14" {
                continue; // même calcul que E27, trou plus petit
            }
            let hole = 2.0 * inner_edge(&run(model, v));
            assert!((hole - d).abs() <= 0.05, "{model} {socket} : {hole}");
        }
    }
}

#[test]
fn fittings_differ_by_the_clearance() {
    for clearance in [0.2, 0.45] {
        // Lampe à poser : face extérieure du collier contre bord intérieur de l'anneau.
        let base = run(
            "table_lamp",
            json!({ "part": "base", "clearance": clearance }),
        );
        let shade = run(
            "table_lamp",
            json!({ "part": "shade", "clearance": clearance }),
        );
        let collar = outer_edge_at(&base, 0.2 + 9.0 * 0.2 + 5.0);
        let ring = inner_edge(&shade);
        assert!((ring - collar - clearance).abs() < 0.01, "{collar} {ring}");
        // Applique : collier de la platine contre manchon du diffuseur.
        let plate = run(
            "wall_sconce",
            json!({ "part": "plate", "clearance": clearance }),
        );
        let diffuser = run(
            "wall_sconce",
            json!({ "part": "diffuser", "clearance": clearance }),
        );
        let top = plate.points.iter().map(|q| q.z).fold(0.0, f64::max);
        let collar = outer_edge_at(&plate, top - 2.0);
        let sleeve = inner_edge(&diffuser);
        assert!(
            (sleeve - collar - clearance).abs() < 0.01,
            "{collar} {sleeve}"
        );
    }
}
