//! Filaments livrés (v0.3, étape 7) : chacun se lit, se contrôle et survit à
//! l'écriture TOML ; l'enceinte minimale fait attendre la CORE One ; un
//! filament abrasif dans une buse qui ne l'est pas est signalé ; le facteur
//! des ponts du filament s'applique aux ponts seulement.

use std::path::Path;

use fabweave_core::profile::{filament_toml, parse_filament};
use fabweave_core::{MoveKind, Workbench};
use serde_json::Map;

fn read(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel),
    )
    .unwrap()
}

fn shipped() -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles/filaments");
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .map(|p| p.file_stem().unwrap().to_string_lossy().into_owned())
        .collect();
    ids.sort();
    ids
}

const SQUARE: &str = "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet p = new_path();\np.travel_to(-10, -10, 0.2);\np.print_to(10, -10, 0.2, 0.45, 0.2);\np.print_to(10, 10, 0.2, 0.45, 0.2);\np.bridge_to(-10, 10, 0.2, 0.45, 0.2, 0.8);\np";

fn bench(printer: &str, filament: &str) -> Workbench {
    let mut w = Workbench::default();
    w.set_printer(&read(&format!("profiles/printers/{printer}.toml")))
        .unwrap();
    w.set_filament(&read(&format!("profiles/filaments/{filament}.toml")))
        .unwrap();
    w.load_model(SQUARE).unwrap();
    w
}

#[test]
fn shipped_filaments_read_and_round_trip() {
    let ids = shipped();
    for id in [
        "generic_pla",
        "generic_petg",
        "generic_tpu",
        "generic_asa",
        "generic_petg_cf",
    ] {
        assert!(ids.iter().any(|i| i == id), "{id} manquant");
    }
    for id in &ids {
        let f = parse_filament(&read(&format!("profiles/filaments/{id}.toml")))
            .unwrap_or_else(|e| panic!("{id}: {e:?}"));
        assert_eq!(&f.filament.id, id);
        let again = parse_filament(&filament_toml(&f)).unwrap();
        assert_eq!(filament_toml(&again), filament_toml(&f), "{id}");
    }
}

#[test]
fn asa_waits_for_the_chamber_on_the_core_one() {
    let w = bench("prusa_core_one", "generic_asa");
    let ev = w.evaluate(&Map::new());
    let g = w.gcode(&ev).unwrap();
    assert!(g.contains("M140 S115 ; set bed temp for chamber heating"));
    assert!(g.contains("M191 S40 ; wait for minimal chamber temp"));
    assert!(g.contains("M141 S55 ; set nominal chamber temp"));
    assert!(g.contains("M140 S110 ; set bed temp"));

    // Sans enceinte minimale : pas d'attente.
    let w = bench("prusa_core_one", "generic_pla");
    let g = w.gcode(&w.evaluate(&Map::new())).unwrap();
    assert!(!g.contains("M191"));
    assert!(g.contains("M140 S60 ; set bed temp"));
    assert!(g.contains("M141 S20 ; set nominal chamber temp"));

    // Le X1E chauffe et attend son enceinte au-delà de 40 °C.
    let w = bench("bambu_x1e", "generic_asa");
    let g = w.gcode(&w.evaluate(&Map::new())).unwrap();
    assert!(g.contains("M191 S55"), "X1E");
}

#[test]
fn tpu_uses_the_flex_branches_of_the_prusa_templates() {
    let w = bench("prusa_mk4s", "generic_tpu");
    let g = w.gcode(&w.evaluate(&Map::new())).unwrap();
    assert!(g.contains("G1 E-4 F2400"), "rétraction FLEX au démarrage");
    assert!(g.contains("M109 T0 R210"), "palpage du TPU");
}

#[test]
fn an_abrasive_filament_in_a_plain_nozzle_is_reported() {
    let ev = bench("prusa_mk4s", "generic_petg_cf").evaluate(&Map::new());
    assert!(ev.issues.iter().any(|i| i.code == "nozzle.not_abrasive"));
}

#[test]
fn the_bridge_factor_scales_bridges_only() {
    let base = bench("prusa_mk4s", "generic_pla").evaluate(&Map::new());
    let src = read("profiles/filaments/generic_pla.toml").replace(
        "pressure_advance = 0.036",
        "pressure_advance = 0.036\nbridge_flow = 0.5",
    );
    assert!(src.contains("bridge_flow"));
    let mut w = bench("prusa_mk4s", "generic_pla");
    w.set_filament(&src).unwrap();
    let ev = w.evaluate(&Map::new());
    let (a, b) = (base.plan.unwrap(), ev.plan.unwrap());
    let path = ev.path.unwrap();
    for (i, p) in path.points.iter().enumerate().skip(1) {
        let k = if p.kind == MoveKind::Bridge { 0.5 } else { 1.0 };
        assert!((b.moves[i].e - a.moves[i].e * k).abs() < 1e-12, "{i}");
    }
    assert!(path.points.iter().any(|p| p.kind == MoveKind::Bridge));

    let bad = src.replace("bridge_flow = 0.5", "bridge_flow = 3");
    let codes: Vec<_> = parse_filament(&bad)
        .unwrap_err()
        .iter()
        .map(|i| i.code)
        .collect();
    assert_eq!(codes, ["profile.out_of_range"]);
}
