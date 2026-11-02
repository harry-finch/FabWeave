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
        "profiles/printers/bambu_x1e.toml",
        "profiles/printers/marlin_generic.toml",
        "profiles/printers/klipper_generic.toml",
        "profiles/printers/bambu_a1.toml",
        "profiles/printers/bambu_p1s.toml",
    ] {
        // Bambu A1 et P1S : PLA seulement (mêmes dialecte et gabarits que la
        // X1E, déjà essayée en PETG) ; le catalogue est long à calculer.
        let petg = !printer.contains("bambu_a1") && !printer.contains("bambu_p1s");
        for filament in [
            "profiles/filaments/generic_pla.toml",
            "profiles/filaments/generic_petg.toml",
        ]
        .into_iter()
        .filter(|f| petg || f.contains("pla"))
        {
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
fn every_model_takes_a_brim_and_a_raft() {
    let mut w = bench(
        "profiles/printers/prusa_core_one.toml",
        "profiles/filaments/generic_pla.toml",
    );
    for adhesion in [json!({ "type": "brim" }), json!({ "type": "raft" })] {
        w.set_setup(serde_json::from_value(json!({ "adhesion": adhesion })).unwrap())
            .unwrap();
        for file in catalogue() {
            let src = std::fs::read_to_string(&file).unwrap();
            w.load_model(&src).unwrap();
            let ev = w.evaluate(&Map::new());
            let codes: Vec<_> = ev
                .issues
                .iter()
                .filter(|i| !i.is_error() && i.code == "adhesion.skipped")
                .collect();
            assert!(
                codes.is_empty(),
                "{} {adhesion}: adhésion non ajoutée",
                file.display()
            );
            let errors: Vec<_> = ev.issues.iter().filter(|i| i.is_error()).collect();
            assert!(
                errors.is_empty(),
                "{} {adhesion}: {errors:?}",
                file.display()
            );
            w.gcode(&ev).unwrap();
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
                ParamKind::Choice { ref options, .. } => {
                    options.iter().map(|o| json!(o.value)).collect()
                }
            };
            for v in ends {
                let mut values = Map::new();
                values.insert(id.clone(), v.clone());
                let ev = w.evaluate(&values);
                // Les seules erreurs admises aux bornes : pièce trop grande pour
                // le plateau, refus expliqué du modèle (`fail`), collision vue
                // par le garde-buse.
                for i in ev.issues.iter().filter(|i| i.is_error()) {
                    assert!(
                        matches!(
                            i.code,
                            "path.out_of_bed"
                                | "path.too_high"
                                | "path.hop_too_high"
                                | "model.failure"
                                | "guard.nozzle_collision"
                                | "guard.head_collision"
                        ),
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

#[test]
fn pillow_lamp_is_one_spiral_and_respects_the_nozzle_cone() {
    let read = |p: &str| std::fs::read_to_string(root().join(p)).unwrap();
    let core = read("profiles/printers/prusa_core_one.toml");
    let with_cone = |angle: f64, height: f64| {
        core.replacen(
            "diameter = 0.4\n",
            &format!("diameter = 0.4\ncone_angle = {angle}\ncone_height = {height}\n"),
            1,
        )
    };
    let mut w = bench(
        "profiles/printers/prusa_core_one.toml",
        "profiles/filaments/generic_pla.toml",
    );
    w.load_model(&read("models/vases/pillow_lamp.weave"))
        .unwrap();
    // Profil sans cône : la descente est signalée, pas refusée ; une seule
    // spirale, du sommet posé sur le plateau jusqu'au bord.
    let ev = w.evaluate(&Map::new());
    assert!(ev.issues.iter().any(|i| i.code == "model.warning"));
    let s = &ev.plan.as_ref().unwrap().stats;
    assert_eq!(s.travel_moves, 0);
    assert_eq!(s.retractions, 0);
    // Rien dans le vide : chaque point (hors du plateau) est à moins d'un
    // boudin de la matière posée au moins 3 mm de tracé plus tôt. La première
    // version, aux tours qui ne se touchaient qu'aux nœuds, en laissait 63 %.
    let pts: Vec<[f64; 3]> = ev
        .path
        .as_ref()
        .unwrap()
        .points
        .iter()
        .map(|p| [p.x, p.y, p.z])
        .collect();
    assert!(unsupported_share(&pts, 0.7, 3.0) < 0.01);
    // Défaut : 2,5 mm de descente à 29°. Cône de 45° sur 3 mm : la buse
    // passe ; sur 2,5 mm, ou à 30°, elle toucherait les tours déjà posés.
    for (angle, height, passes) in [(45.0, 3.0, true), (45.0, 2.5, false), (30.0, 5.0, false)] {
        w.set_printer(&with_cone(angle, height)).unwrap();
        let ev = w.evaluate(&Map::new());
        let refused = ev.issues.iter().any(|i| i.code == "model.failure");
        assert_eq!(refused, !passes, "cône {angle}° / {height} mm");
        // Le garde-buse ne contredit pas le modèle : au pire, elle frôle.
        if passes {
            assert!(
                !ev.issues.iter().any(|i| i.is_error()),
                "cône {angle}° / {height} mm: {:?}",
                ev.issues
            );
        }
    }
}

/// Part des points (au-dessus du plateau) à plus de `reach` mm de toute
/// matière posée au moins `recent` mm de tracé plus tôt.
fn unsupported_share(pts: &[[f64; 3]], reach: f64, recent: f64) -> f64 {
    use std::collections::HashMap;
    let dist = |a: [f64; 3], b: [f64; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    let key = |p: [f64; 3]| {
        [
            p[0].floor() as i64,
            p[1].floor() as i64,
            p[2].floor() as i64,
        ]
    };
    let z_bed = pts.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
    let mut cum = vec![0.0; pts.len()];
    for i in 1..pts.len() {
        cum[i] = cum[i - 1] + dist(pts[i], pts[i - 1]);
    }
    let mut grid: HashMap<[i64; 3], Vec<[f64; 3]>> = HashMap::new();
    let (mut pending, mut air, mut counted) = (0, 0, 0);
    for (i, &p) in pts.iter().enumerate() {
        while pending < i && cum[i] - cum[pending] > recent {
            grid.entry(key(pts[pending]))
                .or_default()
                .push(pts[pending]);
            pending += 1;
        }
        if p[2] < z_bed + 0.15 {
            continue;
        }
        counted += 1;
        let k = key(p);
        let near = (-1..=1).any(|dx| {
            (-1..=1).any(|dy| {
                (-1..=1).any(|dz| {
                    grid.get(&[k[0] + dx, k[1] + dy, k[2] + dz])
                        .is_some_and(|v| v.iter().any(|&q| dist(p, q) <= reach))
                })
            })
        });
        if !near {
            air += 1;
        }
    }
    air as f64 / counted.max(1) as f64
}
