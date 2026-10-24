//! Exporte le G-code d'un modèle sans interface.
//!
//! `cargo run --release -p fabweave-core --example export -- <modèle.weave>
//!  <imprimante.toml> <filament.toml> <sortie.gcode> [id=valeur …]`
//!
//! Les mesures et les messages sont écrits sur la sortie d'erreur.

use fabweave_core::Workbench;
use serde_json::{Map, Value};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 4 {
        eprintln!("usage : export <modèle.weave> <imprimante.toml> <filament.toml> <sortie.gcode> [id=valeur …]");
        std::process::exit(2);
    }
    let read = |p: &str| std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{p} : {e}"));
    let mut w = Workbench::default();
    let fail = |e: Vec<fabweave_core::Issue>| -> ! {
        eprintln!("{}", serde_json::to_string_pretty(&e).unwrap());
        std::process::exit(1)
    };
    w.load_model(&read(&args[0])).unwrap_or_else(|e| fail(e));
    w.set_printer(&read(&args[1])).unwrap_or_else(|e| fail(e));
    w.set_filament(&read(&args[2])).unwrap_or_else(|e| fail(e));

    let mut values = Map::new();
    for kv in &args[4..] {
        let (k, v) = kv.split_once('=').expect("id=valeur");
        let v = serde_json::from_str::<Value>(v).unwrap_or(Value::String(v.into()));
        values.insert(k.to_string(), v);
    }
    let ev = w.evaluate(&values);
    for line in &ev.log {
        eprintln!("print : {line}");
    }
    if !ev.issues.is_empty() {
        eprintln!("{}", serde_json::to_string_pretty(&ev.issues).unwrap());
    }
    let gcode = w.gcode(&ev).unwrap_or_else(|e| fail(e));
    if let Some(plan) = &ev.plan {
        eprintln!("{}", serde_json::to_string_pretty(&plan.stats).unwrap());
    }
    std::fs::write(&args[3], gcode).unwrap();
    eprintln!("→ {}", args[3]);
}
