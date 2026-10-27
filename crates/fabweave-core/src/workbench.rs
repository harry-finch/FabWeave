//! Poste de travail : un modèle compilé, un profil imprimante, un profil
//! filament, la préparation choisie par l'utilisateur (buse, placement,
//! adhésion), et l'enchaînement valeurs → script → préparation → plan →
//! G-code. Utilisé tel quel par le pont WebAssembly et par l'outil en ligne
//! de commande.

use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::gcode::{emit, Job};
use crate::issue::{has_error, Issue};
use crate::model::{self, Model};
use crate::params::{resolve, ParamValue};
use crate::path::Toolpath;
use crate::plan::{plan_at, Plan};
use crate::profile::{parse_filament, parse_printer, FilamentProfile, PrinterProfile};
use crate::script::{Compiled, Limits, PrinterContext, RunOutput, ScriptEngine};
use crate::setup::{arrange, Setup};

pub struct Workbench {
    scripts: ScriptEngine,
    model: Option<(Model, Compiled)>,
    printer: Option<PrinterProfile>,
    filament: Option<FilamentProfile>,
    setup: Setup,
}

/// Résultat d'un calcul.
pub struct Evaluation {
    pub params: IndexMap<String, ParamValue>,
    /// Parcours préparé (rotation, copies, adhésion), repère du modèle.
    pub path: Option<Toolpath>,
    pub plan: Option<Plan>,
    pub issues: Vec<Issue>,
    pub log: Vec<String>,
    /// Profils tels que la buse choisie les rend (ceux du G-code).
    pub printer: Option<PrinterProfile>,
    pub filament: Option<FilamentProfile>,
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
            setup: Setup::default(),
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

    /// Préparation de l'impression ; refusée (et l'ancienne gardée) si une
    /// valeur sort de son domaine.
    pub fn set_setup(&mut self, setup: Setup) -> Result<(), Vec<Issue>> {
        let issues = setup.check();
        if has_error(&issues) {
            return Err(issues);
        }
        self.setup = setup;
        Ok(())
    }

    pub fn setup(&self) -> &Setup {
        &self.setup
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

    /// Profils tels que la buse choisie les rend, et les messages du choix.
    pub fn effective(&self) -> Option<(PrinterProfile, FilamentProfile, Vec<Issue>)> {
        match (&self.printer, &self.filament) {
            (Some(p), Some(f)) => Some(self.setup.effective(p, f)),
            _ => None,
        }
    }

    /// Ce que le script sait de la machine (buse choisie comprise).
    pub fn printer_context(&self) -> Option<PrinterContext> {
        let eff = self.effective();
        let p = eff.as_ref().map(|e| &e.0).or(self.printer.as_ref());
        p.map(|p| PrinterContext {
            nozzle_diameter: p.nozzle.diameter,
            layer_height: p.layers.layer_height,
            first_layer_height: p.layers.first_layer_height,
            nozzle_cone_angle: p.nozzle.cone_angle.unwrap_or(0.0),
            nozzle_cone_height: p.nozzle.cone_height.unwrap_or(0.0),
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
            printer: None,
            filament: None,
        };
        if !ev.issues.is_empty() {
            return ev;
        }
        let (m, c) = self.model.as_ref().expect("modèle chargé");
        let (printer, filament, nozzle_issues) = self.effective().expect("profils");
        let (params, issues) = resolve(&m.params, values);
        ev.params = params;
        ev.issues = issues;
        ev.issues.extend(nozzle_issues);
        let nozzle = printer.nozzle.diameter; // buse choisie
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
            let (path, issues) = arrange(&path, &self.setup, &printer);
            ev.issues.extend(issues);
            let pl = &self.setup.placement;
            let target = match (pl.x, pl.y) {
                (None, None) => None,
                (x, y) => {
                    let c = crate::plan::bed_centre(&printer);
                    Some([x.unwrap_or(c[0]), y.unwrap_or(c[1])])
                }
            };
            let (pl, issues) = plan_at(&path, &printer, &filament, target);
            ev.issues.extend(issues);
            ev.plan = pl;
            ev.path = Some(path);
        }
        ev.printer = Some(printer);
        ev.filament = Some(filament);
        ev
    }

    /// G-code d'un calcul sans erreur bloquante.
    pub fn gcode(&self, ev: &Evaluation) -> Result<String, Vec<Issue>> {
        let missing = self.missing();
        if !missing.is_empty() {
            return Err(missing);
        }
        match (&ev.path, &ev.plan, &ev.printer, &ev.filament) {
            (Some(path), Some(plan), Some(printer), Some(filament)) if !has_error(&ev.issues) => {
                emit(&Job {
                    model: &self.model.as_ref().expect("modèle").0,
                    params: &ev.params,
                    path,
                    plan,
                    printer,
                    filament,
                    setup: &self.setup,
                })
            }
            _ => Err(ev.issues.iter().filter(|i| i.is_error()).cloned().collect()),
        }
    }

    /// Paquet `.gcode.3mf` (Bambu Lab) d'un calcul sans erreur bloquante :
    /// le G-code de [`Workbench::gcode`], sa somme MD5, les métadonnées et
    /// les vignettes de la plaque.
    pub fn gcode_3mf(&self, ev: &Evaluation) -> Result<Vec<u8>, Vec<Issue>> {
        let g = self.gcode(ev)?;
        let (path, plan) = (
            ev.path.as_ref().expect("parcours"),
            ev.plan.as_ref().expect("plan"),
        );
        Ok(crate::package::bambu_3mf(
            &g,
            path,
            plan,
            ev.printer.as_ref().expect("profil"),
            ev.filament.as_ref().expect("profil"),
        ))
    }
}
