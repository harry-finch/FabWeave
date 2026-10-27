//! Import depuis les trancheurs : un export à plat de PrusaSlicer (MK4S et
//! PLA), un préréglage machine et un filament de Bambu Studio. Les valeurs
//! reprises, absentes et ignorées sont listées ; les gabarits sont traduits
//! et le G-code n'est exporté qu'après relecture.

use std::path::Path;

use fabweave_core::import::{import, Fallback, Imported, ImportedProfile};
use fabweave_core::profile::{
    check_printer, filament_toml, parse_filament, parse_printer, printer_toml, BedShape,
    FilamentProfile, Firmware, PrinterProfile,
};
use fabweave_core::Workbench;
use serde_json::Map;

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

fn bases() -> (Vec<PrinterProfile>, Vec<FilamentProfile>) {
    let printers = [
        "prusa_mk4s",
        "bambu_x1e",
        "marlin_generic",
        "klipper_generic",
    ]
    .map(|id| parse_printer(&read(&format!("../../profiles/printers/{id}.toml"))).unwrap());
    let filaments = ["generic_pla", "generic_petg"]
        .map(|id| parse_filament(&read(&format!("../../profiles/filaments/{id}.toml"))).unwrap());
    (printers.to_vec(), filaments.to_vec())
}

fn run(file: &str) -> Vec<Imported> {
    let (p, f) = bases();
    import(&read(&format!("tests/fixtures/import/{file}")), &p, &f).unwrap()
}

fn printer_of(i: &Imported) -> &PrinterProfile {
    match &i.profile {
        ImportedProfile::Printer(p) => p,
        _ => panic!("imprimante attendue"),
    }
}

fn filament_of(i: &Imported) -> &FilamentProfile {
    match &i.profile {
        ImportedProfile::Filament(f) => f,
        _ => panic!("filament attendu"),
    }
}

fn key_of(i: &Imported, field: &str) -> String {
    i.report
        .taken
        .iter()
        .find(|t| t.field == field)
        .map(|t| t.key.clone())
        .unwrap_or_else(|| panic!("{field} non repris"))
}

#[test]
fn prusaslicer_flat_export_gives_a_printer_and_a_filament() {
    let found = run("prusaslicer_mk4s_pla.ini");
    assert_eq!(found.len(), 2);
    let (pi, fi) = (&found[0], &found[1]);

    let p = printer_of(pi);
    assert_eq!(pi.report.slicer, "PrusaSlicer 2.9.2");
    assert_eq!(pi.report.base, "prusa_mk4s");
    assert_eq!(p.printer.firmware, Firmware::Prusa);
    assert_eq!(p.printer.model, "MK4S");
    assert_eq!(p.bed.size, [250.0, 210.0]);
    assert_eq!(p.bed.shape, BedShape::Rectangle);
    assert_eq!(p.bed.max_height, 220.0);
    assert_eq!(p.nozzle.clearance_radius, Some(45.0));
    assert_eq!(p.limits.max_z_speed, 40.0);
    // Accélération de la paroi extérieure (réglages d'impression du fichier).
    assert_eq!(
        key_of(pi, "limits.acceleration"),
        "external_perimeter_acceleration"
    );
    assert_eq!(p.gcode.thumbnails.len(), 4);
    assert_eq!(p.printer.review, ["gcode.start", "gcode.end"]);
    // Gabarits entièrement traduits ; réglages du fichier remplacés par
    // leur valeur, et listés.
    assert!(
        pi.report.untranslated.is_empty(),
        "{:?}",
        pi.report.untranslated
    );
    assert!(pi.report.constants.iter().any(|c| c.name == "z_offset"));
    assert!(check_printer(p).is_empty(), "{:?}", check_printer(p));
    // Ignorés : réglages d'impression ; gabarits sans équivalent.
    assert!(pi.report.ignored.iter().any(|k| k == "perimeter_speed"));
    assert!(!pi.report.ignored.iter().any(|k| k == "temperature"));
    assert_eq!(
        pi.report.ignored_gcode,
        ["before_layer_gcode", "color_change_gcode", "layer_gcode"]
    );

    let f = filament_of(fi);
    assert_eq!(f.filament.name.get("fr"), "Generic PLA @PG");
    assert_eq!(f.filament.material, "PLA");
    assert_eq!(f.filament.pressure_advance, 0.036);
    assert_eq!(
        key_of(fi, "filament.pressure_advance"),
        "start_filament_gcode (M572)"
    );
    assert_eq!(
        [f.temperature.first_layer, f.temperature.nozzle],
        [215.0, 210.0]
    );
    assert_eq!(f.temperature.idle, Some(70.0));
    assert_eq!(f.cooling.first_layer_fan, 0.0);
    // Rétraction de l'imprimante (le filament ne la règle pas).
    assert_eq!(key_of(fi, "retraction.length"), "retract_length");
    // Absentes des trancheurs : vitesses gardées de la base, débits par buse vidés.
    let missing = |field: &str| {
        fi.report
            .missing
            .iter()
            .find(|m| m.field == field)
            .map(|m| m.fallback)
    };
    assert_eq!(missing("speed.nominal"), Some(Fallback::Base));
    assert_eq!(missing("speed.by_nozzle"), Some(Fallback::Empty));
    assert!(f.speed.by_nozzle.is_empty());
    assert!(fi.report.ignored.is_empty());
    assert_eq!(
        fi.report.ignored_gcode,
        ["end_filament_gcode", "start_filament_gcode"]
    );
}

