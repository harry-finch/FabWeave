//! Moteur FabWeave pour l'interface : appelé depuis un Web Worker (ADR-0001).
//! Il reçoit le modèle et les profils en texte, les valeurs des curseurs en
//! JSON, et renvoie le parcours en tableaux plats (transférables sans copie
//! vers le fil principal) et un rapport JSON (mesures, messages, journal).
//!
//! Les messages gardent leur code stable (`param.out_of_range`…), traduit
//! par l'interface.

use fabweave_core::issue::has_error;
use fabweave_core::{Evaluation, Issue, Workbench};
use serde_json::{json, Map, Value};
use wasm_bindgen::prelude::*;

/// Version du moteur.
#[wasm_bindgen]
pub fn engine_version() -> String {
    fabweave_core::version().into()
}

/// Description d'un modèle sans le compiler (catalogue). JSON : `{ ok, model }`
/// ou `{ ok, issues }`.
#[wasm_bindgen]
pub fn read_model(source: &str) -> String {
    match fabweave_core::model::parse(source) {
        Ok(m) => json!({ "ok": true, "model": m }).to_string(),
        Err(e) => issues_json(&e),
    }
}

/// Profil sans le charger (listes des imprimantes et filaments, éditeur) :
/// `kind` vaut `printer` ou `filament`. JSON : `{ ok, profile, issues }` ;
/// un profil lisible mais aux valeurs fautives garde `profile` (pour être
/// corrigé), `ok` est faux s'il y a une erreur.
#[wasm_bindgen]
pub fn read_profile(kind: &str, source: &str) -> String {
    use fabweave_core::profile::{check_filament, check_printer, read_filament, read_printer};
    let r = match kind {
        "printer" => read_printer(source).map(|p| (check_printer(&p), json!(p))),
        _ => read_filament(source).map(|f| (check_filament(&f), json!(f))),
    };
    match r {
        Ok((issues, p)) => {
            json!({ "ok": !has_error(&issues), "profile": p, "issues": issues }).to_string()
        }
        Err(e) => issues_json(&e),
    }
}

/// Profil modifié dans l'éditeur (JSON) : contrôlé et écrit en TOML.
/// JSON : `{ ok, issues, toml }` (`toml` absent si le JSON ne décrit pas un
/// profil).
#[wasm_bindgen]
pub fn check_profile(kind: &str, profile: &str) -> String {
    use fabweave_core::profile::*;
    let invalid =
        |e: serde_json::Error| vec![Issue::error("profile.invalid").detail(e.to_string())];
    let r = match kind {
        "printer" => serde_json::from_str::<PrinterProfile>(profile)
            .map_err(invalid)
            .map(|p| (check_printer(&p), printer_toml(&p))),
        _ => serde_json::from_str::<FilamentProfile>(profile)
            .map_err(invalid)
            .map(|f| (check_filament(&f), filament_toml(&f))),
    };
    match r {
        Ok((issues, toml)) => {
            json!({ "ok": !has_error(&issues), "issues": issues, "toml": toml }).to_string()
        }
        Err(e) => issues_json(&e),
    }
}

