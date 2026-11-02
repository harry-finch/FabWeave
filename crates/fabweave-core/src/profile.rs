//! Profils imprimante et filament (TOML, versionnés). Le modèle décrit la
//! géométrie ; les profils apportent la physique (débit, vitesses, plafonds)
//! et le dialecte de la machine (gabarits de démarrage et de fin).
//!
//! Lecture stricte : champ inconnu, valeur hors domaine ou gabarit invalide
//! produisent une erreur à code stable, avec la ligne dans le fichier.
//!
//! Profil imprimante v2 (FabWeave 0.3) : zones exclues du plateau,
//! accélérations, cône de la buse, vignettes, nom par langue, buses
//! proposées et couches par buse, dégagement de la tête (copies), plateau
//! rond. Profil filament v2 : débit maximal par buse. Champs ajoutés tous
//! facultatifs : un profil v1 se lit tel quel (migration sans changement),
//! et un lecteur v1 refuse un profil v2 sur son numéro de version plutôt
//! que sur un champ inconnu.

use serde::{Deserialize, Serialize};

use crate::issue::{line_col, Issue};
use crate::model::Text;
use crate::template::TemplateEngine;

pub const PRINTER_FORMAT: &str = "fabweave.printer";
pub const FILAMENT_FORMAT: &str = "fabweave.filament";
/// Version du profil imprimante (v1 lue sans changement).
pub const PRINTER_VERSION: u32 = 2;
pub const FILAMENT_VERSION: u32 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    /// (0, 0) au coin avant gauche du plateau (Prusa, Bambu, la plupart).
    Corner,
    /// (0, 0) au centre du plateau (delta, certaines Voron).
    Center,
}

/// Dialecte du G-code (voir `gcode.rs`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Firmware {
    /// Prusa Buddy (MK4S, CORE One…) : `M204 P T`, vignettes QOI / PNG.
    Prusa,
    Marlin,
    /// Klipper : `M204 S`, couches annoncées par `SET_PRINT_STATS_INFO`.
    Klipper,
    /// Bambu Lab : en-tête `HEADER_BLOCK`, `M73 L` / `M991`, `M204 S`.
    Bambu,
}

