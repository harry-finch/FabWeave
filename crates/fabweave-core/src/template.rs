//! Gabarits de G-code des profils (démarrage, fin) : texte libre où chaque
//! `{ expression }` est une expression Rhai évaluée avec les variables de
//! l'impression (`first_layer_temperature`, `first_layer_min_x`, `max_z`…).
//!
//! Exemples : `M104 S{first_layer_temperature}`,
//! `G1 Z{min(max_z + 1, max_print_height)}`,
//! `{if filament_type == "FLEX" { "G1 E-4" } else { "G1 E-2" }}`.
//! `{{` et `}}` écrivent une accolade. Une expression qui ne renvoie rien
//! (`if` sans `else` faux) n'écrit rien.

use rhai::{Dynamic, Engine, Scope};

use crate::format::num;

/// Morceau d'un gabarit.
#[derive(Debug, PartialEq)]
enum Piece<'a> {
    Text(String),
    Expr(&'a str, usize),
}

/// Découpe un gabarit ; l'erreur donne la ligne (à partir de 1) de
/// l'accolade non fermée.
fn pieces(t: &str) -> Result<Vec<Piece<'_>>, usize> {
    let mut out = Vec::new();
    let mut text = String::new();
    let bytes = t.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'{' if bytes.get(i + 1) == Some(&b'{') => {
                text.push('{');
                i += 2;
            }
            b'}' if bytes.get(i + 1) == Some(&b'}') => {
                text.push('}');
                i += 2;
            }
            b'{' => {
                let line = t[..i].matches('\n').count() + 1;
                // Accolade fermante correspondante, hors chaînes de caractères.
                let (mut depth, mut j, mut in_str) = (1, i + 1, false);
                while j < bytes.len() && depth > 0 {
                    match bytes[j] {
                        b'"' => in_str = !in_str,
                        b'\\' if in_str => j += 1,
                        b'{' if !in_str => depth += 1,
                        b'}' if !in_str => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                if depth > 0 {
                    return Err(line);
                }
                if !text.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut text)));
                }
                out.push(Piece::Expr(&t[i + 1..j - 1], line));
                i = j;
            }
            _ => {
                let c = t[i..].chars().next().unwrap_or(' ');
                text.push(c);
                i += c.len_utf8();
            }
        }
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    Ok(out)
}

/// Erreur dans un gabarit : ligne dans le gabarit et message de Rhai.
#[derive(Debug, PartialEq)]
pub struct TemplateError {
    pub line: usize,
    pub message: String,
}

pub struct TemplateEngine {
    engine: Engine,
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine
            .set_max_operations(100_000)
            .set_max_string_size(10_000)
            .set_strict_variables(true);
        engine.disable_symbol("eval");
        // `min` / `max` mêlant entiers et flottants (`max(0, first_layer_min_y)`).
        for (name, f) in [("min", f64::min as fn(f64, f64) -> f64), ("max", f64::max)] {
            engine.register_fn(name, move |a: f64, b: f64| f(a, b));
            engine.register_fn(name, move |a: i64, b: f64| f(a as f64, b));
            engine.register_fn(name, move |a: f64, b: i64| f(a, b as f64));
        }
        Self { engine }
    }

    /// Vérifie un gabarit (syntaxe, variables connues) sans l'évaluer.
    pub fn check(&self, template: &str, vars: &Scope) -> Result<(), TemplateError> {
        for p in pieces(template).map_err(|line| TemplateError {
            line,
            message: "{".into(),
        })? {
            if let Piece::Expr(e, line) = p {
                self.engine
                    .compile_with_scope(vars, e)
                    .map_err(|err| TemplateError {
                        line,
                        message: err.to_string(),
                    })?;
            }
        }
        Ok(())
    }

    pub fn render(&self, template: &str, vars: &Scope) -> Result<String, TemplateError> {
        let mut out = String::with_capacity(template.len());
        let mut scope = vars.clone();
        for p in pieces(template).map_err(|line| TemplateError {
            line,
            message: "{".into(),
        })? {
            match p {
                Piece::Text(t) => out.push_str(&t),
                Piece::Expr(e, line) => {
                    let v: Dynamic = self.engine.eval_with_scope(&mut scope, e).map_err(|err| {
                        TemplateError {
                            line,
                            message: err.to_string(),
                        }
                    })?;
                    out.push_str(&show(&v));
                }
            }
        }
        Ok(out)
    }
}

fn show(v: &Dynamic) -> String {
    if v.is_unit() {
        String::new()
    } else if let Ok(f) = v.as_float() {
        num(f, 3)
    } else if let Ok(i) = v.as_int() {
        i.to_string()
    } else if let Ok(b) = v.as_bool() {
        u8::from(b).to_string()
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Scope<'static> {
        let mut s = Scope::new();
        s.push_constant("t", 215.0_f64);
        s.push_constant("max_z", 40.25_f64);
        s.push_constant("top", 220.0_f64);
        s.push_constant("kind", "PLA".to_string());
        s.push_constant("hf", false);
        s
    }

    #[test]
    fn renders_expressions_and_braces() {
        let e = TemplateEngine::new();
        let out = e
            .render(
                "M104 S{t}\nG1 Z{min(max_z + 1, top)} ; {{fin}}\n{if kind == \"FLEX\" { \"G1 E-4\" } else { \"G1 E-2\" }}\nF{if hf {1} else {0}}{if hf { \"X\" }}",
                &vars(),
            )
            .unwrap();
        assert_eq!(out, "M104 S215\nG1 Z41.25 ; {fin}\nG1 E-2\nF0");
    }

    #[test]
    fn unknown_variables_and_unclosed_braces_are_reported() {
        let e = TemplateEngine::new();
        let err = e.check("G28\nM104 S{temp}", &vars()).unwrap_err();
        assert_eq!(err.line, 2);
        assert!(err.message.contains("temp"));
        assert_eq!(e.check("ok\n{t", &vars()).unwrap_err().line, 2);
    }
}
