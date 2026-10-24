//! Exécution du corps Rhai d'un modèle en bac à sable (ADR-0002).
//!
//! - Variables injectées : `params` (valeurs des curseurs), `printer`
//!   (`nozzle_diameter`, `layer_height`, `first_layer_height`), `PI`, `TAU`.
//! - API du parcours : `new_path()`, puis `print_to`, `print_polar`,
//!   `travel_to`, `set_speed`, `set_speed_factor`, `set_fan`, `reset_fan`.
//! - Fonctions mathématiques réenregistrées depuis `libm` : même résultat en
//!   natif et en WebAssembly. Entiers et flottants s'acceptent partout.
//! - Limites : opérations, profondeur, tailles, nombre de points ; pas de
//!   modules, pas d'horloge, pas d'`eval`. Variables strictes : une variable
//!   inconnue est une erreur dès la compilation.
//!
//! Toute erreur devient un [`Issue`] à code stable, positionné dans le
//! fichier `.weave` (ligne du script + décalage de l'en-tête).

use std::cell::RefCell;
use std::rc::Rc;

use indexmap::IndexMap;
use rhai::{Dynamic, Engine, EvalAltResult, ParseErrorType, Position, Scope, AST, FLOAT, INT};
use serde::Serialize;

use crate::issue::Issue;
use crate::model::Model;
use crate::params::ParamValue;
use crate::path::{MoveKind, Speed, Toolpath, WayPoint};

/// Garde-fous de l'exécution d'un script.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Opérations Rhai au plus (≈ temps de calcul).
    pub max_operations: u64,
    /// Points du parcours au plus.
    pub max_points: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_operations: 50_000_000,
            max_points: 1_000_000,
        }
    }
}

/// Ce que le modèle sait de la machine (variable `printer` du script).
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PrinterContext {
    pub nozzle_diameter: f64,
    pub layer_height: f64,
    pub first_layer_height: f64,
}

/// Lignes de `print()` / `debug()` gardées au plus, par exécution.
const MAX_LOG_LINES: usize = 200;

/// Erreur levée par une fonction de l'API, avec un code stable.
#[derive(Clone, Debug)]
struct Fault {
    code: &'static str,
    detail: String,
}

fn fault(code: &'static str, detail: impl Into<String>) -> Box<EvalAltResult> {
    EvalAltResult::ErrorRuntime(
        Dynamic::from(Fault {
            code,
            detail: detail.into(),
        }),
        Position::NONE,
    )
    .into()
}

/// Nombre (entier ou flottant) passé à une fonction de l'API.
fn num(v: &Dynamic, what: &str) -> Result<f64, Box<EvalAltResult>> {
    if let Ok(f) = v.as_float() {
        return Ok(f);
    }
    if let Ok(i) = v.as_int() {
        return Ok(i as f64);
    }
    Err(fault(
        "script.not_a_number",
        format!("{what} : {}", v.type_name()),
    ))
}

/// Parcours en construction, manipulé par le script.
#[derive(Clone)]
pub struct ScriptPath {
    path: Toolpath,
    speed: Speed,
    fan: Option<u8>,
    limit: usize,
}

impl ScriptPath {
    fn new(limit: usize) -> Self {
        Self {
            path: Toolpath::default(),
            speed: Speed::Factor(1.0),
            fan: None,
            limit,
        }
    }

    fn push(
        &mut self,
        xyz: [f64; 3],
        w: f64,
        h: f64,
        kind: MoveKind,
    ) -> Result<(), Box<EvalAltResult>> {
        if self.path.points.len() >= self.limit {
            return Err(fault("path.too_many_points", self.limit.to_string()));
        }
        if let Some(i) = xyz.iter().position(|v| !v.is_finite()) {
            return Err(fault("path.invalid_position", ["x", "y", "z"][i]));
        }
        if kind != MoveKind::Travel {
            if !(w > 0.0 && w.is_finite()) {
                return Err(fault("path.invalid_width", w.to_string()));
            }
            if !(h > 0.0 && h.is_finite()) {
                return Err(fault("path.invalid_height", h.to_string()));
            }
        }
        self.path.points.push(WayPoint {
            x: xyz[0],
            y: xyz[1],
            z: xyz[2],
            w,
            h,
            kind,
            speed: self.speed,
            fan: self.fan,
            flow: 1.0,
        });
        Ok(())
    }
}

type R<T> = Result<T, Box<EvalAltResult>>;