/// Profils d'un fichier de trancheur (PrusaSlicer `.ini`, Bambu Studio ou
/// OrcaSlicer `.json`). `bases` : `{ printers: [toml], filaments: [toml] }`,
/// profils de départ possibles (le premier sert à défaut). JSON :
/// `{ ok, profiles: [{ kind, profile, report, toml, issues }] }` ou
/// `{ ok, issues }`.
#[wasm_bindgen]
pub fn import_profiles(source: &str, bases: &str) -> String {
    use fabweave_core::import::{import, ImportedProfile};
    use fabweave_core::profile::*;
    let b: Value = serde_json::from_str(bases).unwrap_or_default();
    let texts = |k: &str| -> Vec<String> {
        b[k].as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let printers: Vec<_> = texts("printers")
        .iter()
        .filter_map(|t| parse_printer(t).ok())
        .collect();
    let filaments: Vec<_> = texts("filaments")
        .iter()
        .filter_map(|t| parse_filament(t).ok())
        .collect();
    match import(source, &printers, &filaments) {
        Ok(found) => {
            let profiles: Vec<Value> = found
                .iter()
                .map(|i| {
                    let (issues, toml) = match &i.profile {
                        ImportedProfile::Printer(p) => (check_printer(p), printer_toml(p)),
                        ImportedProfile::Filament(f) => (check_filament(f), filament_toml(f)),
                    };
                    let mut v = json!(i);
                    v["toml"] = json!(toml);
                    v["issues"] = json!(issues);
                    v
                })
                .collect();
            json!({ "ok": true, "profiles": profiles }).to_string()
        }
        Err(e) => issues_json(&e),
    }
}

/// Parcours calculé, prêt à dessiner.
#[wasm_bindgen]
pub struct Frame {
    positions: Vec<f32>,
    kinds: Vec<u8>,
    speeds: Vec<f32>,
    widths: Vec<f32>,
    heights: Vec<f32>,
    flows: Vec<f32>,
    fans: Vec<u8>,
    outside: Vec<u8>,
    requested: Vec<f32>,
    limits: Vec<u8>,
    times: Vec<f32>,
    report: String,
}

#[wasm_bindgen]
impl Frame {
    /// Points dans le repère de la machine : `x, y, z` à la suite (mm).
    pub fn take_positions(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.positions)
    }

    /// Nature du mouvement qui arrive à chaque point : 0 déplacement,
    /// 1 impression, 2 pont, 3 goutte, 4 essuyage, 5 pause.
    pub fn take_kinds(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.kinds)
    }

    /// Vitesse obtenue du mouvement qui arrive à chaque point (mm/s).
    pub fn take_speeds(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.speeds)
    }

    /// Largeur du boudin qui arrive à chaque point (mm) ; diamètre pour une
    /// goutte, 0 pour un déplacement.
    pub fn take_widths(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.widths)
    }

    /// Hauteur du boudin qui arrive à chaque point (mm) ; 0 pour un
    /// mouvement qui ne dépose rien.
    pub fn take_heights(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.heights)
    }

    /// Débit volumétrique obtenu du mouvement qui arrive à chaque point
    /// (mm³/s), 0 pour un déplacement.
    pub fn take_flows(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.flows)
    }

    /// Ventilateur pendant le mouvement qui arrive à chaque point (%).
    pub fn take_fans(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.fans)
    }

    /// 1 pour un mouvement imprimé hors du volume d'impression ou dans une
    /// zone exclue du plateau (surligné dans la vue), 0 sinon.
    pub fn take_outside(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.outside)
    }

    /// Vitesse demandée par le modèle pour le mouvement qui arrive à chaque
    /// point (mm/s).
    pub fn take_requested(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.requested)
    }

    /// Plafond qui a limité la vitesse du mouvement : 0 aucun, 1 première
    /// couche, 2 machine, 3 débit du filament, 4 axe Z.
    pub fn take_limits(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.limits)
    }

    /// Durée estimée du mouvement qui arrive à chaque point (s), accélérations,
    /// rétraction et saut compris.
    pub fn take_times(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.times)
    }

    /// Mesures, messages et journal du script (JSON).
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