#[test]
fn imported_profiles_print_only_after_review() {
    let found = run("prusaslicer_mk4s_pla.ini");
    let mut p = printer_of(&found[0]).clone();
    let f = filament_of(&found[1]).clone();
    let mut bench = Workbench::default();
    bench
        .load_model(&read("../../models/vases/ripple_vase.weave"))
        .unwrap();
    bench.set_printer(&printer_toml(&p)).unwrap();
    bench.set_filament(&filament_toml(&f)).unwrap();
    let ev = bench.evaluate(&Map::new());
    assert!(ev.plan.is_some(), "le parcours se montre");
    assert!(ev.issues.iter().any(|i| i.code == "profile.review_pending"));
    assert!(bench.gcode(&ev).is_err());

    p.printer.review.clear();
    bench.set_printer(&printer_toml(&p)).unwrap();
    let ev = bench.evaluate(&Map::new());
    let g = bench.gcode(&ev).unwrap();
    // Gabarit de PrusaSlicer rendu avec les valeurs de FabWeave.
    assert!(g.contains("M862.3 P \"MK4S\" ; printer model check"));
    assert!(g.contains("M104 T0 S170 ; set extruder temp for bed leveling"));
    assert!(g.contains("\nG1 E-2 F2400 ; retraction\n"));
    assert!(g.contains("\nM106 S100\n"));
    assert!(g.contains("F720 ; Move print head up"));
}

#[test]
fn bambu_studio_machine_and_user_filament() {
    let found = run("bambu_x1e_machine.json");
    let pi = &found[0];
    let p = printer_of(pi);
    assert_eq!(pi.report.base, "bambu_x1e");
    assert_eq!(p.printer.firmware, Firmware::Bambu);
    assert_eq!(p.bed.size, [256.0, 256.0]);
    assert_eq!(p.bed.exclude.len(), 1);
    assert_eq!(p.bed.exclude[0].max, [18.0, 28.0]);
    assert_eq!(p.nozzle.clearance_height, Some(34.0));
    assert_eq!(p.limits.jerk, Some(9.0));
    assert!(
        pi.report.untranslated.is_empty(),
        "{:?}",
        pi.report.untranslated
    );
    assert!(check_printer(p).is_empty(), "{:?}", check_printer(p));
    assert!(pi
        .report
        .ignored_gcode
        .iter()
        .any(|k| k == "layer_change_gcode"));

    let found = run("bambu_pla_user.json");
    let fi = &found[0];
    let f = filament_of(fi);
    // Préréglage parent du système, absent du fichier : signalé.
    assert_eq!(
        fi.report.inherits.as_deref(),
        Some("Bambu PLA Basic @BBL X1E")
    );
    assert_eq!(fi.report.base, "generic_pla");
    assert_eq!(f.filament.pressure_advance, 0.025);
    assert_eq!(f.filament.flow_multiplier, 0.98);
    assert_eq!(
        [f.temperature.first_layer, f.temperature.nozzle],
        [220.0, 215.0]
    );
    assert_eq!(f.speed.max_volumetric, 18.0);
    assert!(fi
        .report
        .missing
        .iter()
        .any(|m| m.field == "filament.diameter" && m.fallback == Fallback::Base));
    assert!(parse_filament(&filament_toml(f)).is_ok());
}

#[test]
fn files_without_a_profile_are_refused() {
    let (p, f) = bases();
    let code = |src: &str| import(src, &p, &f).unwrap_err()[0].code;
    assert_eq!(
        code(&read("tests/fixtures/import/bambu_process.json")),
        "import.process_profile"
    );
    assert_eq!(code("{ \"a\": "), "import.invalid_json");
    assert_eq!(code("layer_height = 0.2\n"), "import.no_profile");
}

#[test]
fn shipped_profiles_survive_a_toml_round_trip() {
    let (printers, filaments) = bases();
    for p in printers {
        let again = parse_printer(&printer_toml(&p)).unwrap();
        assert_eq!(printer_toml(&again), printer_toml(&p));
    }
    for f in filaments {
        let again = parse_filament(&filament_toml(&f)).unwrap();
        assert_eq!(filament_toml(&again), filament_toml(&f));
    }
}