impl Firmware {
    /// Une seule accélération (`M204 S`) : celle des déplacements est
    /// réglée à chaque passage impression ↔ déplacement.
    pub fn single_acceleration(self) -> bool {
        matches!(self, Firmware::Klipper | Firmware::Bambu)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrinterInfo {
    pub id: String,
    /// Nom affiché : chaîne, ou table par langue.
    pub name: Text,
    /// Modèle annoncé au firmware (`M862.3`, `; printer_model`).
    pub model: String,
    pub firmware: Firmware,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Champs à relire avant d'imprimer (`gcode.start`, `gcode.end`) : posé
    /// à l'import depuis un trancheur, retiré par l'utilisateur après
    /// relecture. Tant qu'il en reste, le G-code n'est pas exporté.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub review: Vec<String>,
}

/// Champs qu'un profil importé peut demander de relire.
pub const REVIEW_FIELDS: &[&str] = &["gcode.start", "gcode.end"];

/// Forme du plateau.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BedShape {
    #[default]
    Rectangle,
    /// Plateau rond (delta) : `size = [diamètre, diamètre]`.
    Circle,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bed {
    /// Surface imprimable X, Y (mm) ; diamètre deux fois pour un plateau rond.
    pub size: [f64; 2],
    #[serde(default, skip_serializing_if = "is_default")]
    pub shape: BedShape,
    /// Hauteur d'impression maximale (mm).
    pub max_height: f64,
    pub origin: Origin,
    /// Type de plaque (variable `bed_type` des gabarits ; Bambu Lab :
    /// « Textured PEI Plate », « Cool Plate »…). Facultatif.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub plate: String,
    /// Zones où la pièce ne doit pas passer (purge, essuyage, pinces…).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<Zone>,
}

/// Rectangle du plateau interdit à la pièce (repère machine, mm).
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Zone {
    pub name: Text,
    pub min: [f64; 2],
    pub max: [f64; 2],
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Nozzle {
    pub diameter: f64,
    #[serde(default)]
    pub high_flow: bool,
    #[serde(default)]
    pub abrasive: bool,
    /// Cône de dégagement : pente maximale (degrés depuis l'horizontale)
    /// d'une surface que la buse peut longer sans la toucher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cone_angle: Option<f64>,
    /// Hauteur du cône sous le bloc de chauffe (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cone_height: Option<f64>,
    /// Diamètres proposés par le sélecteur de buse (mm), `diameter` compris.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<f64>,
    /// L'imprimante accepte une buse haut débit (HF) ; une buse abrasive.
    #[serde(default, skip_serializing_if = "is_default")]
    pub high_flow_available: bool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub abrasive_available: bool,
    /// Dégagement de la tête pour l'impression pièce par pièce (copies) :
    /// rayon autour de la buse (mm) et hauteur libre sous le portique (mm).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance_radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clearance_height: Option<f64>,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
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
    /// Accélération d'impression (mm/s²), écrite après le gabarit de
    /// démarrage et prise en compte dans le temps estimé. Sans elle, ni
    /// `M204` ni accélérations dans l'estimation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceleration: Option<f64>,
    /// Accélération des déplacements, sinon `acceleration`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub travel_acceleration: Option<f64>,
    /// Accélération de la première couche, sinon `acceleration`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_layer_acceleration: Option<f64>,
    /// Saut de vitesse dans un angle (mm/s, *jerk*), pour l'estimation ;
    /// 8 mm/s par défaut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jerk: Option<f64>,
    /// Consigne la plus haute de la buse et du plateau (°C, fiche du
    /// fabricant) ; sans elle, les plafonds génériques, signalés.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_nozzle_temperature: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_bed_temperature: Option<f64>,
}

/// Accélérations retenues (mm/s², mm/s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Accelerations {
    pub print: f64,
    pub travel: f64,
    pub first_layer: f64,
    pub jerk: f64,
}

pub const DEFAULT_JERK: f64 = 8.0;
/// Plafonds de température d'un profil qui n'en donne pas (°C) : une buse
/// tout métal et un plateau courants.
pub const DEFAULT_MAX_NOZZLE_TEMPERATURE: f64 = 300.0;
pub const DEFAULT_MAX_BED_TEMPERATURE: f64 = 120.0;

impl MachineLimits {
    /// Plafonds de la buse et du plateau (°C), génériques à défaut.
    pub fn max_temperatures(&self) -> [f64; 2] {
        [
            self.max_nozzle_temperature
                .unwrap_or(DEFAULT_MAX_NOZZLE_TEMPERATURE),
            self.max_bed_temperature
                .unwrap_or(DEFAULT_MAX_BED_TEMPERATURE),
        ]
    }

    pub fn accelerations(&self) -> Option<Accelerations> {
        self.acceleration.map(|a| Accelerations {
            print: a,
            travel: self.travel_acceleration.unwrap_or(a),
            first_layer: self.first_layer_acceleration.unwrap_or(a),
            jerk: self.jerk.unwrap_or(DEFAULT_JERK),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LayerDefaults {
    /// Hauteur de couche conseillée aux modèles (`printer.layer_height`).
    pub layer_height: f64,
    pub first_layer_height: f64,
    /// Hauteurs pour les autres buses (sélecteur de buse).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_nozzle: Vec<NozzleLayers>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NozzleLayers {
    pub diameter: f64,
    pub layer_height: f64,
    /// Sinon celle du profil.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_layer_height: Option<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Templates {
    /// Vignettes intégrées au G-code, `"LxH/FORMAT"` (PNG ou QOI), comme le
    /// réglage `thumbnails` de PrusaSlicer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnails: Vec<String>,
    pub start: String,
    pub end: String,
    /// Pause de l'impression (`path.pause`) : `M601` (Prusa, Marlin),
    /// `M400 U1` (Bambu Lab), `PAUSE` (Klipper) ; variable `pause_text`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pause: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Qoi,
}

/// Vignette demandée par le profil.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThumbnailSpec {
    pub width: usize,
    pub height: usize,
    pub format: ImageFormat,
}

/// Côté maximal d'une vignette (px).
pub const MAX_THUMBNAIL: usize = 1024;

impl ThumbnailSpec {
    pub fn parse(s: &str) -> Option<Self> {
        let (size, format) = s.trim().split_once('/')?;
        let (w, h) = size.split_once('x')?;
        let (width, height) = (w.parse().ok()?, h.parse().ok()?);
        let format = match format {
            "PNG" => ImageFormat::Png,
            "QOI" => ImageFormat::Qoi,
            _ => return None,
        };
        ((1..=MAX_THUMBNAIL).contains(&width) && (1..=MAX_THUMBNAIL).contains(&height)).then_some(
            Self {
                width,
                height,
                format,
            },
        )
    }
}

impl Templates {
    /// Vignettes valides (les autres sont refusées à la lecture du profil).
    pub fn thumbnail_specs(&self) -> Vec<ThumbnailSpec> {
        self.thumbnails
            .iter()
            .filter_map(|t| ThumbnailSpec::parse(t))
            .collect()
    }
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
    /// Nom affiché : chaîne, ou table par langue.
    pub name: Text,
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
    /// Facteur des ponts : multiplie le débit demandé par le modèle sur un
    /// pont (`bridge_to`, `bridge_points`), 1 par défaut.
    #[serde(default = "one")]
    pub bridge_flow: f64,
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
    /// Enceinte minimale avant d'imprimer (ASA, ABS…), 0 : pas d'attente.
    #[serde(default)]
    pub chamber_min: f64,
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
    /// Débit maximal selon la buse (plus grosse buse, buse haut débit).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub by_nozzle: Vec<NozzleFlow>,
}

/// Débit maximal pour une buse : s'applique si `diameter` et `high_flow`
/// (quand ils sont donnés) correspondent ; la plus précise l'emporte.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NozzleFlow {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diameter: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub high_flow: Option<bool>,
    pub max_volumetric: f64,
}

/// Diamètres égaux au micromètre près.
pub fn same_diameter(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

impl Speeds {
    /// Débit maximal pour cette buse.
    pub fn max_volumetric_for(&self, diameter: f64, high_flow: bool) -> f64 {
        self.by_nozzle
            .iter()
            .filter(|e| e.diameter.is_none_or(|d| same_diameter(d, diameter)))
            .filter(|e| e.high_flow.is_none_or(|h| h == high_flow))
            // Plus précise d'abord ; à égalité, la première du fichier
            // (`max_by_key` garde la dernière : parcours à rebours).
            .rev()
            .max_by_key(|e| usize::from(e.diameter.is_some()) + usize::from(e.high_flow.is_some()))
            .map_or(self.max_volumetric, |e| e.max_volumetric)
    }
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
    latest: u32,
    out: &mut Vec<Issue>,
) {
    if format.as_deref().is_some_and(|f| f != expected) {
        out.push(
            Issue::error("profile.wrong_format")
                .field("format")
                .detail(format.clone().unwrap_or_default()),
        );
    }
    if version.is_some_and(|v| v == 0 || v > latest) {
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
    let p = read_printer(src)?;
    let out = check_printer(&p);
    if out.is_empty() {
        Ok(p)
    } else {
        Err(out)
    }
}

/// Lecture seule (syntaxe et champs), sans contrôler les valeurs : un
/// profil lisible mais faux s'ouvre dans l'éditeur pour être corrigé.
pub fn read_printer(src: &str) -> Result<PrinterProfile, Vec<Issue>> {
    read(src)
}

pub fn read_filament(src: &str) -> Result<FilamentProfile, Vec<Issue>> {
    read(src)
}

/// Contrôle des valeurs d'un profil imprimante (domaines, zones, gabarits).
pub fn check_printer(p: &PrinterProfile) -> Vec<Issue> {
    let mut out = Vec::new();
    control_characters(
        &[
            ("printer.name", p.printer.name.has_control()),
            ("printer.model", has_control(&p.printer.model)),
            ("bed.plate", has_control(&p.bed.plate)),
        ],
        &mut out,
    );
    check_format(
        &p.format,
        p.version,
        PRINTER_FORMAT,
        PRINTER_VERSION,
        &mut out,
    );
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
    let l = &p.limits;
    let optional = [
        ("limits.acceleration", l.acceleration),
        ("limits.travel_acceleration", l.travel_acceleration),
        (
            "limits.first_layer_acceleration",
            l.first_layer_acceleration,
        ),
        ("limits.jerk", l.jerk),
        ("limits.max_nozzle_temperature", l.max_nozzle_temperature),
        ("limits.max_bed_temperature", l.max_bed_temperature),
        ("nozzle.cone_height", p.nozzle.cone_height),
    ];
    let present: Vec<_> = optional
        .iter()
        .filter_map(|(f, v)| v.map(|v| (*f, v)))
        .collect();
    positive(&present, &mut out);
    if l.acceleration.is_none()
        && (l.travel_acceleration.is_some() || l.first_layer_acceleration.is_some())
    {
        out.push(Issue::error("profile.missing_acceleration").field("limits.acceleration"));
    }
    if let Some(a) = p.nozzle.cone_angle {
        if !(a > 0.0 && a < 90.0) {
            out.push(
                Issue::error("profile.out_of_range")
                    .field("nozzle.cone_angle")
                    .value(a),
            );
        }
    }
    for (i, z) in p.bed.exclude.iter().enumerate() {
        let ok =
            (0..2).all(|k| z.min[k].is_finite() && z.max[k].is_finite() && z.min[k] < z.max[k]);
        if !ok {
            out.push(
                Issue::error("profile.invalid_zone")
                    .field(format!("bed.exclude[{i}]"))
                    .detail(z.name.get("fr").to_string()),
            );
        }
    }
    if p.bed.shape == BedShape::Circle && !same_diameter(p.bed.size[0], p.bed.size[1]) {
        out.push(Issue::error("profile.invalid_bed").field("bed.size"));
    }
    let n = &p.nozzle;
    let mut sizes: Vec<(&str, f64)> = n.choices.iter().map(|d| ("nozzle.choices", *d)).collect();
    sizes.extend(n.clearance_radius.map(|v| ("nozzle.clearance_radius", v)));
    sizes.extend(n.clearance_height.map(|v| ("nozzle.clearance_height", v)));
    for e in &p.layers.by_nozzle {
        sizes.push(("layers.by_nozzle.diameter", e.diameter));
        sizes.push(("layers.by_nozzle.layer_height", e.layer_height));
        sizes.extend(
            e.first_layer_height
                .map(|v| ("layers.by_nozzle.first_layer_height", v)),
        );
    }
    positive(&sizes, &mut out);
    for t in &p.gcode.thumbnails {
        if ThumbnailSpec::parse(t).is_none() {
            out.push(
                Issue::error("profile.invalid_thumbnail")
                    .field("gcode.thumbnails")
                    .detail(t.clone()),
            );
        }
    }
    let engine = TemplateEngine::new();
    let vars = crate::gcode::sample_template_scope();
    for (field, t) in [
        ("gcode.start", &p.gcode.start),
        ("gcode.end", &p.gcode.end),
        ("gcode.pause", &p.gcode.pause),
    ] {
        if let Err(e) = engine.check(t, &vars) {
            out.push(
                Issue::error("profile.template_error")
                    .field(field)
                    .detail(format!("{} : {}", e.line, e.message)),
            );
        }
    }
    for r in &p.printer.review {
        if !REVIEW_FIELDS.contains(&r.as_str()) {
            out.push(
                Issue::error("profile.invalid_review")
                    .field("printer.review")
                    .detail(r.clone()),
            );
        }
    }
    out
}

pub fn parse_filament(src: &str) -> Result<FilamentProfile, Vec<Issue>> {
    let f = read_filament(src)?;
    let out = check_filament(&f);
    if out.is_empty() {
        Ok(f)
    } else {
        Err(out)
    }
}

/// Contrôle des valeurs d'un profil filament.
pub fn check_filament(f: &FilamentProfile) -> Vec<Issue> {
    let mut out = Vec::new();
    control_characters(
        &[
            ("filament.name", f.filament.name.has_control()),
            ("filament.type", has_control(&f.filament.material)),
        ],
        &mut out,
    );
    check_format(
        &f.format,
        f.version,
        FILAMENT_FORMAT,
        FILAMENT_VERSION,
        &mut out,
    );
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
    if !(f.filament.bridge_flow > 0.0 && f.filament.bridge_flow <= 2.0) {
        out.push(
            Issue::error("profile.out_of_range")
                .field("filament.bridge_flow")
                .value(f.filament.bridge_flow),
        );
    }
    let flows: Vec<(&str, f64)> = f
        .speed
        .by_nozzle
        .iter()
        .flat_map(|e| {
            [
                ("speed.by_nozzle.max_volumetric", Some(e.max_volumetric)),
                ("speed.by_nozzle.diameter", e.diameter),
            ]
        })
        .filter_map(|(k, v)| v.map(|v| (k, v)))
        .collect();
    positive(&flows, &mut out);
    if let Some(idle) = f.temperature.idle {
        positive(&[("temperature.idle", idle)], &mut out);
    }
    non_negative(
        &[
            ("filament.pressure_advance", f.filament.pressure_advance),
            ("temperature.bed_first_layer", f.temperature.bed_first_layer),
            ("temperature.bed", f.temperature.bed),
            ("temperature.chamber", f.temperature.chamber),
            ("temperature.chamber_min", f.temperature.chamber_min),
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
    out
}

fn has_control(s: &str) -> bool {
    s.chars().any(char::is_control)
}

/// Textes recopiés dans le G-code : un retour à la ligne y ouvrirait une
/// commande.
fn control_characters(fields: &[(&'static str, bool)], out: &mut Vec<Issue>) {
    for (field, bad) in fields {
        if *bad {
            out.push(Issue::error("profile.control_character").field(*field));
        }
    }
}

/// Profil imprimante écrit en TOML (format et version courants), relu tel
/// quel par [`parse_printer`]. Les commentaires du fichier d'origine ne sont
/// pas conservés.
pub fn printer_toml(p: &PrinterProfile) -> String {
    let mut p = p.clone();
    p.format = Some(PRINTER_FORMAT.into());
    p.version = Some(PRINTER_VERSION);
    toml::to_string(&p).expect("profil sérialisable")
}

pub fn filament_toml(f: &FilamentProfile) -> String {
    let mut f = f.clone();
    f.format = Some(FILAMENT_FORMAT.into());
    f.version = Some(FILAMENT_VERSION);
    toml::to_string(&f).expect("profil sérialisable")
}
