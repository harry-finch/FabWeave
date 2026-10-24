//! Poste de travail : un modèle compilé, un profil imprimante, un profil
//! filament, et l'enchaînement valeurs → script → plan → G-code. Utilisé tel
//! quel par le pont WebAssembly et par les exemples en ligne de commande.

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::gcode::{emit, Job};
use crate::issue::{has_error, Issue};
use crate::model::{self, Model};
use crate::params::{resolve, ParamValue};
use crate::path::Toolpath;
use crate::plan::{plan, Plan};
use crate::profile::{parse_filament, parse_printer, FilamentProfile, PrinterProfile};
use crate::script::{Compiled, Limits, PrinterContext, RunOutput, ScriptEngine};

pub struct Workbench {
    scripts: ScriptEngine,
    model: Option<(Model, Compiled)>,
    printer: Option<PrinterProfile>,
    filament: Option<FilamentProfile>,
}

/// Résultat d'un calcul.
pub struct Evaluation {
    pub params: IndexMap<String, ParamValue>,
    pub path: Option<Toolpath>,
    pub plan: Option<Plan>,
    pub issues: Vec<Issue>,
    pub log: Vec<String>,
}

impl Default for Workbench {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

impl Workbench {
    pub fn new(limits: Limits) -> Self {
        Self {
            scripts: ScriptEngine::new(limits),
            model: None,
            printer: None,
            filament: None,
        }
    }

    /// Lit et compile un modèle ; en cas d'échec, plus aucun modèle n'est chargé.
    pub fn load_model(&mut self, source: &str) -> Result<&Model, Vec<Issue>> {
        self.model = None;
        let m = model::parse(source)?;
        let c = self.scripts.compile(&m).map_err(|i| vec![i])?;
        self.model = Some((m, c));
        Ok(&self.model.as_ref().expect("modèle chargé").0)
    }

    pub fn set_printer(&mut self, source: &str) -> Result<&PrinterProfile, Vec<Issue>> {
        self.printer = Some(parse_printer(source)?);
        Ok(self.printer.as_ref().expect("profil chargé"))
    }

    pub fn set_filament(&mut self, source: &str) -> Result<&FilamentProfile, Vec<Issue>> {
        self.filament = Some(parse_filament(source)?);
        Ok(self.filament.as_ref().expect("profil chargé"))
    }

    pub fn model(&self) -> Option<&Model> {
        self.model.as_ref().map(|(m, _)| m)
    }

    pub fn printer(&self) -> Option<&PrinterProfile> {
        self.printer.as_ref()
    }

    pub fn filament(&self) -> Option<&FilamentProfile> {
        self.filament.as_ref()
    }

    /// Ce que le script sait de la machine.
    pub fn printer_context(&self) -> Option<PrinterContext> {
        self.printer.as_ref().map(|p| PrinterContext {
            nozzle_diameter: p.nozzle.diameter,
            layer_height: p.layers.layer_height,
            first_layer_height: p.layers.first_layer_height,
        })
    }

    fn missing(&self) -> Vec<Issue> {
        let mut out = Vec::new();
        if self.model.is_none() {
            out.push(Issue::error("model.not_loaded"));
        }
        if self.printer.is_none() {
            out.push(Issue::error("profile.printer_not_loaded"));
        }
        if self.filament.is_none() {
            out.push(Issue::error("profile.filament_not_loaded"));
        }
        out
    }

    /// Exécute seulement le script (mesures de performance).
    pub fn run_script(&self, values: &Map<String, Value>) -> Result<RunOutput, Vec<Issue>> {
        let missing = self.missing();
        if !missing.is_empty() {
            return Err(missing);
        }
        let (m, c) = self.model.as_ref().expect("modèle chargé");
        let (params, issues) = resolve(&m.params, values);
        if has_error(&issues) {
            return Err(issues);
        }
        Ok(self
            .scripts
            .run(c, &params, &self.printer_context().expect("profil chargé")))
    }

    pub fn evaluate(&self, values: &Map<String, Value>) -> Evaluation {
        let mut ev = Evaluation {
            params: IndexMap::new(),
            path: None,
            plan: None,
            issues: self.missing(),
            log: Vec::new(),
        };
        if !ev.issues.is_empty() {
            return ev;
        }
        let (m, c) = self.model.as_ref().expect("modèle chargé");
        let (printer, filament) = (
            self.printer.as_ref().expect("profil"),
            self.filament.as_ref().expect("profil"),
        );
        let (params, issues) = resolve(&m.params, values);
        ev.params = params;
        ev.issues = issues;
        let nozzle = printer.nozzle.diameter;
        if !m.meta.recommended_nozzle.is_empty()
            && !m
                .meta
                .recommended_nozzle
                .iter()
                .any(|d| (d - nozzle).abs() < 1e-6)
        {
            ev.issues.push(
                Issue::warning("nozzle.not_recommended")
                    .field("printer.nozzle")
                    .value(nozzle),
            );
        }
        if has_error(&ev.issues) {
            return ev;
        }
        let out = self
            .scripts
            .run(c, &ev.params, &self.printer_context().expect("profil"));
        ev.log = out.log;
        ev.issues.extend(out.issues);
        if let Some(path) = out.path {
            let (pl, issues) = plan(&path, printer, filament);
            ev.issues.extend(issues);
            ev.plan = pl;
            ev.path = Some(path);
        }
        ev
    }

    /// G-code d'un calcul sans erreur bloquante.
    pub fn gcode(&self, ev: &Evaluation) -> Result<String, Vec<Issue>> {
        let missing = self.missing();
        if !missing.is_empty() {
            return Err(missing);
        }
        match (&ev.path, &ev.plan) {
            (Some(path), Some(plan)) if !has_error(&ev.issues) => emit(&Job {
                model: &self.model.as_ref().expect("modèle").0,
                params: &ev.params,
                path,
                plan,
                printer: self.printer.as_ref().expect("profil"),
                filament: self.filament.as_ref().expect("profil"),
            }),
            _ => Err(ev.issues.iter().filter(|i| i.is_error()).cloned().collect()),
        }
    }
}
