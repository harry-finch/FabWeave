//! Écriture du G-code à partir du plan : vignettes, gabarit de démarrage,
//! accélérations, parcours en extrusion relative (`M83`) avec progression
//! (`M73 P R`) et changements de couche, gabarit de fin, métadonnées lues
//! par les imprimantes Prusa (temps estimé, filament).
//!
//! Couches : les parcours continus ou non planaires n'en ont pas ; une couche
//! commence quand l'impression dépasse d'une hauteur de couche du profil le
//! début de la précédente (paliers de hauteur atteinte), et le compte repart
//! du plateau à chaque copie imprimée pièce par pièce. Chaque dialecte les
//! annonce à sa façon :
//! - Prusa, Marlin : commentaires `;LAYER_CHANGE`, `;Z:`, `;HEIGHT:` (ceux de
//!   PrusaSlicer, lus par les visionneuses et OctoPrint) ;
//! - Klipper : `SET_PRINT_STATS_INFO` (couche courante dans Mainsail, Fluidd) ;
//! - Bambu Lab : en-tête `HEADER_BLOCK` lu par le firmware (durée, nombre de
//!   couches, filament), puis `M73 L` / `M991` (compteur à l'écran).
//!
//! Accélérations (si le profil les donne) : `M204 P T` (Prusa, Marlin :
//! impression et déplacements), `M204 S` réécrit à chaque changement
//! (Klipper, Bambu Lab : une seule accélération) ; celle de la première
//! couche, puis celle d'impression.
//!
//! Machine à états : un axe, une vitesse `F` ou une consigne de ventilateur
//! n'est écrit que s'il change.

use std::fmt::Write;

use indexmap::IndexMap;
use rhai::Scope;

use crate::format::{duration, num};
use crate::image;
use crate::issue::Issue;
use crate::model::Model;
use crate::params::ParamValue;
use crate::path::{MoveKind, Toolpath};
use crate::plan::Plan;
use crate::profile::{Accelerations, FilamentProfile, Firmware, ImageFormat, PrinterProfile};
use crate::setup::{Adhesion, Setup};
use crate::template::TemplateEngine;

/// Tout ce qu'il faut pour écrire un G-code.
pub struct Job<'a> {
    pub model: &'a Model,
    pub params: &'a IndexMap<String, ParamValue>,
    pub path: &'a Toolpath,
    pub plan: &'a Plan,
    pub printer: &'a PrinterProfile,
    pub filament: &'a FilamentProfile,
    /// Préparation choisie (rappelée en commentaire).
    pub setup: &'a Setup,
}

macro_rules! line {
    ($g:expr, $($arg:tt)*) => {{
        let _ = writeln!($g, $($arg)*);
    }};
}

/// Variables des gabarits pour une impression donnée.
pub fn template_scope(
    printer: &PrinterProfile,
    filament: &FilamentProfile,
    plan: &Plan,
    model_name: &str,
    layers: usize,
) -> Scope<'static> {
    let s = &plan.stats;
    scope(
        printer,
        filament,
        [s.first_layer_bounds.min, s.first_layer_bounds.max],
        [s.bounds.min, s.bounds.max],
        model_name,
        [layers as f64, s.time],
    )
}

/// Variables des gabarits avec des valeurs d'exemple (contrôle d'un profil).
pub fn sample_template_scope() -> Scope<'static> {
    let mut s = Scope::new();
    for name in [
        "printer_model",
        "printer_name",
        "filament_type",
        "filament_name",
        "model_name",
        "bed_type",
    ] {
        s.push_constant(name, String::new());
    }
    for name in ["nozzle_high_flow", "nozzle_abrasive", "filament_abrasive"] {
        s.push_constant(name, false);
    }
    for name in FLOAT_VARIABLES {
        s.push_constant(*name, 0.0_f64);
    }
    s
}

/// Variables numériques des gabarits (documentées dans `docs/WEAVE_FORMAT.md`).
const FLOAT_VARIABLES: &[&str] = &[
    "nozzle_diameter",
    "bed_size_x",
    "bed_size_y",
    "max_print_height",
    "first_layer_temperature",
    "temperature",
    "first_layer_bed_temperature",
    "bed_temperature",
    "probe_temperature",
    "idle_temperature",
    "chamber_temperature",
    "pressure_advance",
    "retraction_length",
    "retraction_speed",
    "first_layer_min_x",
    "first_layer_min_y",
    "first_layer_max_x",
    "first_layer_max_y",
    "print_min_x",
    "print_min_y",
    "print_max_x",
    "print_max_y",
    "max_z",
    "max_volumetric_speed",
    "acceleration",
    "travel_acceleration",
    "first_layer_acceleration",
    "total_layer_count",
    "estimated_time",
];

