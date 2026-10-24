//! Profils imprimante et filament (TOML, versionnés). Le modèle décrit la
//! géométrie ; les profils apportent la physique (débit, vitesses, plafonds)
//! et le dialecte de la machine (gabarits de démarrage et de fin).
//!
//! Lecture stricte : champ inconnu, valeur hors domaine ou gabarit invalide
//! produisent une erreur à code stable, avec la ligne dans le fichier.

use serde::{Deserialize, Serialize};

use crate::issue::{line_col, Issue};
use crate::template::TemplateEngine;

pub const PRINTER_FORMAT: &str = "fabweave.printer";
pub const FILAMENT_FORMAT: &str = "fabweave.filament";
pub const VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// (0, 0) au coin avant gauche du plateau (Prusa, Bambu, la plupart).
    Corner,
    /// (0, 0) au centre du plateau (delta, certaines Voron).
    Center,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrinterInfo {
    pub id: String,
    pub name: String,
    /// Modèle annoncé au firmware (`M862.3`, `; printer_model`).
    pub model: String,
    /// Dialecte : `prusa`, `marlin`, `klipper`, `bambu`.
    pub firmware: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bed {
    /// Surface imprimable X, Y (mm).
    pub size: [f64; 2],
    /// Hauteur d'impression maximale (mm).
    pub max_height: f64,
    pub origin: Origin,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nozzle {
    pub diameter: f64,
    #[serde(default)]
    pub high_flow: bool,
    #[serde(default)]
    pub abrasive: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineLimits {
    /// Vitesse d'impression maximale (mm/s).
    pub max_print_speed: f64,
    /// Vitesse des déplacements (mm/s).
    pub travel_speed: f64,
    /// Vitesse maximale de l'axe Z (mm/s) : plafond essentiel en non planaire.
    pub max_z_speed: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayerDefaults {
    /// Hauteur de couche conseillée aux modèles (`printer.layer_height`).
    pub layer_height: f64,
    pub first_layer_height: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Templates {
    pub start: String,
    pub end: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrinterProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    pub printer: PrinterInfo,
    pub bed: Bed,
    pub nozzle: Nozzle,
    pub limits: MachineLimits,
    pub layers: LayerDefaults,
    pub gcode: Templates,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilamentInfo {
    pub id: String,
    pub name: String,
    /// Matière (`PLA`, `PETG`, `FLEX`…), écrite dans le G-code.
    #[serde(rename = "type")]
    pub material: String,
    pub diameter: f64,
    /// Masse volumique (g/cm³).
    pub density: f64,
    #[serde(default = "one")]
    pub flow_multiplier: f64,
    /// Avance de pression (`M572` Prusa, `M900` Marlin), 0 si inconnue.
    #[serde(default)]
    pub pressure_advance: f64,
    /// Matière abrasive (chargée de fibres) : buse dure exigée.
    #[serde(default)]
    pub abrasive: bool,
}

fn one() -> f64 {
    1.0
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Temperatures {
    pub first_layer: f64,
    pub nozzle: f64,
    pub bed_first_layer: f64,
    pub bed: f64,
    /// Buse pendant le palpage du plateau (sans suinter).
    pub probe: f64,
    /// Buse en attente (chauffe du plateau, de l'enceinte), sinon `probe`.
    #[serde(default)]
    pub idle: Option<f64>,
    /// Consigne de l'enceinte fermée (Core One…), 0 : sans régulation.
    #[serde(default)]
    pub chamber: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Speeds {
    /// Vitesse d'impression pour `set_speed_factor(1)` (mm/s).
    pub nominal: f64,
    /// Plafond de la première couche (mm/s).
    pub first_layer: f64,
    /// Débit volumétrique maximal (mm³/s).
    pub max_volumetric: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Cooling {
    /// Ventilateur en croisière (%).
    pub fan: f64,
    /// Ventilateur sur la première couche (%).
    #[serde(default)]
    pub first_layer_fan: f64,
    /// Ventilateur au moins sur les ponts (%).
    pub bridge_fan: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Retraction {
    pub length: f64,
    /// mm/s.
    pub speed: f64,
    pub z_hop: f64,
    /// Pas de rétraction pour un déplacement plus court (mm).
    pub min_travel: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilamentProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    pub filament: FilamentInfo,
    pub temperature: Temperatures,
    pub speed: Speeds,
    pub cooling: Cooling,
    pub retraction: Retraction,
}

impl FilamentProfile {
    /// Section du filament (mm²).
    pub fn area(&self) -> f64 {
        std::f64::consts::PI * (self.filament.diameter / 2.0).powi(2)
    }
}

fn read<T: for<'de> Deserialize<'de>>(src: &str) -> Result<T, Vec<Issue>> {
    toml::from_str(src).map_err(|e| {
        vec![Issue::error("profile.invalid")
            .at(e.span().map(|s| line_col(src, s.start)))
            .detail(e.message().to_string())]
    })
}

fn check_format(
    format: &Option<String>,
    version: Option<u32>,
    expected: &str,
    out: &mut Vec<Issue>,
) {
    if format.as_deref().is_some_and(|f| f != expected) {
        out.push(
            Issue::error("profile.wrong_format")
                .field("format")
                .detail(format.clone().unwrap_or_default()),
        );
    }
    if version.is_some_and(|v| v == 0 || v > VERSION) {
        out.push(Issue::error("profile.unsupported_version").field("version"));
    }
}

/// Valeurs qui doivent être strictement positives.
fn positive(values: &[(&str, f64)], out: &mut Vec<Issue>) {
    for (field, v) in values {
        if !(*v > 0.0 && v.is_finite()) {
            out.push(
                Issue::error("profile.must_be_positive")
                    .field(*field)
                    .value(*v),
            );
        }
    }
}

fn non_negative(values: &[(&str, f64)], out: &mut Vec<Issue>) {
    for (field, v) in values {
        if !(*v >= 0.0 && v.is_finite()) {
            out.push(
                Issue::error("profile.must_not_be_negative")
                    .field(*field)
                    .value(*v),
            );
        }
    }
}

fn percent(values: &[(&str, f64)], out: &mut Vec<Issue>) {
    for (field, v) in values {
        if !(0.0..=100.0).contains(v) {
            out.push(Issue::error("profile.out_of_range").field(*field).value(*v));
        }
    }
}

pub fn parse_printer(src: &str) -> Result<PrinterProfile, Vec<Issue>> {
    let p: PrinterProfile = read(src)?;
    let mut out = Vec::new();
    check_format(&p.format, p.version, PRINTER_FORMAT, &mut out);
    positive(
        &[
            ("bed.size", p.bed.size[0].min(p.bed.size[1])),
            ("bed.max_height", p.bed.max_height),
            ("nozzle.diameter", p.nozzle.diameter),
            ("limits.max_print_speed", p.limits.max_print_speed),
            ("limits.travel_speed", p.limits.travel_speed),
            ("limits.max_z_speed", p.limits.max_z_speed),
            ("layers.layer_height", p.layers.layer_height),
            ("layers.first_layer_height", p.layers.first_layer_height),
        ],
        &mut out,
    );
    let engine = TemplateEngine::new();
    let vars = crate::gcode::sample_template_scope();
    for (field, t) in [("gcode.start", &p.gcode.start), ("gcode.end", &p.gcode.end)] {
        if let Err(e) = engine.check(t, &vars) {
            out.push(
                Issue::error("profile.template_error")
                    .field(field)
                    .detail(format!("{} : {}", e.line, e.message)),
            );
        }
    }
    if out.is_empty() {
        Ok(p)
    } else {
        Err(out)
    }
}

pub fn parse_filament(src: &str) -> Result<FilamentProfile, Vec<Issue>> {
    let f: FilamentProfile = read(src)?;
    let mut out = Vec::new();
    check_format(&f.format, f.version, FILAMENT_FORMAT, &mut out);
    positive(
        &[
            ("filament.diameter", f.filament.diameter),
            ("filament.density", f.filament.density),
            ("filament.flow_multiplier", f.filament.flow_multiplier),
            ("temperature.first_layer", f.temperature.first_layer),
            ("temperature.nozzle", f.temperature.nozzle),
            ("temperature.probe", f.temperature.probe),
            ("speed.nominal", f.speed.nominal),
            ("speed.first_layer", f.speed.first_layer),
            ("speed.max_volumetric", f.speed.max_volumetric),
            ("retraction.speed", f.retraction.speed),
        ],
        &mut out,
    );
    non_negative(
        &[
            ("filament.pressure_advance", f.filament.pressure_advance),
            ("temperature.bed_first_layer", f.temperature.bed_first_layer),
            ("temperature.bed", f.temperature.bed),
            ("temperature.chamber", f.temperature.chamber),
            ("retraction.length", f.retraction.length),
            ("retraction.z_hop", f.retraction.z_hop),
            ("retraction.min_travel", f.retraction.min_travel),
        ],
        &mut out,
    );
    percent(
        &[
            ("cooling.fan", f.cooling.fan),
            ("cooling.first_layer_fan", f.cooling.first_layer_fan),
            ("cooling.bridge_fan", f.cooling.bridge_fan),
        ],
        &mut out,
    );
    if out.is_empty() {
        Ok(f)
    } else {
        Err(out)
    }
}
