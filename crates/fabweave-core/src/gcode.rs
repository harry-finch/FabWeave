//! Écriture du G-code à partir du plan : gabarit de démarrage, parcours en
//! extrusion relative (`M83`), gabarit de fin, métadonnées lues par les
//! imprimantes Prusa (temps estimé, filament).
//!
//! Machine à états : un axe, une vitesse `F` ou une consigne de ventilateur
//! n'est écrit que s'il change.

use std::fmt::Write;

use indexmap::IndexMap;
use rhai::Scope;

use crate::format::{duration, num};
use crate::issue::Issue;
use crate::model::Model;
use crate::params::ParamValue;
use crate::path::{MoveKind, Toolpath};
use crate::plan::Plan;
use crate::profile::{FilamentProfile, PrinterProfile};
use crate::template::TemplateEngine;

/// Tout ce qu'il faut pour écrire un G-code.
pub struct Job<'a> {
    pub model: &'a Model,
    pub params: &'a IndexMap<String, ParamValue>,
    pub path: &'a Toolpath,
    pub plan: &'a Plan,
    pub printer: &'a PrinterProfile,
    pub filament: &'a FilamentProfile,
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
) -> Scope<'static> {
    let s = &plan.stats;
    scope(
        printer,
        filament,
        [s.first_layer_bounds.min, s.first_layer_bounds.max],
        [s.bounds.min, s.bounds.max],
        model_name,
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
];

fn scope(
    p: &PrinterProfile,
    f: &FilamentProfile,
    first_layer: [[f64; 3]; 2],
    print: [[f64; 3]; 2],
    model_name: &str,
) -> Scope<'static> {
    let mut s = Scope::new();
    s.push_constant("printer_model", p.printer.model.clone());
    s.push_constant("printer_name", p.printer.name.clone());
    s.push_constant("filament_type", f.filament.material.clone());
    s.push_constant("filament_name", f.filament.name.get("fr").to_string());
    s.push_constant("model_name", model_name.to_string());
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
    match *v {
        ParamValue::Float(f) => num(f, 4),
        ParamValue::Int(i) => i.to_string(),
        ParamValue::Bool(b) => b.to_string(),
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

pub fn emit(job: &Job) -> Result<String, Vec<Issue>> {
    let (p, f, plan) = (job.printer, job.filament, job.plan);
    let r = &f.retraction;
    let lim = &p.limits;
    let engine = TemplateEngine::new();
    let model_name = job.model.meta.name.get("fr").to_string();
    let vars = template_scope(p, f, plan, &model_name);
    let start = render(&engine, "gcode.start", &p.gcode.start, &vars);
    let end = render(&engine, "gcode.end", &p.gcode.end, &vars);
    let (start, end) = match (start, end) {
        (Ok(s), Ok(e)) => (s, e),
        (s, e) => return Err([s.err(), e.err()].into_iter().flatten().collect()),
    };

    let mut g = String::with_capacity(plan.moves.len() * 36 + start.len() + end.len() + 2048);
    line!(g, "; generated by FabWeave {}", crate::version());
    line!(g, "; model = {}", model_name);
    for (k, v) in job.params {
        line!(g, "; param {k} = {}", param_text(v));
    }
    line!(g, "; printer = {}", p.printer.name);
    line!(g, "; filament = {}", f.filament.name.get("fr"));
    g.push('\n');
    g.push_str(&start);
    g.push('\n');
    line!(g, "; --- FabWeave : parcours ---");

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

    line!(g, "; --- FabWeave : fin du parcours ---");
    g.push('\n');
    g.push_str(&end);
    g.push('\n');
    let s = &plan.stats;
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
    /// sur les mouvements imprimés.
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
            // Déplacement initial : rétraction, montée, déplacement, descente.
            "G1 E-0.7 F2100",
            "G1 Z0.4 F720",
            "G1 X105 Y90 F10800",
            "G1 Z0.2 F720",
            // Première couche : filament détendu, ventilateur coupé, 20 mm/s.
            "G1 E0.7 F2100",
            "M107",
            "G1 X115 E* F1200",
            "G1 Y91 E*",
            // Déplacement court (0,5 mm < 1,5 mm) : ni rétraction ni saut.
            "G1 X115.5 F10800",
            "G1 X125 E* F1200",
            // Déplacement long : rétraction, saut au-dessus du plus haut des deux Z.
            "G1 E-0.7 F2100",
            "G1 Z0.6 F720",
            "G1 Y110 F10800",
            "G1 Z0.4 F720",
            // Deuxième couche : température de croisière une seule fois,
            // ventilateur du script (50 %), vitesse nominale 40 mm/s.
            "G1 E0.7 F2100",
            "M104 S210 ; température après la première couche",
            "M106 S128",
            "G1 X135 E* F2400",
            // Retour au ventilateur du filament (100 %).
            "M106 S255",
            "G1 Y120 E*",
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
        })
        .unwrap_err();
        assert_eq!(e[0].code, "profile.template_error");
        assert_eq!(e[0].field.as_deref(), Some("gcode.end"));
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