fn scope(
    p: &PrinterProfile,
    f: &FilamentProfile,
    first_layer: [[f64; 3]; 2],
    print: [[f64; 3]; 2],
    model_name: &str,
    [layers, time]: [f64; 2],
) -> Scope<'static> {
    let mut s = Scope::new();
    s.push_constant("printer_model", p.printer.model.clone());
    s.push_constant("printer_name", p.printer.name.get("fr").to_string());
    s.push_constant("filament_type", f.filament.material.clone());
    s.push_constant("filament_name", f.filament.name.get("fr").to_string());
    s.push_constant("model_name", model_name.to_string());
    s.push_constant("bed_type", p.bed.plate.clone());
    s.push_constant("nozzle_high_flow", p.nozzle.high_flow);
    s.push_constant("nozzle_abrasive", p.nozzle.abrasive);
    s.push_constant("filament_abrasive", f.filament.abrasive);
    let values = [
        p.nozzle.diameter,
        p.bed.size[0],
        p.bed.size[1],
        p.bed.max_height,
        f.temperature.first_layer,
        f.temperature.nozzle,
        f.temperature.bed_first_layer,
        f.temperature.bed,
        f.temperature.probe,
        f.temperature.idle.unwrap_or(f.temperature.probe),
        f.temperature.chamber,
        f.filament.pressure_advance,
        f.retraction.length,
        f.retraction.speed,
        first_layer[0][0],
        first_layer[0][1],
        first_layer[1][0],
        first_layer[1][1],
        print[0][0],
        print[0][1],
        print[1][0],
        print[1][1],
        print[1][2],
        f.speed.max_volumetric,
        p.limits.acceleration.unwrap_or(0.0),
        p.limits.accelerations().map_or(0.0, |a| a.travel),
        p.limits.accelerations().map_or(0.0, |a| a.first_layer),
        layers,
        time,
    ];
    for (name, v) in FLOAT_VARIABLES.iter().zip(values) {
        s.push_constant(*name, v);
    }
    s
}

fn render(engine: &TemplateEngine, field: &str, t: &str, vars: &Scope) -> Result<String, Issue> {
    engine
        .render(t, vars)
        .map(|mut s| {
            if !s.ends_with('\n') {
                s.push('\n');
            }
            s
        })
        .map_err(|e| {
            Issue::error("profile.template_error")
                .field(field.to_string())
                .detail(format!("{} : {}", e.line, e.message))
        })
}

fn param_text(v: &ParamValue) -> String {
    match v {
        ParamValue::Float(f) => num(*f, 4),
        ParamValue::Int(i) => i.to_string(),
        ParamValue::Bool(b) => b.to_string(),
        ParamValue::Choice(s) => s.clone(),
    }
}

/// Position et consignes déjà écrites.
struct State {
    x: String,
    y: String,
    z: String,
    f: i64,
    fan: Option<u8>,
    retracted: bool,
}

impl State {
    /// `G1` avec les seuls axes qui changent (et `E`, `F` éventuels).
    fn g1(&mut self, g: &mut String, xyz: [Option<f64>; 3], e: Option<f64>, speed: Option<f64>) {
        let mut cmd = String::from("G1");
        for (i, v) in xyz.iter().enumerate() {
            if let Some(v) = v {
                let s = num(*v, 3);
                let last = match i {
                    0 => &mut self.x,
                    1 => &mut self.y,
                    _ => &mut self.z,
                };
                if *last != s {
                    cmd.push_str([" X", " Y", " Z"][i]);
                    cmd.push_str(&s);
                    *last = s;
                }
            }
        }
        if let Some(e) = e {
            cmd.push_str(" E");
            cmd.push_str(&num(e, 5));
        }
        if let Some(v) = speed {
            let f = (v * 60.0).round() as i64;
            if f != self.f {
                cmd.push_str(&format!(" F{f}"));
                self.f = f;
            }
        }
        if cmd.len() > 2
            && !(cmd.contains('X') || cmd.contains('Y') || cmd.contains('Z') || cmd.contains('E'))
        {
            // Seule la vitesse change : rien à écrire.
            return;
        }
        if cmd.len() > 2 {
            g.push_str(&cmd);
            g.push('\n');
        }
    }

    fn fan(&mut self, g: &mut String, percent: u8) {
        if self.fan == Some(percent) {
            return;
        }
        if percent == 0 {
            line!(g, "M107");
        } else {
            // Arrondi exact en entiers (50 % → 128, et non 127 par 50 × 2,55 flottant).
            line!(g, "M106 S{}", (u32::from(percent) * 255 + 50) / 100);
        }
        self.fan = Some(percent);
    }
}