struct Log {
    lines: Vec<String>,
    dropped: usize,
}

/// Moteur de script, créé une fois et réutilisé pour chaque calcul.
pub struct ScriptEngine {
    engine: Engine,
    log: Rc<RefCell<Log>>,
    limits: Limits,
}

/// Script compilé d'un modèle (gardé tant que le modèle ne change pas).
pub struct Compiled {
    ast: AST,
    script_line: usize,
}

/// Résultat d'une exécution.
pub struct RunOutput {
    pub path: Option<Toolpath>,
    pub issues: Vec<Issue>,
    /// Sorties de `print()` et `debug()` (aide à la mise au point d'un modèle).
    pub log: Vec<String>,
}

type Unary = fn(f64) -> f64;
type Binary = fn(f64, f64) -> f64;

fn register_math(engine: &mut Engine) {
    let unary: [(&str, Unary); 15] = [
        ("sin", libm::sin),
        ("cos", libm::cos),
        ("tan", libm::tan),
        ("asin", libm::asin),
        ("acos", libm::acos),
        ("atan", libm::atan),
        ("sinh", libm::sinh),
        ("cosh", libm::cosh),
        ("tanh", libm::tanh),
        ("exp", libm::exp),
        ("ln", libm::log),
        ("log", libm::log10),
        ("sqrt", libm::sqrt),
        ("rad", |d| d * std::f64::consts::PI / 180.0),
        ("deg", |r| r * 180.0 / std::f64::consts::PI),
    ];
    for (name, f) in unary {
        engine.register_fn(name, move |x: FLOAT| f(x));
        engine.register_fn(name, move |x: INT| f(x as f64));
    }
    let binary: [(&str, Binary); 6] = [
        ("atan", libm::atan2),
        ("atan2", libm::atan2),
        ("pow", libm::pow),
        ("hypot", libm::hypot),
        ("min", f64::min),
        ("max", f64::max),
    ];
    for (name, f) in binary {
        engine.register_fn(name, move |a: FLOAT, b: FLOAT| f(a, b));
        engine.register_fn(name, move |a: INT, b: FLOAT| f(a as f64, b));
        engine.register_fn(name, move |a: FLOAT, b: INT| f(a, b as f64));
        if !matches!(name, "min" | "max") {
            engine.register_fn(name, move |a: INT, b: INT| f(a as f64, b as f64));
        }
    }
    engine.register_fn("lerp", |a: Dynamic, b: Dynamic, t: Dynamic| -> R<f64> {
        let (a, b, t) = (num(&a, "a")?, num(&b, "b")?, num(&t, "t")?);
        Ok(a + (b - a) * t)
    });
    engine.register_fn("clamp", |x: Dynamic, lo: Dynamic, hi: Dynamic| -> R<f64> {
        let (x, lo, hi) = (num(&x, "x")?, num(&lo, "lo")?, num(&hi, "hi")?);
        Ok(x.max(lo).min(hi))
    });
}

fn register_path(engine: &mut Engine, max_points: usize) {
    engine.register_type_with_name::<ScriptPath>("Path");
    engine.register_fn("new_path", move || ScriptPath::new(max_points));
    engine.register_fn("len", |p: &mut ScriptPath| p.path.points.len() as INT);
    engine.register_fn(
        "print_to",
        |p: &mut ScriptPath, x: Dynamic, y: Dynamic, z: Dynamic, w: Dynamic, h: Dynamic| -> R<()> {
            let xyz = [num(&x, "x")?, num(&y, "y")?, num(&z, "z")?];
            p.push(xyz, num(&w, "w")?, num(&h, "h")?, MoveKind::Print)
        },
    );
    engine.register_fn(
        "print_polar",
        |p: &mut ScriptPath, r: Dynamic, a: Dynamic, z: Dynamic, w: Dynamic, h: Dynamic| -> R<()> {
            let (r, a) = (num(&r, "r")?, num(&a, "angle")?);
            let xyz = [r * libm::cos(a), r * libm::sin(a), num(&z, "z")?];
            p.push(xyz, num(&w, "w")?, num(&h, "h")?, MoveKind::Print)
        },
    );
    engine.register_fn(
        "travel_to",
        |p: &mut ScriptPath, x: Dynamic, y: Dynamic, z: Dynamic| -> R<()> {
            let xyz = [num(&x, "x")?, num(&y, "y")?, num(&z, "z")?];
            p.push(xyz, 0.0, 0.0, MoveKind::Travel)
        },
    );
    engine.register_fn("set_speed", |p: &mut ScriptPath, v: Dynamic| -> R<()> {
        let v = num(&v, "speed")?;
        if !(v > 0.0 && v.is_finite()) {
            return Err(fault("path.invalid_speed", v.to_string()));
        }
        p.speed = Speed::Absolute(v);
        Ok(())
    });
    engine.register_fn(
        "set_speed_factor",
        |p: &mut ScriptPath, f: Dynamic| -> R<()> {
            let f = num(&f, "factor")?;
            if !(f > 0.0 && f.is_finite()) {
                return Err(fault("path.invalid_speed", f.to_string()));
            }
            p.speed = Speed::Factor(f);
            Ok(())
        },
    );
    engine.register_fn("set_fan", |p: &mut ScriptPath, percent: Dynamic| -> R<()> {
        let v = num(&percent, "percent")?;
        if !(0.0..=100.0).contains(&v) {
            return Err(fault("path.invalid_fan", v.to_string()));
        }
        p.fan = Some(v.round() as u8);
        Ok(())
    });
    engine.register_fn("reset_fan", |p: &mut ScriptPath| p.fan = None);
}

