//! Temps de calcul par étage, en natif (ADR-0002) : lecture et compilation,
//! script Rhai, plan (physique), écriture du G-code.
//!
//! `cargo run --release -p fabweave-core --example bench -- [répétitions]
//!  [modèle.weave …]` (sans modèle : cas de référence).

use std::time::Instant;

use fabweave_core::{plan, Workbench};
use serde_json::{json, Map, Value};

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

fn main() {
    let repeats: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let files: Vec<String> = std::env::args().skip(2).collect();
    let mut cases: Vec<(String, String, Value)> = vec![
        (
            "vase à ondulations (défaut)".into(),
            "models/vases/ripple_vase.weave".into(),
            json!({}),
        ),
        (
            "spirale 50 000 points".into(),
            "crates/fabweave-core/tests/fixtures/bench_spiral.weave".into(),
            json!({ "points": 50000 }),
        ),
        (
            "spirale 200 000 points".into(),
            "crates/fabweave-core/tests/fixtures/bench_spiral.weave".into(),
            json!({ "points": 200000 }),
        ),
        (
            "vase, 12 points par ondulation".into(),
            "models/vases/ripple_vase.weave".into(),
            json!({ "ripple_segments": 12 }),
        ),
    ];
    if !files.is_empty() {
        cases = files
            .iter()
            .map(|f| (f.clone(), f.clone(), json!({})))
            .collect();
    }
    let read = |p: &str| {
        std::fs::read_to_string(p)
            .or_else(|_| std::fs::read_to_string(format!("{root}/{p}")))
            .unwrap()
    };
    println!("| Cas | Points | Lecture + compilation | Script | Plan | G-code | Taille |");
    println!("| --- | ---: | ---: | ---: | ---: | ---: | ---: |");
    for (name, file, values) in cases {
        let (name, file) = (name.as_str(), file.as_str());
        let values: Map<String, Value> = values.as_object().unwrap().clone();
        let mut w = Workbench::default();
        w.set_printer(&read("profiles/printers/prusa_mk4s.toml"))
            .unwrap();
        w.set_filament(&read("profiles/filaments/generic_pla.toml"))
            .unwrap();
        let src = read(file);
        let t = Instant::now();
        w.load_model(&src).unwrap();
        let load = t.elapsed().as_secs_f64() * 1e3;
        let (mut script, mut plan, mut emit) = (Vec::new(), Vec::new(), Vec::new());
        let (mut points, mut size) = (0, 0);
        for _ in 0..repeats {
            let t = Instant::now();
            let out = w.run_script(&values).unwrap();
            script.push(t.elapsed().as_secs_f64() * 1e3);
            let path = out.path.unwrap();
            points = path.points.len();
            let t = Instant::now();
            let (p, _) = plan::plan(&path, w.printer().unwrap(), w.filament().unwrap());
            plan.push(t.elapsed().as_secs_f64() * 1e3);
            assert!(p.is_some(), "{name} : parcours non imprimable");
            let ev = w.evaluate(&values);
            let t = Instant::now();
            size = w.gcode(&ev).unwrap().len();
            emit.push(t.elapsed().as_secs_f64() * 1e3);
        }
        println!(
            "| {name} | {points} | {load:.2} ms | {:.1} ms | {:.1} ms | {:.1} ms | {:.1} Mo |",
            median(script),
            median(plan),
            median(emit),
            size as f64 / 1e6
        );
    }
}