/// Indices des mouvements qui ouvrent une couche, au sens du compteur de
/// l'imprimante (voir l'en-tête du module).
pub fn layer_starts(job: &Job) -> Vec<usize> {
    let lh = job.printer.layers.layer_height;
    let mut out = Vec::new();
    let mut top: Option<f64> = None;
    let mut restart = false;
    for (i, m) in job.plan.moves.iter().enumerate().skip(1) {
        let w = &job.path.points[i];
        restart |= job.path.starts.contains(&i);
        if m.skipped || w.kind == MoveKind::Travel {
            continue;
        }
        if restart {
            // Début d'une pièce : copie suivante (pièce par pièce), on repart
            // du plateau ; mais la première pièce, posée à la hauteur de la
            // bordure, reste dans la même couche.
            restart = false;
            if top.is_some_and(|t| w.z < t - 0.5 * lh) {
                top = None;
            }
        }
        if top.is_none_or(|t| w.z >= t + 0.99 * lh) {
            out.push(i);
            top = Some(w.z);
        }
    }
    out
}

/// Progression `M73 P R` (pourcentage écoulé, minutes restantes), écrite
/// quand l'un des deux change (Prusa, Marlin, Klipper, Bambu Lab).
struct Progress {
    total: f64,
    last: Option<(u32, u32)>,
}

impl Progress {
    fn at(&mut self, g: &mut String, elapsed: f64, force: bool) {
        let pct = if self.total > 0.0 {
            (elapsed / self.total * 100.0).floor().clamp(0.0, 100.0)
        } else {
            100.0
        } as u32;
        let left = ((self.total - elapsed).max(0.0) / 60.0).ceil() as u32;
        if force || self.last != Some((pct, left)) {
            line!(g, "M73 P{pct} R{left}");
            self.last = Some((pct, left));
        }
    }
}

/// Accélération en cours (dialectes à une seule accélération).
struct Accel {
    acc: Accelerations,
    single: bool,
    current: f64,
}

impl Accel {
    fn set(&mut self, g: &mut String, travel: bool, first_layer: bool) {
        let a = if travel && self.single {
            self.acc.travel
        } else if first_layer {
            self.acc.first_layer
        } else {
            self.acc.print
        };
        if a == self.current {
            return;
        }
        if self.single {
            line!(g, "M204 S{}", num(a, 0));
        } else {
            line!(g, "M204 P{} T{}", num(a, 0), num(self.acc.travel, 0));
        }
        self.current = a;
    }
}

/// Vignettes du profil en commentaires (format de PrusaSlicer : base64 en
/// lignes de 78 caractères).
fn thumbnails(g: &mut String, job: &Job) {
    let specs = job.printer.gcode.thumbnail_specs();
    if specs.is_empty() {
        return;
    }
    let bambu = job.printer.printer.firmware == Firmware::Bambu;
    if bambu {
        line!(g, "; THUMBNAIL_BLOCK_START");
    }
    for spec in specs {
        let tag = match spec.format {
            ImageFormat::Png => "thumbnail",
            ImageFormat::Qoi => "thumbnail_QOI",
        };
        let b64 = image::base64(&image::encode(job.path, job.plan, spec));
        line!(g, ";");
        line!(
            g,
            "; {tag} begin {}x{} {}",
            spec.width,
            spec.height,
            b64.len()
        );
        for chunk in b64.as_bytes().chunks(78) {
            line!(g, "; {}", std::str::from_utf8(chunk).unwrap_or_default());
        }
        line!(g, "; {tag} end");
        line!(g, ";");
    }
    if bambu {
        line!(g, "; THUMBNAIL_BLOCK_END");
    }
    g.push('\n');
}

