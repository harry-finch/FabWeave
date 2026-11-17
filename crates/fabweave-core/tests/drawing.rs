//! Paramètre `image` (R-025) : un dessin percé laisse son trou vide dans le
//! parcours, se refuse avec un code stable, et revient à l'identique par le
//! projet.

use std::path::Path;

use fabweave_core::project::Project;
use fabweave_core::{MoveKind, Workbench};
use serde_json::{json, Map, Value};

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

const SQUARE_HOLE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><path d="M10,10 H90 V90 H10 Z M50 30 A20 20 0 1 0 50 70 A20 20 0 1 0 50 30 Z"/></svg>"#;

fn coaster() -> Workbench {
    let mut w = Workbench::default();
    w.set_printer(&read("../../profiles/printers/prusa_mk4s.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_pla.toml"))
        .unwrap();
    w.load_model(&read("../../models/maison/logo_coaster.weave"))
        .unwrap();
    w
}

fn values(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn the_hole_of_a_drawing_stays_empty() {
    let w = coaster();
    let ev = w.evaluate(&values(
        json!({ "logo": SQUARE_HOLE, "color_change": false }),
    ));
    assert!(!ev.issues.iter().any(|i| i.is_error()), "{:?}", ev.issues);
    let path = ev.path.unwrap();
    // Logo de 60 mm : carré de 80 sur 100 → 60 mm ; trou de rayon 20 → 15 mm.
    // Au-dessus du disque, aucun point imprimé à moins de 15 − boudin du centre.
    let disc_top = 0.2 + 9.0 * 0.2 + 1e-6;
    let inside = path
        .points
        .iter()
        .filter(|p| p.kind == MoveKind::Print && p.z > disc_top)
        .filter(|p| p.x.hypot(p.y) < 15.0 - 0.5)
        .count();
    assert_eq!(inside, 0);
    let ring = path
        .points
        .iter()
        .filter(|p| p.kind == MoveKind::Print && p.z > disc_top)
        .map(|p| p.x.hypot(p.y))
        .fold(f64::INFINITY, f64::min);
    assert!((ring - 15.0).abs() < 0.1, "{ring}");
}

#[test]
fn bad_drawings_are_refused_with_a_code() {
    let w = coaster();
    let code = |v: Value| {
        let ev = w.evaluate(&values(json!({ "logo": v })));
        ev.issues
            .iter()
            .find(|i| i.is_error())
            .map(|i| (i.code, i.field.clone()))
    };
    let field = Some("params.logo".to_string());
    assert_eq!(
        code(json!("pas un dessin")),
        Some(("image.invalid", field.clone()))
    );
    assert_eq!(
        code(json!("<svg></svg>")),
        Some(("image.empty", field.clone()))
    );
    assert_eq!(code(json!(42)), Some(("param.wrong_type", field)));
}

#[test]
fn a_drawing_survives_the_project() {
    let w = coaster();
    let ev = w.evaluate(&values(json!({ "logo": SQUARE_HOLE })));
    let g = w.gcode(&ev).unwrap();
    let p = Project::read(g.as_bytes()).unwrap();
    assert_eq!(p.values["logo"], json!(SQUARE_HOLE));
    let mut w2 = Workbench::default();
    w2.load_model(&p.model).unwrap();
    w2.set_printer(&p.printer).unwrap();
    w2.set_filament(&p.filament).unwrap();
    let v2: Map<String, Value> = p.values.into_iter().collect();
    assert_eq!(w2.gcode(&w2.evaluate(&v2)).unwrap(), g);
}
