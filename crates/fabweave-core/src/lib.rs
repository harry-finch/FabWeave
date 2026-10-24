//! Moteur FabWeave : modèles paramétriques `.weave` (en-tête TOML + script
//! Rhai), parcours de la buse, physique de l'extrusion et G-code.
//!
//! Pur et déterministe : aucune IO, aucun réseau, aucune dépendance
//! d'interface. Millimètres, degrés (paramètres), secondes.

// Les messages (`Issue`) sont rares et portés par valeur : leur taille dans
// un `Result` n'a pas d'effet mesurable.
#![allow(clippy::result_large_err)]

pub mod format;
pub mod gcode;
pub mod issue;
pub mod model;
pub mod params;
pub mod path;
pub mod plan;
pub mod profile;
pub mod script;
pub mod template;
pub mod workbench;

#[cfg(test)]
mod testing;

pub use issue::{Issue, Severity};
pub use model::Model;
pub use params::ParamValue;
pub use path::{MoveKind, Toolpath, WayPoint};
pub use workbench::{Evaluation, Workbench};

/// Version du moteur.
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
