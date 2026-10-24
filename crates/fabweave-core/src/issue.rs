//! Messages du moteur : un code stable (traduit par l'interface), une
//! gravité, le champ concerné, une valeur ou un nombre éventuels, et pour les
//! fichiers texte (`.weave`, profils) la ligne et la colonne.

use serde::Serialize;

use crate::model::Text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Le parcours ou le G-code ne peut pas être produit tant que la cause
    /// n'est pas corrigée.
    Error,
    /// Le résultat est produit mais un point demande attention.
    Warning,
    /// Adaptation attendue, comptée (vitesse plafonnée, points fusionnés…).
    Info,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Issue {
    pub code: &'static str,
    pub severity: Severity,
    /// Champ concerné (`params.radius`, `printer.bed`, `meta.name`…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    /// Ligne (à partir de 1) dans le fichier source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// Colonne (à partir de 1) dans le fichier source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// Détail technique non traduit (message de TOML ou de Rhai, nom d'une
    /// variable inconnue…), montré en second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Texte écrit par l'auteur du modèle (`warn`, `fail`), par langue.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<Text>,
}

impl Issue {
    pub fn new(code: &'static str, severity: Severity) -> Self {
        Self {
            code,
            severity,
            field: None,
            value: None,
            count: None,
            line: None,
            column: None,
            detail: None,
            text: None,
        }
    }

    pub fn error(code: &'static str) -> Self {
        Self::new(code, Severity::Error)
    }

    pub fn warning(code: &'static str) -> Self {
        Self::new(code, Severity::Warning)
    }

    pub fn info(code: &'static str) -> Self {
        Self::new(code, Severity::Info)
    }

    pub fn field(mut self, f: impl Into<String>) -> Self {
        self.field = Some(f.into());
        self
    }

    pub fn value(mut self, v: f64) -> Self {
        self.value = Some(v);
        self
    }

    pub fn count(mut self, n: usize) -> Self {
        self.count = Some(n);
        self
    }

    pub fn at(mut self, pos: Option<(usize, usize)>) -> Self {
        if let Some((line, column)) = pos {
            self.line = Some(line);
            self.column = Some(column);
        }
        self
    }

    pub fn detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }

    pub fn text(mut self, t: Text) -> Self {
        self.text = Some(t);
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}

/// Vrai si l'un des messages est bloquant.
pub fn has_error(issues: &[Issue]) -> bool {
    issues.iter().any(Issue::is_error)
}

/// Ligne et colonne (à partir de 1) d'une position en octets dans `text`.
pub fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map_or(0, |l| l.chars().count()) + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column_are_one_based() {
        let t = "ab\ncdé\nf";
        assert_eq!(line_col(t, 0), (1, 1));
        assert_eq!(line_col(t, 4), (2, 2));
        assert_eq!(line_col(t, t.len()), (3, 2));
    }
}
