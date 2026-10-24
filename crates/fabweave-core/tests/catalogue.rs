//! Chaque modèle du catalogue (`models/**/*.weave`) se lit, se calcule aux
//! valeurs par défaut sans message bloquant sur les profils livrés, donne un
//! G-code déterministe, et tient le calcul aux bornes de chaque curseur.

use std::path::{Path, PathBuf};

use fabweave_core::model::ParamKind;
use fabweave_core::Workbench;
use serde_json::{json, Map, Value};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn weave_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            weave_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "weave") {
            out.push(p);
        }
    }
}

fn catalogue() -> Vec<PathBuf> {
    let mut v = Vec::new();
    weave_files(&root().join("models"), &mut v);
    v.sort();
    assert!(!v.is_empty());
    v
}

fn bench(printer: &str, filament: &str) -> Workbench {
    let mut w = Workbench::default();
    let read = |p: &str| std::fs::read_to_string(root().join(p)).unwrap();
    w.set_printer(&read(printer)).unwrap();
    w.set_filament(&read(filament)).unwrap();
    w
}

#[test]
fn every_model_prints_with_default_values() {
    for printer in [
        "profiles/printers/prusa_mk4s.toml",
        "profiles/printers/prusa_core_one.toml",
    ] {
        for filament in [
            "profiles/filaments/generic_pla.toml",
            "profiles/filaments/generic_petg.toml",
        ] {
            let mut w = bench(printer, filament);
            for file in catalogue() {
                let src = std::fs::read_to_string(&file).unwrap();
                w.load_model(&src)
                    .unwrap_or_else(|e| panic!("{}: {e:?}", file.display()));
                let ev = w.evaluate(&Map::new());
                let errors: Vec<_> = ev.issues.iter().filter(|i| i.is_error()).collect();
                assert!(
                    errors.is_empty(),
                    "{} / {printer} / {filament}: {errors:?}",
                    file.display()
                );
                let a = w.gcode(&ev).unwrap();
                let b = w.gcode(&w.evaluate(&Map::new())).unwrap();
                assert_eq!(a, b, "{} : G-code non déterministe", file.display());
            }
        }
    }
}

#[test]
fn every_model_survives_its_parameter_bounds() {
    let mut w = bench(
        "profiles/printers/prusa_core_one.toml",
        "profiles/filaments/generic_pla.toml",
    );
    for file in catalogue() {
        let src = std::fs::read_to_string(&file).unwrap();
        let model = w.load_model(&src).unwrap().clone();
        for (id, def) in &model.params {
            let ends: Vec<Value> = match def.kind {
                ParamKind::Float { min, max, .. } => vec![json!(min), json!(max)],
                ParamKind::Int { min, max, .. } => vec![json!(min), json!(max)],
                ParamKind::Bool { .. } => vec![json!(false), json!(true)],
            };
            for v in ends {
                let mut values = Map::new();
                values.insert(id.clone(), v.clone());
                let ev = w.evaluate(&values);
                // Les seules erreurs admises aux bornes : pièce trop grande pour le plateau.
                for i in ev.issues.iter().filter(|i| i.is_error()) {
                    assert!(
                        matches!(i.code, "path.out_of_bed" | "path.too_high"),
                        "{} {id} = {v}: {i:?}",
                        file.display()
                    );
                }
            }
        }
    }
}

#[test]
fn ripple_vase_matches_the_fullcontrol_design() {
    let mut w = bench(
        "profiles/printers/prusa_mk4s.toml",
        "profiles/filaments/generic_pla.toml",
    );
    let src = std::fs::read_to_string(root().join("models/vases/ripple_vase.weave")).unwrap();
    w.load_model(&src).unwrap();
    let ev = w.evaluate(&Map::new());
    let path = ev.path.as_ref().unwrap();
    // 40 / 0,24 = 166 tours de (50 + 0,5) × 2 points.
    assert_eq!(path.points.len(), 16766);
    let plan = ev.plan.as_ref().unwrap();
    let s = &plan.stats;
    // Premier tour à 0,8 × 0,24 mm ; dernier point un pas avant 166 tours.
    assert!((s.bounds.min[2] - 0.192).abs() < 1e-9);
    assert!((s.bounds.max[2] - (0.192 + 16765.0 / 101.0 * 0.24)).abs() < 1e-9);
    // Centré sur le plateau de la MK4S.
    let c = [
        (s.bounds.min[0] + s.bounds.max[0]) / 2.0,
        (s.bounds.min[1] + s.bounds.max[1]) / 2.0,
    ];
    assert!((c[0] - 125.0).abs() < 1e-9 && (c[1] - 105.0).abs() < 1e-9);
    // Aucun déplacement, aucune rétraction : une seule spirale.
    assert_eq!(s.travel_moves, 0);
    assert_eq!(s.retractions, 0);
}