impl ScriptEngine {
    pub fn new(limits: Limits) -> Self {
        let mut engine = Engine::new();
        engine
            .set_max_operations(limits.max_operations)
            .set_max_call_levels(64)
            .set_max_expr_depths(64, 32)
            .set_max_string_size(10_000)
            .set_max_array_size(limits.max_points)
            .set_max_map_size(10_000)
            .set_strict_variables(true)
            .set_fail_on_invalid_map_property(true);
        engine.disable_symbol("eval");
        register_math(&mut engine);
        register_path(&mut engine, limits.max_points);

        let log = Rc::new(RefCell::new(Log {
            lines: Vec::new(),
            dropped: 0,
        }));
        let sink = |log: Rc<RefCell<Log>>| {
            move |s: &str| {
                let mut l = log.borrow_mut();
                if l.lines.len() < MAX_LOG_LINES {
                    l.lines.push(s.to_string());
                } else {
                    l.dropped += 1;
                }
            }
        };
        engine.on_print(sink(log.clone()));
        let debug = sink(log.clone());
        engine.on_debug(move |s, _, _| debug(s));
        Self {
            engine,
            log,
            limits,
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    /// Variables connues du script (pour la compilation en mode strict).
    /// `params` et `printer` ne sont pas des constantes ici : l'optimiseur ne
    /// doit pas figer leurs valeurs dans l'AST.
    fn declared_scope() -> Scope<'static> {
        let mut s = Scope::new();
        s.push_constant("PI", std::f64::consts::PI);
        s.push_constant("TAU", std::f64::consts::TAU);
        s.push("params", Dynamic::UNIT);
        s.push("printer", Dynamic::UNIT);
        s
    }

    pub fn compile(&self, model: &Model) -> Result<Compiled, Issue> {
        let scope = Self::declared_scope();
        match self.engine.compile_with_scope(&scope, &model.script) {
            Ok(ast) => Ok(Compiled {
                ast,
                script_line: model.script_line,
            }),
            Err(e) => {
                let code = match e.err_type() {
                    ParseErrorType::VariableUndefined(_) => "script.unknown_variable",
                    _ => "script.syntax",
                };
                Err(Issue::error(code)
                    .at(file_pos(e.position(), model.script_line))
                    .detail(e.err_type().to_string()))
            }
        }
    }

    pub fn run(
        &self,
        compiled: &Compiled,
        params: &IndexMap<String, ParamValue>,
        printer: &PrinterContext,
    ) -> RunOutput {
        {
            let mut l = self.log.borrow_mut();
            l.lines.clear();
            l.dropped = 0;
        }
        let mut p = rhai::Map::new();
        for (k, v) in params {
            let d = match *v {
                ParamValue::Float(f) => Dynamic::from_float(f),
                ParamValue::Int(i) => Dynamic::from_int(i),
                ParamValue::Bool(b) => Dynamic::from_bool(b),
            };
            p.insert(k.as_str().into(), d);
        }
        let mut m = rhai::Map::new();
        m.insert(
            "nozzle_diameter".into(),
            Dynamic::from_float(printer.nozzle_diameter),
        );
        m.insert(
            "layer_height".into(),
            Dynamic::from_float(printer.layer_height),
        );
        m.insert(
            "first_layer_height".into(),
            Dynamic::from_float(printer.first_layer_height),
        );

        let mut scope = Scope::new();
        scope.push_constant("PI", std::f64::consts::PI);
        scope.push_constant("TAU", std::f64::consts::TAU);
        scope.push_constant("params", p);
        scope.push_constant("printer", m);

        let result = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut scope, &compiled.ast);
        let mut issues = Vec::new();
        let path = match result {
            Ok(v) => match v.try_cast::<ScriptPath>() {
                Some(sp) => Some(sp.path),
                None => {
                    issues.push(Issue::error("script.no_path"));
                    None
                }
            },
            Err(e) => {
                issues.push(eval_issue(&e, compiled.script_line));
                None
            }
        };
        let mut log = std::mem::take(&mut self.log.borrow_mut().lines);
        let dropped = self.log.borrow().dropped;
        if dropped > 0 {
            log.push(format!("… {dropped}"));
        }
        RunOutput { path, issues, log }
    }
}

