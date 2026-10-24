//! Moteur FabWeave pour l'interface : appelé depuis un Web Worker (ADR-0001).
//! Il reçoit le modèle et les profils en texte, les valeurs des curseurs en
//! JSON, et renvoie le parcours en tableaux plats (transférables sans copie
//! vers le fil principal) et un rapport JSON (mesures, messages, journal).
//!
//! Les messages gardent leur code stable (`param.out_of_range`…), traduit
//! par l'interface.

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

/// Profil sans le charger (liste des imprimantes et filaments) : `kind` vaut
/// `printer` ou `filament`. JSON : `{ ok, profile }` ou `{ ok, issues }`.
#[wasm_bindgen]
pub fn read_profile(kind: &str, source: &str) -> String {
    let r = match kind {
        "printer" => fabweave_core::profile::parse_printer(source).map(|p| json!(p)),
        _ => fabweave_core::profile::parse_filament(source).map(|f| json!(f)),
    };
    match r {
        Ok(p) => json!({ "ok": true, "profile": p }).to_string(),
        Err(e) => issues_json(&e),
    }
}

/// Parcours calculé, prêt à dessiner.
#[wasm_bindgen]
pub struct Frame {
    positions: Vec<f32>,
    kinds: Vec<u8>,
    speeds: Vec<f32>,
    report: String,
}

#[wasm_bindgen]
impl Frame {
    /// Points dans le repère de la machine : `x, y, z` à la suite (mm).
    pub fn take_positions(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.positions)
    }

    /// Nature du mouvement qui arrive à chaque point : 0 déplacement,
    /// 1 impression, 2 pont.
    pub fn take_kinds(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.kinds)
    }

    /// Vitesse obtenue du mouvement qui arrive à chaque point (mm/s).
    pub fn take_speeds(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.speeds)
    }

    /// Mesures, messages et journal du script (JSON).
    pub fn report(&self) -> String {
        self.report.clone()
    }
}

#[wasm_bindgen]
pub struct Engine {
    bench: Workbench,
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

    /// Calcule le parcours pour des valeurs de curseurs (`{ "id": valeur }`).
    pub fn evaluate(&mut self, values_json: &str) -> Frame {
        let ev = self.bench.evaluate(&values(values_json));
        let n = ev.path.as_ref().map_or(0, |p| p.points.len());
        let (mut positions, mut kinds, mut speeds) = (
            Vec::with_capacity(n * 3),
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

    /// Exécute seulement le script (mesures de performance) : nombre de
    /// points, ou -1 en cas d'erreur.
    pub fn run_script(&mut self, values_json: &str) -> i32 {
        match self.bench.run_script(&values(values_json)) {
            Ok(out) => out.path.map_or(-1, |p| p.points.len() as i32),
            Err(_) => -1,
        }
    }
}
