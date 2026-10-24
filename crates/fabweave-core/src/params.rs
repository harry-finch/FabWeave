//! Valeurs des paramètres envoyées par l'interface (ou la ligne de commande),
//! contrôlées contre leur définition. Une valeur hors bornes n'est jamais
//! ramenée dans l'intervalle : elle produit une erreur, et le parcours n'est
//! pas calculé.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;

use crate::issue::Issue;
use crate::model::{ParamDef, ParamKind};

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ParamValue {
    Float(f64),
    Int(i64),
    Bool(bool),
}

impl ParamDef {
    pub fn default_value(&self) -> ParamValue {
        match self.kind {
            ParamKind::Float { default, .. } => ParamValue::Float(default),
            ParamKind::Int { default, .. } => ParamValue::Int(default),
            ParamKind::Bool { default } => ParamValue::Bool(default),
        }
    }
}

/// Valeurs de tous les paramètres : celles fournies, sinon les valeurs par
/// défaut. Les messages disent ce qui est refusé (erreur) ou ignoré
/// (avertissement pour un identifiant inconnu).
pub fn resolve(
    defs: &IndexMap<String, ParamDef>,
    given: &serde_json::Map<String, Value>,
) -> (IndexMap<String, ParamValue>, Vec<Issue>) {
    let mut out = IndexMap::new();
    let mut issues = Vec::new();
    for (id, def) in defs {
        let field = format!("params.{id}");
        let value = match given.get(id) {
            None | Some(Value::Null) => def.default_value(),
            Some(v) => match check(&def.kind, v) {
                Ok(v) => v,
                Err(issue) => {
                    issues.push(issue.field(field));
                    def.default_value()
                }
            },
        };
        out.insert(id.clone(), value);
    }
    for id in given.keys().filter(|k| !defs.contains_key(*k)) {
        issues.push(Issue::warning("param.unknown").field(format!("params.{id}")));
    }
    (out, issues)
}

fn check(kind: &ParamKind, v: &Value) -> Result<ParamValue, Issue> {
    let wrong = || Issue::error("param.wrong_type");
    match *kind {
        ParamKind::Float { min, max, .. } => {
            let x = v.as_f64().filter(|x| x.is_finite()).ok_or_else(wrong)?;
            if x < min || x > max {
                return Err(Issue::error("param.out_of_range").value(x));
            }
            Ok(ParamValue::Float(x))
        }
        ParamKind::Int { min, max, .. } => {
            let x = v
                .as_i64()
                .or_else(|| {
                    v.as_f64()
                        .filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
                        .map(|f| f as i64)
                })
                .ok_or_else(wrong)?;
            if x < min || x > max {
                return Err(Issue::error("param.out_of_range").value(x as f64));
            }
            Ok(ParamValue::Int(x))
        }
        ParamKind::Bool { .. } => v.as_bool().map(ParamValue::Bool).ok_or_else(wrong),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Text;
    use serde_json::json;

    fn defs() -> IndexMap<String, ParamDef> {
        let p = |kind| ParamDef {
            label: Text::Plain("x".into()),
            unit: None,
            help: None,
            advanced: false,
            kind,
        };
        IndexMap::from([
            (
                "r".to_string(),
                p(ParamKind::Float {
                    default: 10.0,
                    min: 5.0,
                    max: 50.0,
                    step: None,
                }),
            ),
            (
                "n".to_string(),
                p(ParamKind::Int {
                    default: 4,
                    min: 1,
                    max: 9,
                    step: None,
                }),
            ),
            ("b".to_string(), p(ParamKind::Bool { default: false })),
        ])
    }

    fn map(v: Value) -> serde_json::Map<String, Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn defaults_fill_missing_values() {
        let (v, issues) = resolve(&defs(), &map(json!({ "n": 7 })));
        assert!(issues.is_empty());
        assert_eq!(v["r"], ParamValue::Float(10.0));
        assert_eq!(v["n"], ParamValue::Int(7));
        assert_eq!(v["b"], ParamValue::Bool(false));
    }

    #[test]
    fn out_of_range_is_refused_not_clamped() {
        let (_, issues) = resolve(
            &defs(),
            &map(json!({ "r": 80.0, "n": 2.5, "b": 1, "zz": 3 })),
        );
        let codes: Vec<_> = issues
            .iter()
            .map(|i| (i.code, i.field.clone().unwrap()))
            .collect();
        assert_eq!(
            codes,
            [
                ("param.out_of_range", "params.r".to_string()),
                ("param.wrong_type", "params.n".to_string()),
                ("param.wrong_type", "params.b".to_string()),
                ("param.unknown", "params.zz".to_string()),
            ]
        );
        assert_eq!(issues[0].value, Some(80.0));
        assert!(issues[3].severity == crate::issue::Severity::Warning);
    }
}
