//! Profils imprimante : chaque profil livré se lit ; l'ancien format (v1)
//! se lit encore ; les valeurs du format v2 sont contrôlées.

use std::path::Path;

use fabweave_core::profile::{parse_printer, Firmware, ImageFormat, PRINTER_VERSION};

fn read(rel: &str) -> String {
    std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)).unwrap()
}

const SHIPPED: [&str; 5] = [
    "prusa_mk4s",
    "prusa_core_one",
    "bambu_x1e",
    "marlin_generic",
    "klipper_generic",
];

#[test]
fn shipped_printers_are_current_v2_profiles() {
    for id in SHIPPED {
        let src = read(&format!("../../profiles/printers/{id}.toml"));
        let p = parse_printer(&src).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        assert_eq!(p.printer.id, id);
        assert_eq!(p.version, Some(PRINTER_VERSION), "{id}");
        assert!(p.limits.accelerations().is_some(), "{id} : accélérations");
    }
}

#[test]
fn a_v1_profile_still_reads() {
    // Le profil MK4S de FabWeave 0.2, gardé tel quel.
    let p = parse_printer(&read("tests/fixtures/prusa_mk4s_v1.toml")).unwrap();
    assert_eq!(p.version, Some(1));
    assert_eq!(p.printer.firmware, Firmware::Prusa);
    assert_eq!(p.printer.name.get("en"), "Prusa MK4S");
    // Champs v2 absents : valeurs neutres (pas de zone, pas d'accélération,
    // pas de vignette, cône inconnu).
    assert!(p.bed.exclude.is_empty());
    assert!(p.limits.accelerations().is_none());
    assert!(p.gcode.thumbnail_specs().is_empty());
    assert!(p.nozzle.cone_angle.is_none());
}

fn with(src: &str, from: &str, to: &str) -> String {
    assert!(src.contains(from), "{from}");
    src.replacen(from, to, 1)
}

fn codes(src: &str) -> Vec<&'static str> {
    parse_printer(src)
        .unwrap_err()
        .iter()
        .map(|i| i.code)
        .collect()
}

#[test]
fn v2_values_are_checked() {
    let mk4s = read("../../profiles/printers/prusa_mk4s.toml");
    let thumbs = parse_printer(&mk4s).unwrap().gcode.thumbnail_specs();
    assert_eq!(thumbs.len(), 4);
    assert_eq!(
        (thumbs[0].width, thumbs[0].height, thumbs[0].format),
        (16, 16, ImageFormat::Qoi)
    );

    // Version plus récente que le moteur, dialecte inconnu.
    assert_eq!(
        codes(&with(&mk4s, "version = 2", "version = 3")),
        ["profile.unsupported_version"]
    );
    assert_eq!(
        codes(&with(
            &mk4s,
            "firmware = \"prusa\"",
            "firmware = \"reprap\""
        )),
        ["profile.invalid"]
    );
    // Vignette mal écrite, ou trop grande.
    for bad in ["16x16/JPG", "16-16/QOI", "2000x10/PNG", "0x10/PNG"] {
        let src = with(&mk4s, "\"16x16/QOI\"", &format!("\"{bad}\""));
        assert_eq!(codes(&src), ["profile.invalid_thumbnail"], "{bad}");
    }
    // Accélération nulle ; accélération des déplacements sans accélération d'impression.
    assert_eq!(
        codes(&with(&mk4s, "acceleration = 1500.0", "acceleration = 0.0")),
        ["profile.must_be_positive"]
    );
    assert_eq!(
        codes(&with(&mk4s, "\nacceleration = 1500.0", "\n")),
        ["profile.missing_acceleration"]
    );
    // Cône : pente de 0 à 90° exclus.
    assert_eq!(
        codes(&with(
            &mk4s,
            "diameter = 0.4",
            "diameter = 0.4\ncone_angle = 95.0"
        )),
        ["profile.out_of_range"]
    );
    // Zone exclue retournée.
    let x1e = read("../../profiles/printers/bambu_x1e.toml");
    assert_eq!(
        codes(&with(&x1e, "max = [18.0, 28.0]", "max = [18.0, -28.0]")),
        ["profile.invalid_zone"]
    );
}