#[wasm_bindgen]
pub struct Engine {
    bench: Workbench,
    /// Poste séparé pour les vignettes du catalogue (le modèle courant reste chargé).
    thumbs: Option<Workbench>,
    /// Dernier calcul et les valeurs qui l'ont produit (réutilisé à l'export).
    last: Option<(String, Evaluation)>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

fn values(json: &str) -> Map<String, Value> {
    serde_json::from_str::<Map<String, Value>>(json).unwrap_or_default()
}

fn issues_json(issues: &[Issue]) -> String {
    json!({ "ok": false, "issues": issues }).to_string()
}

#[wasm_bindgen]
impl Engine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            bench: Workbench::default(),
            thumbs: None,
            last: None,
        }
    }

    /// Lit et compile un modèle `.weave`. JSON : `{ ok, model }` ou `{ ok, issues }`.
    pub fn load_model(&mut self, source: &str) -> String {
        self.last = None;
        match self.bench.load_model(source) {
            Ok(m) => json!({ "ok": true, "model": m }).to_string(),
            Err(e) => issues_json(&e),
        }
    }

    /// Profil imprimante (TOML). JSON : `{ ok, profile }` ou `{ ok, issues }`.
    pub fn set_printer(&mut self, source: &str) -> String {
        self.last = None;
        match self.bench.set_printer(source) {
            Ok(p) => json!({ "ok": true, "profile": p }).to_string(),
            Err(e) => issues_json(&e),
        }
    }

    /// Profil filament (TOML). JSON : `{ ok, profile }` ou `{ ok, issues }`.
    pub fn set_filament(&mut self, source: &str) -> String {
        self.last = None;
        match self.bench.set_filament(source) {
            Ok(f) => json!({ "ok": true, "profile": f }).to_string(),
            Err(e) => issues_json(&e),
        }
    }

    /// Préparation de l'impression (buse, placement, adhésion ; JSON de
    /// `fabweave_core::setup::Setup`). JSON : `{ ok }` ou `{ ok, issues }`
    /// (la préparation précédente est alors gardée).
    pub fn set_setup(&mut self, setup_json: &str) -> String {
        self.last = None;
        let setup = match serde_json::from_str::<fabweave_core::setup::Setup>(setup_json) {
            Ok(s) => s,
            Err(e) => {
                return issues_json(&[Issue::error("setup.invalid").detail(e.to_string())]);
            }
        };
        match self.bench.set_setup(setup) {
            Ok(()) => json!({ "ok": true }).to_string(),
            Err(e) => issues_json(&e),
        }
    }

    /// Calcule le parcours pour des valeurs de curseurs (`{ "id": valeur }`).
    pub fn evaluate(&mut self, values_json: &str) -> Frame {
        let ev = self.bench.evaluate(&values(values_json));
        let n = ev.path.as_ref().map_or(0, |p| p.points.len());
        let (mut flows, mut fans) = (Vec::with_capacity(n), Vec::with_capacity(n));
        let (mut requested, mut limits, mut times) = (
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
        );
        let (mut positions, mut kinds, mut speeds, mut widths, mut heights, mut outside) = (
            Vec::with_capacity(n * 3),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
            Vec::with_capacity(n),
        );
        if let (Some(path), Some(plan)) = (&ev.path, &ev.plan) {
            let o = plan.offset;
            for (p, m) in path.points.iter().zip(&plan.moves) {
                positions.extend([
                    (p.x + o[0]) as f32,
                    (p.y + o[1]) as f32,
                    (p.z + o[2]) as f32,
                ]);
                kinds.push(p.kind as u8);
                speeds.push(m.speed as f32);
                widths.push(if p.kind.deposits() { p.w as f32 } else { 0.0 });
                heights.push(if p.kind.deposits() { p.h as f32 } else { 0.0 });
                flows.push(m.flow as f32);
                fans.push(m.fan);
                outside.push(u8::from(m.outside));
                requested.push(m.requested as f32);
                limits.push(m.limit as u8);
                times.push(m.time as f32);
            }
        }
        // Annotations dans le repère de la machine.
        let notes: Vec<Value> = match (&ev.path, &ev.plan) {
            (Some(path), Some(plan)) => path
                .notes
                .iter()
                .map(|n| json!({ "x": n.x + plan.offset[0], "y": n.y + plan.offset[1], "z": n.z + plan.offset[2], "text": n.text }))
                .collect(),
            _ => Vec::new(),
        };
        let report = json!({
            "notes": notes,
            "params": ev.params,
            "stats": ev.plan.as_ref().map(|p| &p.stats),
            "offset": ev.plan.as_ref().map(|p| p.offset),
            "first_layer_top": ev.plan.as_ref().map(|p| p.first_layer_top),
            // Buse montée et ce qu'elle change (profils effectifs).
            "nozzle": match (&ev.printer, &ev.filament) {
                (Some(p), Some(f)) => json!({
                    "diameter": p.nozzle.diameter,
                    "high_flow": p.nozzle.high_flow,
                    "abrasive": p.nozzle.abrasive,
                    "layer_height": p.layers.layer_height,
                    "first_layer_height": p.layers.first_layer_height,
                    "max_volumetric": f.speed.max_volumetric,
                }),
                _ => Value::Null,
            },
            "points": n,
            "issues": ev.issues,
            "log": ev.log,
        })
        .to_string();
        self.last = Some((values_json.to_string(), ev));
        Frame {
            positions,
            kinds,
            speeds,
            widths,
            heights,
            flows,
            fans,
            outside,
            requested,
            limits,
            times,
            report,
        }
    }

    /// G-code pour ces valeurs. En cas d'erreur : messages bloquants en JSON.
    pub fn gcode(&mut self, values_json: &str) -> Result<String, JsError> {
        let reuse = matches!(&self.last, Some((v, _)) if v == values_json);
        if !reuse {
            let ev = self.bench.evaluate(&values(values_json));
            self.last = Some((values_json.to_string(), ev));
        }
        let (_, ev) = self.last.as_ref().expect("calcul présent");
        self.bench
            .gcode(ev)
            .map_err(|e| JsError::new(&serde_json::to_string(&e).unwrap_or_default()))
    }

    /// Paquet `.gcode.3mf` (Bambu Lab) des valeurs ; en cas de refus,
    /// l'erreur porte le JSON des messages bloquants.
    pub fn gcode_3mf(&mut self, values_json: &str) -> Result<Vec<u8>, JsError> {
        let reuse = matches!(&self.last, Some((v, _)) if v == values_json);
        if !reuse {
            let ev = self.bench.evaluate(&values(values_json));
            self.last = Some((values_json.to_string(), ev));
        }
        let (_, ev) = self.last.as_ref().expect("calcul présent");
        self.bench
            .gcode_3mf(ev)
            .map_err(|e| JsError::new(&serde_json::to_string(&e).unwrap_or_default()))
    }

    /// Parcours d'un modèle aux valeurs par défaut, pour sa vignette :
    /// `x, y, z, nature` à la suite (4 nombres par point), au plus `max_points`
    /// points (sous-échantillonné ; déplacements et gouttes toujours gardés). Vide si le modèle ne se calcule pas.
    pub fn thumbnail(
        &mut self,
        model: &str,
        printer: &str,
        filament: &str,
        max_points: u32,
    ) -> Vec<f32> {
        let w = self.thumbs.get_or_insert_with(Workbench::default);
        if w.set_printer(printer).is_err()
            || w.set_filament(filament).is_err()
            || w.load_model(model).is_err()
        {
            return Vec::new();
        }
        let ev = w.evaluate(&Map::new());
        let Some(path) = ev.path.filter(|_| !ev.issues.iter().any(Issue::is_error)) else {
            return Vec::new();
        };
        let n = path.points.len();
        let stride = n.div_ceil(max_points.max(2) as usize).max(1);
        let mut out = Vec::with_capacity(n / stride * 4 + 4);
        for (i, p) in path.points.iter().enumerate() {
            if i % stride == 0
                || i + 1 == n
                || matches!(
                    p.kind,
                    fabweave_core::MoveKind::Travel
                        | fabweave_core::MoveKind::Blob
                        | fabweave_core::MoveKind::Wipe
                )
            {
                out.extend([p.x as f32, p.y as f32, p.z as f32, p.kind as u8 as f32]);
            }
        }
        out
    }

    /// Exécute seulement le script (mesures de performance) : nombre de
    /// points, ou -1 en cas d'erreur.
    pub fn run_script(&mut self, values_json: &str) -> i32 {
        match self.bench.run_script(&values(values_json)) {
            Ok(out) => out.path.map_or(-1, |p| p.points.len() as i32),
            Err(_) => -1,
        }
    }
}