/// Position Rhai (dans le script) → position dans le fichier `.weave`.
fn file_pos(p: Position, script_line: usize) -> Option<(usize, usize)> {
    p.line()
        .map(|l| (l + script_line - 1, p.position().unwrap_or(1)))
}

fn eval_issue(e: &EvalAltResult, script_line: usize) -> Issue {
    use EvalAltResult as E;
    // Cause première (dans une fonction du script, l'erreur est enveloppée).
    let mut inner = e;
    let mut pos = e.position();
    while let E::ErrorInFunctionCall(_, _, cause, _) = inner {
        inner = cause;
        if !inner.position().is_none() {
            pos = inner.position();
        }
    }
    let (code, detail) = match inner {
        E::ErrorRuntime(d, _) => match d.clone().try_cast::<Fault>() {
            Some(f) => (f.code, f.detail),
            None => ("script.thrown", d.to_string()),
        },
        E::ErrorVariableNotFound(n, _) => ("script.unknown_variable", n.clone()),
        E::ErrorPropertyNotFound(n, _) => ("script.unknown_property", n.clone()),
        E::ErrorFunctionNotFound(n, _) => ("script.unknown_function", n.clone()),
        E::ErrorTooManyOperations(_) => ("script.too_many_operations", String::new()),
        E::ErrorStackOverflow(_) => ("script.too_deep", String::new()),
        E::ErrorDataTooLarge(what, _) => ("script.data_too_large", what.clone()),
        E::ErrorArithmetic(m, _) => ("script.arithmetic", m.clone()),
        E::ErrorArrayBounds(..) | E::ErrorStringBounds(..) | E::ErrorIndexNotFound(..) => {
            ("script.index_out_of_range", inner.to_string())
        }
        E::ErrorMismatchDataType(..) | E::ErrorMismatchOutputType(..) | E::ErrorFor(_) => {
            ("script.wrong_type", inner.to_string())
        }
        E::ErrorAssignmentToConstant(n, _) => ("script.constant_assignment", n.clone()),
        _ => ("script.runtime", inner.to_string()),
    };
    let issue = Issue::error(code).at(file_pos(pos, script_line));
    if detail.is_empty() {
        issue
    } else {
        issue.detail(detail)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model;

    const PRINTER: PrinterContext = PrinterContext {
        nozzle_diameter: 0.4,
        layer_height: 0.2,
        first_layer_height: 0.2,
    };

    fn run_src(body: &str) -> RunOutput {
        let src = format!(
            "[meta]\nname = \"t\"\ncategory = \"t\"\n[params.n]\ntype = \"int\"\nlabel = \"n\"\ndefault = 3\nmin = 1\nmax = 10\n---\n{body}"
        );
        let m = model::parse(&src).unwrap();
        let eng = ScriptEngine::new(Limits::default());
        let c = match eng.compile(&m) {
            Ok(c) => c,
            Err(i) => {
                return RunOutput {
                    path: None,
                    issues: vec![i],
                    log: vec![],
                }
            }
        };
        let params = IndexMap::from([("n".to_string(), ParamValue::Int(3))]);
        eng.run(&c, &params, &PRINTER)
    }

    #[test]
    fn builds_a_path_with_mixed_numbers() {
        let out = run_src(
            "let p = new_path();\np.travel_to(0, 0, 0.2);\nfor i in 0..params.n { p.print_to(i * 1.5, 0, 0.2, printer.nozzle_diameter, 0.2); }\np.set_speed(25);\np.print_polar(10, PI / 2, 0.4, 0.5, 0.2);\nprint(`n = ${params.n}`);\np",
        );
        assert!(out.issues.is_empty(), "{:?}", out.issues);
        let pts = out.path.unwrap().points;
        assert_eq!(pts.len(), 5);
        assert_eq!(pts[0].kind, MoveKind::Travel);
        assert_eq!(pts[3].x, 3.0);
        assert!(pts[4].x.abs() < 1e-12 && (pts[4].y - 10.0).abs() < 1e-12);
        assert_eq!(pts[4].speed, Speed::Absolute(25.0));
        assert_eq!(out.log, ["n = 3"]);
    }

    #[test]
    fn function_call_style_also_mutates_the_path() {
        let out = run_src("let p = new_path();\nprint_to(p, 1, 2, 0.2, 0.4, 0.2);\np");
        assert_eq!(out.path.unwrap().points.len(), 1);
    }

    #[test]
    fn mixed_integer_float_arithmetic_works() {
        let out = run_src("let p = new_path();\nlet a = 2 * 0.5 + params.n / 2.0 + sin(0) + max(1, 2.5);\np.print_to(a, 0, 0.2, 0.4, 0.2);\np");
        assert!(out.issues.is_empty(), "{:?}", out.issues);
        assert_eq!(out.path.unwrap().points[0].x, 1.0 + 1.5 + 0.0 + 2.5);
    }

    #[test]
    fn errors_have_stable_codes_and_file_lines() {
        // Le script commence à la ligne 11 du fichier.
        let out = run_src("let p = new_path();\nlet r = params.radius;\np");
        assert_eq!(out.issues[0].code, "script.unknown_property");
        assert_eq!(out.issues[0].line, Some(12));

        let out = run_src("let p = new_path();\np.print_to(0, 0, 0.2, -1, 0.2);\np");
        assert_eq!(out.issues[0].code, "path.invalid_width");
        assert_eq!(out.issues[0].line, Some(12));

        let out = run_src("let p = new_path();\np.print_to(0, 0, 0.2, rayon, 0.2);\np");
        assert_eq!(out.issues[0].code, "script.unknown_variable");
        assert_eq!(out.issues[0].line, Some(12));

        let out = run_src("let p = new_path();\np.print_to(0, 0, 0.2);\np");
        assert_eq!(out.issues[0].code, "script.unknown_function");

        let out = run_src("let p = new_path(;\np");
        assert_eq!(out.issues[0].code, "script.syntax");
        assert_eq!(out.issues[0].line, Some(11));

        let out = run_src("42");
        assert_eq!(out.issues[0].code, "script.no_path");

        let out = run_src("params.n = 4;\nnew_path()");
        assert_eq!(out.issues[0].code, "script.constant_assignment");

        let out = run_src("let p = new_path();\np.set_fan(120);\np");
        assert_eq!(out.issues[0].code, "path.invalid_fan");
    }

    #[test]
    fn runaway_scripts_are_stopped() {
        let out = run_src("let p = new_path();\nloop { }\np");
        assert_eq!(out.issues[0].code, "script.too_many_operations");
    }

    #[test]
    fn engine_functions_override_the_standard_package() {
        // Les fonctions enregistrées sur le moteur passent avant celles du
        // paquet standard : c'est ce qui garantit l'usage de `libm`.
        let mut e = Engine::new();
        e.register_fn("sin", |_: FLOAT| 42.0 as FLOAT);
        assert_eq!(e.eval::<FLOAT>("sin(1.0)").unwrap(), 42.0);
        assert_eq!(e.eval::<FLOAT>("1.0.sin()").unwrap(), 42.0);
    }

    #[test]
    fn compiled_script_follows_new_parameter_values() {
        let src = "[meta]\nname = \"t\"\ncategory = \"t\"\n[params.n]\ntype = \"int\"\nlabel = \"n\"\ndefault = 3\nmin = 1\nmax = 10\n---\nlet p = new_path();\nfor i in 0..params.n { p.print_to(i, 0, 0.2, 0.4, 0.2); }\np";
        let m = model::parse(src).unwrap();
        let eng = ScriptEngine::new(Limits::default());
        let c = eng.compile(&m).unwrap();
        for n in [2, 7] {
            let params = IndexMap::from([("n".to_string(), ParamValue::Int(n))]);
            let out = eng.run(&c, &params, &PRINTER);
            assert_eq!(out.path.unwrap().points.len(), n as usize);
        }
    }
}
