//! Projet embarqué (DESK-04) : un G-code ou un paquet exporté rouvre la même
//! préparation, qui redonne le même G-code, octet pour octet.

use std::path::Path;

use fabweave_core::project::Project;
use fabweave_core::setup::{Adhesion, NozzleSetup, Placement, Setup};
use fabweave_core::Workbench;
use serde_json::{json, Map, Value};

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

/// Poste de travail chargé depuis un projet, comme le fait l'interface.
fn reopen(p: &Project) -> (Workbench, Map<String, Value>) {
    let mut w = Workbench::default();
    w.load_model(&p.model).unwrap();
    w.set_printer(&p.printer).unwrap();
    w.set_filament(&p.filament).unwrap();
    w.set_setup(p.setup.clone()).unwrap();
    w.set_name(&p.name);
    (w, p.values.clone().into_iter().collect())
}

#[test]
fn an_exported_gcode_reopens_the_same_preparation() {
    let mut w = Workbench::default();
    w.load_model(&read("../../models/vases/ripple_vase.weave"))
        .unwrap();
    w.set_printer(&read("../../profiles/printers/prusa_core_one.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_petg.toml"))
        .unwrap();
    w.set_setup(Setup {
        nozzle: NozzleSetup {
            diameter: Some(0.6),
            ..NozzleSetup::default()
        },
        placement: Placement {
            rotation: 30.0,
            ..Placement::default()
        },
        adhesion: Adhesion::Brim {
            width: 3.0,
            gap: 0.1,
        },
    })
    .unwrap();
    w.set_name("Vase de l'entrée");
    let values = json!({ "height": 60.0, "ripples_per_layer": 12 });
    let values = values.as_object().unwrap();
    let ev = w.evaluate(values);
    let g = w.gcode(&ev).unwrap();
    assert_eq!(g, w.gcode(&ev).unwrap(), "déterministe");

    let p = Project::read(g.as_bytes()).unwrap();
    assert_eq!(p.name, "Vase de l'entrée");
    assert_eq!(p.values["height"], json!(60.0));
    assert_eq!(p.values["ripples_per_layer"], json!(12));
    // Tous les paramètres, pas seulement ceux modifiés.
    assert_eq!(p.values.len(), ev.params.len());
    assert_eq!(p.setup.nozzle.diameter, Some(0.6));

    let (w2, values2) = reopen(&p);
    let g2 = w2.gcode(&w2.evaluate(&values2)).unwrap();
    assert_eq!(g2, g, "même G-code, octet pour octet");
    // Le fichier `.fabweave.json` du même calcul porte le même projet.
    assert_eq!(
        Project::parse(&w.project(&ev, values).unwrap().to_json()).unwrap(),
        p
    );
}

#[test]
fn a_bambu_package_carries_the_project_apart() {
    let mut w = Workbench::default();
    w.load_model(&read("../../models/mecanique/hex_adapter.weave"))
        .unwrap();
    w.set_printer(&read("../../profiles/printers/bambu_x1e.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_pla.toml"))
        .unwrap();
    let ev = w.evaluate(&Map::new());
    let z = w.gcode_3mf(&ev).unwrap();
    let p = Project::read(&z).unwrap();
    assert_eq!(p, w.project(&ev, &Map::new()).unwrap());
    let (w2, values2) = reopen(&p);
    assert_eq!(w2.gcode_3mf(&w2.evaluate(&values2)).unwrap(), z);
}

#[test]
fn a_refused_value_is_kept_in_the_project() {
    let mut w = Workbench::default();
    w.load_model(&read("../../models/vases/ripple_vase.weave"))
        .unwrap();
    w.set_printer(&read("../../profiles/printers/prusa_mk4s.toml"))
        .unwrap();
    w.set_filament(&read("../../profiles/filaments/generic_pla.toml"))
        .unwrap();
    let given = json!({ "height": 9999.0 });
    let given = given.as_object().unwrap();
    let ev = w.evaluate(given);
    assert!(ev.issues.iter().any(|i| i.code == "param.out_of_range"));
    let p = w.project(&ev, given).unwrap();
    assert_eq!(
        p.values["height"],
        json!(9999.0),
        "jamais corrigée en silence"
    );
}
