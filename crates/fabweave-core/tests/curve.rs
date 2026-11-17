//! Paramètre `curve` (R-020) : valeurs contrôlées sans correction, courbe lue
//! par le script, enregistrée dans le projet et rouverte à l'identique.

use std::path::Path;

use fabweave_core::project::Project;
use fabweave_core::Workbench;
use serde_json::{json, Map, Value};

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

fn bench() -> Workbench {
    let mut w = Workbench::default();
    w.load_model(&read("../../models/vases/free_silhouette_vase.weave"))
        .unwrap();
    w.set_printer(&read("../../profiles/printers/prusa_mk4s.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_pla.toml"))
        .unwrap();
    w
}

fn values(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn the_vase_follows_the_curve() {
    let w = bench();
    let ev = w.evaluate(&values(
        json!({ "silhouette": [[20, 0], [20, 30], [40, 60]], "bottom_layers": 0 }),
    ));
    assert!(!ev.issues.iter().any(|i| i.is_error()), "{:?}", ev.issues);
    let path = ev.path.unwrap();
    // Rayon du parcours (au centre du boudin) : 20 mm en bas, 40 mm en haut.
    let r = |p: &fabweave_core::WayPoint| p.x.hypot(p.y);
    let low = path.points.iter().find(|p| p.z > 10.0).unwrap();
    assert!((r(low) - 20.0).abs() < 0.01, "{}", r(low));
    let top = path.points.last().unwrap();
    assert!(top.z > 59.0 && top.z <= 60.0);
    assert!((r(top) - 40.0).abs() < 0.2, "{}", r(top));
}

#[test]
fn bad_curves_are_refused_not_corrected() {
    let w = bench();
    let code = |v: Value| {
        let ev = w.evaluate(&values(json!({ "silhouette": v })));
        ev.issues
            .iter()
            .find(|i| i.is_error())
            .map(|i| (i.code, i.field.clone()))
    };
    let field = Some("params.silhouette".to_string());
    assert_eq!(
        code(json!([[20, 0], [200, 60]])),
        Some(("param.out_of_range", field.clone()))
    );
    assert_eq!(
        code(json!([[20, 0], [30, 60], [30, 40]])),
        Some(("param.curve_not_increasing", field.clone()))
    );
    assert_eq!(
        code(json!([[20, 0]])),
        Some(("param.curve_point_count", field.clone()))
    );
    assert_eq!(code(json!("cylindre")), Some(("param.wrong_type", field)));
}

#[test]
fn a_curve_survives_the_project() {
    let w = bench();
    let given = values(json!({ "silhouette": [[25, 0], [35, 40], [15, 90]] }));
    let ev = w.evaluate(&given);
    let g = w.gcode(&ev).unwrap();
    let p = Project::read(g.as_bytes()).unwrap();
    assert_eq!(
        p.values["silhouette"],
        json!([[25.0, 0.0], [35.0, 40.0], [15.0, 90.0]])
    );
    let mut w2 = Workbench::default();
    w2.load_model(&p.model).unwrap();
    w2.set_printer(&p.printer).unwrap();
    w2.set_filament(&p.filament).unwrap();
    let v2: Map<String, Value> = p.values.into_iter().collect();
    assert_eq!(w2.gcode(&w2.evaluate(&v2)).unwrap(), g);
}