pub fn emit(job: &Job) -> Result<String, Vec<Issue>> {
    let (p, f, plan) = (job.printer, job.filament, job.plan);
    let r = &f.retraction;
    let lim = &p.limits;
    let engine = TemplateEngine::new();
    let model_name = job.model.meta.name.get("fr").to_string();
    let layers = layer_starts(job);
    let vars = template_scope(p, f, plan, &model_name, layers.len());
    let start = render(&engine, "gcode.start", &p.gcode.start, &vars);
    let end = render(&engine, "gcode.end", &p.gcode.end, &vars);
    let (start, end) = match (start, end) {
        (Ok(s), Ok(e)) => (s, e),
        (s, e) => return Err([s.err(), e.err()].into_iter().flatten().collect()),
    };

    let mut g = String::with_capacity(plan.moves.len() * 36 + start.len() + end.len() + 2048);
    let s = &plan.stats;
    let firmware = p.printer.firmware;
    let bambu = firmware == Firmware::Bambu;
    if bambu {
        line!(g, "; HEADER_BLOCK_START");
        line!(g, "; FabWeave {}", crate::version());
        line!(
            g,
            "; model printing time: {0}; total estimated time: {0}",
            duration(s.time)
        );
        line!(g, "; total layer number: {}", layers.len());
        line!(
            g,
            "; total filament length [mm] : {}",
            num(s.filament_length, 2)
        );
        line!(
            g,
            "; total filament volume [cm^3] : {}",
            num(s.filament_volume, 2)
        );
        line!(
            g,
            "; total filament weight [g] : {}",
            num(s.filament_mass, 2)
        );
        line!(g, "; filament_density: {}", num(f.filament.density, 2));
        line!(g, "; filament_diameter: {}", num(f.filament.diameter, 2));
        line!(g, "; max_z_height: {}", num(s.bounds.max[2], 2));
        line!(g, "; HEADER_BLOCK_END");
        g.push('\n');
    } else {
        line!(g, "; generated by FabWeave {}", crate::version());
        g.push('\n');
    }
    thumbnails(&mut g, job);
    if bambu {
        line!(g, "; generated by FabWeave {}", crate::version());
    }
    line!(g, "; model = {}", model_name);
    for (k, v) in job.params {
        line!(g, "; param {k} = {}", param_text(v));
    }
    line!(g, "; printer = {}", p.printer.name.get("fr"));
    line!(g, "; filament = {}", f.filament.name.get("fr"));
    line!(
        g,
        "; nozzle = {}{}{}",
        num(p.nozzle.diameter, 2),
        if p.nozzle.high_flow { " HF" } else { "" },
        if p.nozzle.abrasive { " abrasive" } else { "" }
    );
    let pl = &job.setup.placement;
    if pl.rotation != 0.0 {
        line!(g, "; rotation = {}", num(pl.rotation, 2));
    }
    if pl.copies != [1, 1] {
        line!(
            g,
            "; copies = {} x {} (pièce par pièce)",
            pl.copies[0],
            pl.copies[1]
        );
    }
    match job.setup.adhesion {
        Adhesion::None => {}
        Adhesion::Brim { width, gap } => {
            line!(
                g,
                "; adhesion = brim, width {} mm, gap {} mm",
                num(width, 2),
                num(gap, 2)
            )
        }
        Adhesion::Raft {
            layers,
            gap,
            margin,
        } => line!(
            g,
            "; adhesion = raft, {layers} layers, gap {} mm, margin {} mm",
            num(gap, 2),
            num(margin, 2)
        ),
    }
    g.push('\n');
    g.push_str(&start);
    g.push('\n');
    line!(g, "; --- FabWeave : parcours ---");
    let mut progress = Progress {
        total: s.time,
        last: None,
    };
    progress.at(&mut g, 0.0, true);
    if firmware == Firmware::Klipper {
        line!(g, "SET_PRINT_STATS_INFO TOTAL_LAYER={}", layers.len());
    }
    let mut accel = lim.accelerations().map(|acc| Accel {
        acc,
        single: firmware.single_acceleration(),
        current: f64::NAN,
    });
    if let Some(a) = accel.as_mut() {
        a.set(&mut g, true, true);
    }
    let mut next_layer = layers.iter().copied().enumerate().peekable();
    let mut layer_z = 0.0;
    let mut elapsed = 0.0;

    let off = plan.offset;
    let pts = &job.path.points;
    let mut st = State {
        x: String::new(),
        y: String::new(),
        z: String::new(),
        f: -1,
        fan: None,
        retracted: false,
    };
    let travel = lim.travel_speed;
    let z_speed = lim.max_z_speed;

    // Déplacement initial : rétraction, montée, déplacement, descente.
    let p0 = [pts[0].x + off[0], pts[0].y + off[1], pts[0].z + off[2]];
    if r.length > 0.0 {
        st.g1(&mut g, [None; 3], Some(-r.length), Some(r.speed));
        st.retracted = true;
    }
    st.g1(
        &mut g,
        [None, None, Some(p0[2] + r.z_hop.max(0.2))],
        None,
        Some(z_speed),
    );
    st.g1(&mut g, [Some(p0[0]), Some(p0[1]), None], None, Some(travel));
    st.g1(&mut g, [None, None, Some(p0[2])], None, Some(z_speed));
    let mut cur = p0;
    let mut temps_switched = false;

    for (i, m) in plan.moves.iter().enumerate().skip(1) {
        if m.skipped {
            continue;
        }
        let w = &pts[i];
        let b = [w.x + off[0], w.y + off[1], w.z + off[2]];
        if let Some(&(n, _)) = next_layer.peek().filter(|(_, at)| *at == i) {
            next_layer.next();
            match firmware {
                Firmware::Bambu => {
                    line!(g, "; CHANGE_LAYER");
                    line!(g, "; Z_HEIGHT: {}", num(b[2], 3));
                    line!(
                        g,
                        "; layer num/total_layer_count: {}/{}",
                        n + 1,
                        layers.len()
                    );
                    line!(g, "M73 L{}", n + 1);
                    line!(g, "M991 S0 P{n} ; notify layer change");
                }
                Firmware::Klipper => {
                    line!(g, "SET_PRINT_STATS_INFO CURRENT_LAYER={}", n + 1);
                }
                Firmware::Prusa | Firmware::Marlin => {
                    line!(g, ";LAYER_CHANGE");
                    line!(g, ";Z:{}", num(b[2], 3));
                    line!(g, ";HEIGHT:{}", num(b[2] - layer_z, 3));
                }
            }
            layer_z = b[2];
            progress.at(&mut g, elapsed, bambu);
        } else {
            progress.at(&mut g, elapsed, false);
        }
        if let Some(a) = accel.as_mut() {
            a.set(&mut g, w.kind == MoveKind::Travel, m.first_layer);
        }
        elapsed += m.time;
        if w.kind == MoveKind::Travel {
            if m.retract {
                if !st.retracted && r.length > 0.0 {
                    st.g1(&mut g, [None; 3], Some(-r.length), Some(r.speed));
                    st.retracted = true;
                }
                let hop = cur[2].max(b[2]) + r.z_hop;
                st.g1(&mut g, [None, None, Some(hop)], None, Some(z_speed));
                st.g1(&mut g, [Some(b[0]), Some(b[1]), None], None, Some(travel));
                st.g1(&mut g, [None, None, Some(b[2])], None, Some(z_speed));
            } else {
                st.g1(
                    &mut g,
                    [Some(b[0]), Some(b[1]), Some(b[2])],
                    None,
                    Some(travel),
                );
            }
        } else {
            if st.retracted {
                st.g1(&mut g, [None; 3], Some(r.length), Some(r.speed));
                st.retracted = false;
            }
            if !m.first_layer && !temps_switched {
                if f.temperature.nozzle != f.temperature.first_layer {
                    line!(
                        g,
                        "M104 S{} ; température après la première couche",
                        num(f.temperature.nozzle, 1)
                    );
                }
                if f.temperature.bed != f.temperature.bed_first_layer {
                    line!(g, "M140 S{}", num(f.temperature.bed, 1));
                }
                temps_switched = true;
            }
            st.fan(&mut g, m.fan);
            st.g1(
                &mut g,
                [Some(b[0]), Some(b[1]), Some(b[2])],
                Some(m.e),
                Some(m.speed),
            );
        }
        cur = b;
    }
    if st.retracted && r.length > 0.0 {
        // Le gabarit de fin suppose un filament détendu.
        st.g1(&mut g, [None; 3], Some(r.length), Some(r.speed));
    }

    progress.at(&mut g, s.time, false);
    line!(g, "; --- FabWeave : fin du parcours ---");
    g.push('\n');
    g.push_str(&end);
    g.push('\n');
    line!(g, "; filament used [mm] = {}", num(s.filament_length, 2));
    line!(g, "; filament used [cm3] = {}", num(s.filament_volume, 2));
    line!(g, "; filament used [g] = {}", num(s.filament_mass, 2));
    line!(g, "; total filament used [g] = {}", num(s.filament_mass, 2));
    line!(
        g,
        "; estimated printing time (normal mode) = {}",
        duration(s.time)
    );
    line!(g, "; filament_type = {}", f.filament.material);
    line!(g, "; nozzle_diameter = {}", num(p.nozzle.diameter, 2));
    line!(g, "; printer_model = {}", p.printer.model);
    line!(g, "; max_layer_z = {}", num(s.bounds.max[2], 3));
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{filament, printer};
    use crate::{model, plan, script};

    const SQUARE: &str = r#"[meta]
name = "Carré"
category = "Essais"
[params.side]
type = "float"
label = "Côté"
default = 20
min = 5
max = 50
---
let p = new_path();
let s = params.side / 2;
let h = printer.first_layer_height;
p.travel_to(-s, -s, h);
p.print_to(s, -s, h, 0.45, h);
p.print_to(s, s, h, 0.45, h);
p.print_to(s, s, h, 0.45, h);
p.travel_to(-s, s, h);
p.print_to(-s, -s, h, 0.45, h);
p.print_to(-s, -s, 0.4, 0.45, 0.2);
p
"#;

    fn square_gcode() -> String {
        let m = model::parse(SQUARE).unwrap();
        let eng = script::ScriptEngine::new(script::Limits::default());
        let c = eng.compile(&m).unwrap();
        let (params, _) = crate::params::resolve(&m.params, &serde_json::Map::new());
        let (pr, fi) = (printer(), filament());
        let ctx = script::PrinterContext {
            nozzle_diameter: pr.nozzle.diameter,
            layer_height: pr.layers.layer_height,
            first_layer_height: pr.layers.first_layer_height,
            nozzle_cone_angle: 0.0,
            nozzle_cone_height: 0.0,
        };
        let out = eng.run(&c, &params, &ctx);
        let path = out.path.unwrap();
        let (pl, _) = plan::plan(&path, &pr, &fi);
        let pl = pl.unwrap();
        emit(&Job {
            model: &m,
            params: &params,
            path: &path,
            plan: &pl,
            printer: &pr,
            filament: &fi,
            setup: &Setup::default(),
        })
        .unwrap()
    }

    #[test]
    fn writes_start_moves_and_metadata() {
        let g = square_gcode();
        assert!(g.starts_with("; generated by FabWeave"));
        assert!(g.contains("; param side = 20\n"));
        assert!(g.contains("M862.3 P \"MK4S\""));
        assert!(g.contains("M83 "));
        // Carré centré sur le plateau 250 × 210 : coin (115, 95).
        assert!(g.contains("G1 X115 Y95\n") || g.contains("G1 X115 Y95 F"));
        // Mouvement imprimé : X seul change, E et F présents.
        assert!(g.contains("G1 X135 E"));
        // Point confondu ignoré ; déplacement de 20 mm : rétraction + saut en Z.
        assert!(g.contains("G1 E-0.7 F2100\n"));
        assert!(g.contains("; estimated printing time (normal mode) = "));
        assert!(g.contains("; printer_model = MK4S\n"));
        assert!(g.contains("M104 S210 ; température après la première couche"));
    }

    /// Lignes du parcours (entre les deux repères), valeurs de `E` masquées
    /// sur les mouvements imprimés, progression `M73` retirée (testée à part).
    fn path_lines(body: &str, printer_src: &str) -> Vec<String> {
        let src = format!("[meta]\nname = \"t\"\ncategory = \"t\"\n---\n{body}");
        let mut w = crate::Workbench::default();
        w.set_printer(printer_src).unwrap();
        w.set_filament(crate::testing::PLA).unwrap();
        w.load_model(&src).unwrap();
        let ev = w.evaluate(&serde_json::Map::new());
        let g = w.gcode(&ev).unwrap();
        let start = g.find("; --- FabWeave : parcours ---").unwrap();
        let end = g.find("; --- FabWeave : fin du parcours ---").unwrap();
        g[start..end]
            .lines()
            .skip(1)
            .filter(|l| !l.starts_with("M73 "))
            .map(|l| {
                let moves = l.contains(" X") || l.contains(" Y") || l.contains(" Z");
                if moves && l.contains(" E") {
                    l.split(' ')
                        .map(|p| if p.starts_with('E') { "E*" } else { p })
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    l.to_string()
                }
            })
            .collect()
    }

    #[test]
    fn travels_fan_temperatures_and_feedrates_follow_the_state() {
        let lines = path_lines(
            "let p = new_path();
p.travel_to(0, 0, 0.2);
p.print_to(10, 0, 0.2, 0.45, 0.2);
p.print_to(10, 1, 0.2, 0.45, 0.2);
p.travel_to(10.5, 1, 0.2);
p.print_to(20, 1, 0.2, 0.45, 0.2);
p.travel_to(20, 20, 0.4);
p.set_fan(50);
p.print_to(30, 20, 0.4, 0.45, 0.2);
p.reset_fan();
p.print_to(30, 30, 0.4, 0.45, 0.2);
p.print_to(40, 30, 0.6, 0.45, 0.2);
p",
            crate::testing::MK4S,
        );
        // Encombrement 40 × 30 centré sur le plateau 250 × 210 : décalage (105, 90).
        let expected = [
            // Accélérations de la première couche (impression, déplacements).
            "M204 P500 T4000",
            // Déplacement initial : rétraction, montée, déplacement, descente.
            "G1 E-0.7 F2100",
            "G1 Z0.4 F720",
            "G1 X105 Y90 F10800",
            "G1 Z0.2 F720",
            // Première couche : annoncée comme par PrusaSlicer.
            ";LAYER_CHANGE",
            ";Z:0.2",
            ";HEIGHT:0.2",
            // Filament détendu, ventilateur coupé, 20 mm/s.
            "G1 E0.7 F2100",
            "M107",
            "G1 X115 E* F1200",
            "G1 Y91 E*",
            // Déplacement court (0,5 mm < 1,5 mm) : ni rétraction ni saut.
            "G1 X115.5 F10800",
            "G1 X125 E* F1200",
            // Hors de la première couche : accélération d'impression.
            "M204 P1500 T4000",
            // Déplacement long : rétraction, saut au-dessus du plus haut des deux Z.
            "G1 E-0.7 F2100",
            "G1 Z0.6 F720",
            "G1 Y110 F10800",
            "G1 Z0.4 F720",
            ";LAYER_CHANGE",
            ";Z:0.4",
            ";HEIGHT:0.2",
            // Deuxième couche : température de croisière une seule fois,
            // ventilateur du script (50 %), vitesse nominale 40 mm/s.
            "G1 E0.7 F2100",
            "M104 S210 ; température après la première couche",
            "M106 S128",
            "G1 X135 E* F2400",
            // Retour au ventilateur du filament (100 %).
            "M106 S255",
            "G1 Y120 E*",
            // Montée en imprimant : une couche de plus dès 0,2 mm gagné.
            ";LAYER_CHANGE",
            ";Z:0.6",
            ";HEIGHT:0.2",
            "G1 X145 Z0.6 E*",
        ];
        assert_eq!(lines, expected);
    }

    #[test]
    fn core_one_templates_render_chamber_and_park() {
        let src = "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet p = new_path();\np.travel_to(0, 0, 0.2);\np.print_to(10, 0, 0.2, 0.45, 0.2);\np";
        let mut w = crate::Workbench::default();
        w.set_printer(crate::testing::CORE_ONE).unwrap();
        w.set_filament(crate::testing::PETG).unwrap();
        w.load_model(src).unwrap();
        let g = w.gcode(&w.evaluate(&serde_json::Map::new())).unwrap();
        assert!(g.contains("M862.3 P \"COREONE\""));
        assert!(g.contains("M141 S35 ; set nominal chamber temp"));
        assert!(g.contains("M104 T0 S130\n"));
        // Plateau à 85 °C : pas de ventilateur pendant la chauffe.
        assert!(!g.contains("M106 S70"));
        assert!(g.contains("M572 S0.053 ; pressure advance"));
        assert!(g.contains("G1 Z1.2 F720 ; Move print head up"));
        assert!(g.contains("; filament_type = PETG\n"));
    }

    #[test]
    fn template_errors_block_the_export() {
        let mut p = printer();
        p.gcode.end = "M104 S{temperatur}".into();
        let src = "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet p = new_path();\np.print_to(10, 0, 0.2, 0.45, 0.2);\np.print_to(0, 0, 0.2, 0.45, 0.2);\np";
        let m = model::parse(src).unwrap();
        let eng = script::ScriptEngine::new(script::Limits::default());
        let c = eng.compile(&m).unwrap();
        let params = IndexMap::new();
        let ctx = script::PrinterContext {
            nozzle_diameter: 0.4,
            layer_height: 0.2,
            first_layer_height: 0.2,
            nozzle_cone_angle: 0.0,
            nozzle_cone_height: 0.0,
        };
        let path = eng.run(&c, &params, &ctx).path.unwrap();
        let fi = filament();
        let pl = plan::plan(&path, &p, &fi).0.unwrap();
        let e = emit(&Job {
            model: &m,
            params: &params,
            path: &path,
            plan: &pl,
            printer: &p,
            filament: &fi,
            setup: &Setup::default(),
        })
        .unwrap_err();
        assert_eq!(e[0].code, "profile.template_error");
        assert_eq!(e[0].field.as_deref(), Some("gcode.end"));
    }

    /// G-code complet d'un petit parcours sur deux couches.
    fn two_layers(printer_src: &str) -> String {
        let src = "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet p = new_path();\np.travel_to(0, 0, 0.2);\np.print_to(30, 0, 0.2, 0.45, 0.2);\np.travel_to(30, 10, 0.2);\np.print_to(0, 10, 0.2, 0.45, 0.2);\np.print_to(0, 0, 0.4, 0.45, 0.2);\np.print_to(30, 0, 0.4, 0.45, 0.2);\np";
        let mut w = crate::Workbench::default();
        w.set_printer(printer_src).unwrap();
        w.set_filament(crate::testing::PLA).unwrap();
        w.load_model(src).unwrap();
        w.gcode(&w.evaluate(&serde_json::Map::new())).unwrap()
    }

    #[test]
    fn progress_runs_from_zero_to_one_hundred() {
        for src in [
            crate::testing::MK4S,
            crate::testing::MARLIN,
            crate::testing::KLIPPER,
            crate::testing::X1E,
        ] {
            let g = two_layers(src);
            let pct: Vec<u32> = g
                .lines()
                .filter_map(|l| l.strip_prefix("M73 P"))
                .map(|l| l.split(' ').next().unwrap().parse().unwrap())
                .collect();
            assert_eq!(pct.first(), Some(&0));
            assert_eq!(pct.last(), Some(&100));
            assert!(pct.windows(2).all(|w| w[0] <= w[1]), "{pct:?}");
            assert!(g.contains("\nM73 P100 R0\n"));
        }
    }

    #[test]
    fn each_dialect_announces_layers_and_accelerations() {
        let g = two_layers(crate::testing::KLIPPER);
        assert!(g.contains("SET_PRINT_STATS_INFO TOTAL_LAYER=2\n"));
        assert!(g.contains("SET_PRINT_STATS_INFO CURRENT_LAYER=1\n"));
        assert!(g.contains("SET_PRINT_STATS_INFO CURRENT_LAYER=2\n"));
        assert!(g.contains("SET_PRESSURE_ADVANCE ADVANCE=0.036 ; avance de pression"));
        assert!(!g.contains(";LAYER_CHANGE"));
        // Une seule accélération : réécrite entre déplacements et impression.
        let acc: Vec<&str> = g.lines().filter(|l| l.starts_with("M204")).collect();
        assert_eq!(acc[..3], ["M204 S3000", "M204 S500", "M204 S3000"]);
        assert!(acc.contains(&"M204 S2000"));

        let g = two_layers(crate::testing::MARLIN);
        assert!(g.contains("M900 K0.036 ; avance linéaire"));
        assert_eq!(g.matches(";LAYER_CHANGE").count(), 2);
        // Première couche et impression à 500 mm/s² : une seule consigne.
        let acc: Vec<&str> = g.lines().filter(|l| l.starts_with("M204")).collect();
        assert_eq!(acc, ["M204 P500 T1000"]);

        let g = two_layers(crate::testing::X1E);
        assert!(g.contains("; total layer number: 2\n"));
        assert!(g.contains("M73 L2\n"));
        assert!(g.contains("M204 S9000\n"));
    }

    #[test]
    fn thumbnails_are_embedded_as_the_profile_asks() {
        let g = two_layers(crate::testing::MK4S);
        let begins: Vec<&str> = g
            .lines()
            .filter(|l| l.contains("thumbnail") && l.contains(" begin "))
            .collect();
        assert_eq!(begins.len(), 4);
        assert!(begins[0].starts_with("; thumbnail_QOI begin 16x16 "));
        assert!(begins[3].starts_with("; thumbnail begin 380x285 "));
        // Longueur annoncée = base64 qui suit, en lignes d'au plus 78 caractères.
        let start = g.find("; thumbnail begin 380x285 ").unwrap();
        let block = &g[start..g[start..].find("; thumbnail end").unwrap() + start];
        let mut lines = block.lines();
        let len: usize = lines
            .next()
            .unwrap()
            .rsplit(' ')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let b64: Vec<&str> = lines.map(|l| l.strip_prefix("; ").unwrap()).collect();
        assert!(b64.iter().all(|l| l.len() <= 78));
        assert_eq!(b64.concat().len(), len);
        // Vignettes avant le gabarit de démarrage, après la première ligne.
        assert!(g.starts_with("; generated by FabWeave"));
        assert!(start < g.find("M862.3").unwrap());

        let g = two_layers(crate::testing::X1E);
        let header = g.find("; HEADER_BLOCK_END").unwrap();
        let thumbs = g.find("; THUMBNAIL_BLOCK_START").unwrap();
        assert!(header < thumbs && thumbs < g.find("; thumbnail begin 300x300 ").unwrap());
        assert!(g.contains("; THUMBNAIL_BLOCK_END\n"));

        // Sans vignette dans le profil : aucune.
        assert!(!two_layers(crate::testing::MARLIN).contains("thumbnail"));
    }

    #[test]
    fn copies_restart_the_layer_count_and_the_setup_is_recorded() {
        let src = "[meta]\nname = \"t\"\ncategory = \"t\"\n---\nlet p = new_path();\np.travel_to(-5, -5, 0.2);\nfor k in 0..5 { let z = 0.2 * (k + 1); p.print_to(5, -5, z, 0.45, 0.2); p.print_to(5, 5, z, 0.45, 0.2); p.print_to(-5, 5, z, 0.45, 0.2); p.print_to(-5, -5, z, 0.45, 0.2); }\np";
        let mut w = crate::Workbench::default();
        w.set_printer(crate::testing::X1E).unwrap();
        w.set_filament(crate::testing::PLA).unwrap();
        w.load_model(src).unwrap();
        let one = w.gcode(&w.evaluate(&serde_json::Map::new())).unwrap();
        assert!(one.contains("; total layer number: 5\n"));
        let mut setup = Setup {
            adhesion: Adhesion::Brim {
                width: 3.0,
                gap: 0.1,
            },
            ..Default::default()
        };
        setup.nozzle.diameter = Some(0.6);
        w.set_setup(setup.clone()).unwrap();
        let single = w.gcode(&w.evaluate(&serde_json::Map::new())).unwrap();
        let per_copy = single.matches("\nM73 L").count();
        setup.placement.copies = [2, 1];
        w.set_setup(setup).unwrap();
        let g = w.gcode(&w.evaluate(&serde_json::Map::new())).unwrap();
        // La deuxième copie repart du plateau : ses couches s'ajoutent à celles
        // de la première (bordure comprise dans la première couche).
        let layers = g.matches("\nM73 L").count();
        assert_eq!(layers, 2 * per_copy, "{per_copy} couches par copie");
        assert!(g.contains(&format!("; total layer number: {layers}\n")));
        assert!(g.contains("; nozzle = 0.6 abrasive\n"));
        assert!(g.contains("; copies = 2 x 1 (pièce par pièce)\n"));
        assert!(g.contains("; adhesion = brim, width 3 mm, gap 0.1 mm\n"));
        assert!(g.contains("M862") || g.contains("; HEADER_BLOCK_START"));
    }

    #[test]
    fn output_is_deterministic() {
        assert_eq!(square_gcode(), square_gcode());
    }

    #[test]
    fn sample_scope_accepts_the_shipped_profiles() {
        // Les profils livrés passent le contrôle des gabarits (fait à la lecture).
        let _ = printer();
        let _ = crate::testing::core_one();
        let _ = crate::testing::petg();
    }
}
