//! Lecture d'un modèle `.weave` : un en-tête TOML (`[meta]`, `[params.<id>]`),
//! une ligne `---`, puis le corps Rhai qui trace le parcours (ADR-0002).
//!
//! L'en-tête est lu strictement : un champ inconnu (faute de frappe), un type
//! de paramètre inconnu, des bornes incohérentes produisent une erreur à code
//! stable, avec la ligne et la colonne dans le fichier.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use toml::Spanned;

use crate::issue::{line_col, Issue};

/// Identifiant du format, facultatif dans le fichier.
pub const FORMAT: &str = "fabweave.model";
/// Version du format lue et écrite par ce moteur.
/// Version 2 : paramètres `curve` (une silhouette en points) et `image` (un
/// dessin SVG ou PNG). Un modèle qui s'en sert déclare `version = 2` ; les
/// modèles de version 1 se lisent tels quels.
pub const VERSION: u32 = 2;

/// Texte affiché : une chaîne, ou une table par langue (`{ fr = "…", en = "…" }`).
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Text {
    Plain(String),
    Localized(IndexMap<String, String>),
}

impl Text {
    /// Texte dans la langue demandée, sinon le français, sinon le premier.
    pub fn get(&self, lang: &str) -> &str {
        match self {
            Text::Plain(s) => s,
            Text::Localized(m) => m
                .get(lang)
                .or_else(|| m.get("fr"))
                .or_else(|| m.values().next())
                .map_or("", String::as_str),
        }
    }

    /// Un caractère de contrôle (retour à la ligne…) dans l'une des langues :
    /// recopié dans le G-code, il y ouvrirait une ligne de commande.
    pub fn has_control(&self) -> bool {
        let bad = |s: &str| s.chars().any(char::is_control);
        match self {
            Text::Plain(s) => bad(s),
            Text::Localized(m) => m.values().any(|s| bad(s)),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Meta {
    pub name: Text,
    pub category: Text,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub tags: Vec<String>,
    /// Origine d'un modèle repris d'ailleurs (URL, notebook FullControl…).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Diamètres de buse conseillés (mm) ; vide : tous.
    pub recommended_nozzle: Vec<f64>,
    /// Adhésion conseillée ; l'utilisateur choisit (le modèle ne l'impose pas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended_adhesion: Option<RecommendedAdhesion>,
    /// Modèle d'essai (tour de température, test des rétractions) : seul à
    /// pouvoir changer la température et la rétraction du profil, dans des
    /// bornes fixes (ADR-0007).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub calibration: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAdhesion {
    None,
    Brim,
    Raft,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ParamKind {
    Float {
        default: f64,
        min: f64,
        max: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<f64>,
    },
    Int {
        default: i64,
        min: i64,
        max: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        step: Option<i64>,
    },
    Bool {
        default: bool,
    },
    /// Une valeur parmi une liste (le script reçoit la chaîne `value`).
    Choice {
        default: String,
        options: Vec<ChoiceOption>,
    },
    /// Silhouette `[rayon, hauteur]` (version 2 du format, `curve.rs`).
    Curve {
        default: Vec<[f64; 2]>,
        /// `[rayon, hauteur]` minimaux et maximaux.
        min: [f64; 2],
        max: [f64; 2],
        /// Nombre de points minimal et maximal.
        points: [usize; 2],
        /// Raccord lisse (sinon droit).
        smooth: bool,
        /// Silhouettes prêtes, proposées par l'interface.
        #[serde(skip_serializing_if = "Vec::is_empty")]
        presets: Vec<CurvePreset>,
    },
    /// Dessin SVG ou PNG (version 2, `drawing.rs`) : `default` est le texte
    /// SVG ou le PNG en `data:image/png;base64,…`, vide pour aucun dessin.
    Image {
        default: String,
        /// Plus grand côté du dessin (mm).
        size: f64,
        /// Seuil d'encre d'un PNG sans transparence (luminance 0–255).
        threshold: f64,
        /// Dessin par défaut lu une fois.
        #[serde(skip)]
        parsed: crate::drawing::Drawing,
    },
}

impl ParamKind {
    /// Bornes d'un paramètre `curve`.
    pub fn curve_bounds(&self) -> Option<crate::curve::CurveBounds> {
        match *self {
            ParamKind::Curve {
                min, max, points, ..
            } => Some(crate::curve::CurveBounds { min, max, points }),
            _ => None,
        }
    }
}

/// Silhouette nommée d'un paramètre `curve`.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CurvePreset {
    pub name: Text,
    pub points: Vec<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChoiceOption {
    pub value: String,
    /// Libellé affiché ; absent : la valeur elle-même.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<Text>,
}

/// Paramètre réglable : un curseur, un champ ou une case dans l'interface.
#[derive(Clone, Debug, Serialize)]
pub struct ParamDef {
    pub label: Text,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Infobulle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<Text>,
    /// Rangé derrière « ▸ Avancé ».
    pub advanced: bool,
    #[serde(flatten)]
    pub kind: ParamKind,
}

/// Modèle lu : description, paramètres (dans l'ordre du fichier) et script.
#[derive(Clone, Debug, Serialize)]
pub struct Model {
    pub meta: Meta,
    pub params: IndexMap<String, ParamDef>,
    #[serde(skip)]
    pub script: String,
    /// Ligne du fichier où commence le script (à partir de 1).
    #[serde(skip)]
    pub script_line: usize,
}

// --- Lecture brute (serde), puis contrôle et conversion. ---

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawHeader {
    format: Option<Spanned<String>>,
    version: Option<Spanned<u32>>,
    meta: RawMeta,
    #[serde(default)]
    params: IndexMap<Spanned<String>, Spanned<RawParam>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMeta {
    name: Text,
    category: Text,
    description: Option<Text>,
    author: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    source: Option<String>,
    license: Option<String>,
    #[serde(default)]
    recommended_nozzle: Vec<f64>,
    recommended_adhesion: Option<RecommendedAdhesion>,
    #[serde(default)]
    calibration: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawParam {
    #[serde(rename = "type")]
    kind: Spanned<String>,
    label: Text,
    unit: Option<String>,
    help: Option<Text>,
    #[serde(default)]
    advanced: bool,
    default: Spanned<toml::Value>,
    min: Option<Spanned<toml::Value>>,
    max: Option<Spanned<toml::Value>>,
    step: Option<Spanned<toml::Value>>,
    options: Option<Spanned<Vec<RawOption>>>,
    /// `curve` : nombre de points `[min, max]`, raccord, silhouettes prêtes.
    points: Option<Spanned<toml::Value>>,
    smooth: Option<Spanned<bool>>,
    presets: Option<Spanned<Vec<CurvePreset>>>,
    /// `image` : taille (mm) et seuil d'encre.
    size: Option<Spanned<toml::Value>>,
    threshold: Option<Spanned<toml::Value>>,
}

/// Option d'un paramètre `choice` : `"zigzag"` ou `{ value = "zigzag", label = … }`.
#[derive(Deserialize)]
#[serde(untagged)]
enum RawOption {
    Value(String),
    Labelled { value: String, label: Option<Text> },
}

/// Découpe le fichier : (début de l'en-tête, fin de l'en-tête, début du
/// script, ligne du script). Une ligne `---` en tête de fichier (style
/// *frontmatter*) est acceptée et ignorée.
fn split(source: &str) -> Option<(usize, usize, usize, usize)> {
    let mut header_start = 0;
    let mut offset = 0;
    let mut first_content = true;
    for (index, raw) in source.split_inclusive('\n').enumerate() {
        let line = raw.trim_end_matches(['\n', '\r']).trim_end();
        let next = offset + raw.len();
        if line == "---" {
            if first_content {
                header_start = next;
            } else {
                return Some((header_start, offset, next, index + 2));
            }
        }
        if !line.trim().is_empty() {
            first_content = false;
        }
        offset = next;
    }
    None
}

/// Lit un fichier `.weave`. En cas d'échec, tous les messages trouvés.
pub fn parse(source: &str) -> Result<Model, Vec<Issue>> {
    let Some((h_start, h_end, s_start, script_line)) = split(source) else {
        return Err(vec![Issue::error("model.missing_separator")]);
    };
    let header = &source[h_start..h_end];
    // Position dans le fichier entier d'un intervalle de l'en-tête.
    let pos = |span: std::ops::Range<usize>| Some(line_col(source, h_start + span.start));

    let raw: RawHeader = toml::from_str(header).map_err(|e| {
        vec![Issue::error("model.invalid_header")
            .at(e.span().and_then(pos))
            .detail(e.message().to_string())]
    })?;

    let mut issues = Vec::new();
    if let Some(f) = &raw.format {
        if f.get_ref() != FORMAT {
            issues.push(
                Issue::error("model.wrong_format")
                    .field("format")
                    .at(pos(f.span()))
                    .detail(f.get_ref().clone()),
            );
        }
    }
    if let Some(v) = &raw.version {
        if *v.get_ref() == 0 || *v.get_ref() > VERSION {
            issues.push(
                Issue::error("model.unsupported_version")
                    .field("version")
                    .value(f64::from(*v.get_ref()))
                    .at(pos(v.span())),
            );
        }
    }
    if raw
        .meta
        .recommended_nozzle
        .iter()
        .any(|d| !(*d > 0.0 && d.is_finite()))
    {
        issues.push(Issue::error("model.invalid_nozzle").field("meta.recommended_nozzle"));
    }
    if raw.meta.name.has_control() {
        issues.push(Issue::error("model.control_character").field("meta.name"));
    }

    let mut params = IndexMap::new();
    for (id, def) in raw.params {
        let id_pos = pos(id.span());
        let id = id.into_inner();
        let def_pos = pos(def.span());
        let version = raw.version.as_ref().map_or(1, |v| *v.get_ref());
        match convert_param(&id, def.into_inner(), version, &pos) {
            Ok(d) => {
                if !is_identifier(&id) {
                    issues.push(
                        Issue::error("param.invalid_id")
                            .field(format!("params.{id}"))
                            .at(id_pos),
                    );
                } else {
                    params.insert(id, d);
                }
            }
            Err(mut e) => {
                for i in &mut e {
                    if i.line.is_none() {
                        i.line = def_pos.map(|p| p.0);
                        i.column = def_pos.map(|p| p.1);
                    }
                }
                issues.extend(e);
            }
        }
    }

    if crate::issue::has_error(&issues) {
        return Err(issues);
    }
    let m = raw.meta;
    Ok(Model {
        meta: Meta {
            name: m.name,
            category: m.category,
            description: m.description,
            author: m.author,
            tags: m.tags,
            source: m.source,
            license: m.license,
            recommended_nozzle: m.recommended_nozzle,
            recommended_adhesion: m.recommended_adhesion,
            calibration: m.calibration,
        },
        params,
        script: source[s_start..].to_string(),
        script_line,
    })
}

/// Identifiant utilisable dans le script (`params.<id>`).
fn is_identifier(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase() || c == '_')
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn number(v: &toml::Value) -> Option<f64> {
    match v {
        toml::Value::Float(f) => Some(*f),
        toml::Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

fn integer(v: &toml::Value) -> Option<i64> {
    match v {
        toml::Value::Integer(i) => Some(*i),
        toml::Value::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(*f as i64),
        _ => None,
    }
}

fn convert_param(
    id: &str,
    p: RawParam,
    version: u32,
    pos: &dyn Fn(std::ops::Range<usize>) -> Option<(usize, usize)>,
) -> Result<ParamDef, Vec<Issue>> {
    let field = format!("params.{id}");
    let mut issues = Vec::new();
    let err = |code: &'static str, span: std::ops::Range<usize>| {
        Issue::error(code).field(field.clone()).at(pos(span))
    };
    let required = |v: &Option<Spanned<toml::Value>>, name: &str, issues: &mut Vec<Issue>| {
        if v.is_none() {
            issues.push(
                Issue::error("param.missing_field")
                    .field(field.clone())
                    .detail(name.to_string()),
            );
        }
    };

    if p.kind.get_ref() != "choice" {
        if let Some(o) = &p.options {
            issues.push(err("param.unexpected_field", o.span()).detail("options".to_string()));
        }
    }
    if p.kind.get_ref() != "curve" {
        let spans = [
            p.points.as_ref().map(|v| (v.span(), "points")),
            p.smooth.as_ref().map(|v| (v.span(), "smooth")),
            p.presets.as_ref().map(|v| (v.span(), "presets")),
        ];
        for (span, name) in spans.into_iter().flatten() {
            issues.push(err("param.unexpected_field", span).detail(name.to_string()));
        }
    }
    if p.kind.get_ref() != "image" {
        for (v, name) in [(&p.size, "size"), (&p.threshold, "threshold")] {
            if let Some(v) = v {
                issues.push(err("param.unexpected_field", v.span()).detail(name.to_string()));
            }
        }
    }
    let kind = match p.kind.get_ref().as_str() {
        "image" => {
            if version < 2 {
                issues.push(err("param.requires_version", p.kind.span()).value(2.0));
                return Err(issues);
            }
            for (v, name) in [(&p.min, "min"), (&p.max, "max"), (&p.step, "step")] {
                if let Some(v) = v {
                    issues.push(err("param.unexpected_field", v.span()).detail(name.to_string()));
                }
            }
            let size = match &p.size {
                None => {
                    issues
                        .push(err("param.missing_field", p.kind.span()).detail("size".to_string()));
                    0.0
                }
                Some(v) => number(v.get_ref())
                    .filter(|s| *s > 0.0 && *s <= 1000.0)
                    .unwrap_or_else(|| {
                        issues
                            .push(err("param.invalid_bounds", v.span()).detail("size".to_string()));
                        0.0
                    }),
            };
            let threshold = match &p.threshold {
                None => 128.0,
                Some(v) => number(v.get_ref())
                    .filter(|t| (0.0..=255.0).contains(t))
                    .unwrap_or_else(|| {
                        issues.push(
                            err("param.invalid_bounds", v.span()).detail("threshold".to_string()),
                        );
                        128.0
                    }),
            };
            let default = match p.default.get_ref() {
                toml::Value::String(s) => s.clone(),
                _ => {
                    issues.push(err("param.wrong_type", p.default.span()));
                    String::new()
                }
            };
            let parsed = if issues.is_empty() {
                crate::drawing::read(&default, size, threshold).unwrap_or_else(|(code, d)| {
                    issues.push(
                        err("param.default_out_of_range", p.default.span())
                            .detail(format!("{code} {d}")),
                    );
                    crate::drawing::Drawing::default()
                })
            } else {
                crate::drawing::Drawing::default()
            };
            ParamKind::Image {
                default,
                size,
                threshold,
                parsed,
            }
        }
        "curve" => {
            if version < 2 {
                issues.push(err("param.requires_version", p.kind.span()).value(2.0));
                return Err(issues);
            }
            convert_curve(&p, &err, &mut issues)
        }
        "float" => {
            required(&p.min, "min", &mut issues);
            required(&p.max, "max", &mut issues);
            let get = |v: &Spanned<toml::Value>, issues: &mut Vec<Issue>| {
                let n = number(v.get_ref());
                if n.is_none_or(|n| !n.is_finite()) {
                    issues.push(err("param.not_a_number", v.span()));
                }
                n.unwrap_or(0.0)
            };
            let default = get(&p.default, &mut issues);
            let min = p.min.as_ref().map(|v| get(v, &mut issues));
            let max = p.max.as_ref().map(|v| get(v, &mut issues));
            let step = p.step.as_ref().map(|v| get(v, &mut issues));
            if let (Some(min), Some(max)) = (min, max) {
                check_bounds(min, max, default, step, &p, &err, &mut issues);
            }
            ParamKind::Float {
                default,
                min: min.unwrap_or(0.0),
                max: max.unwrap_or(0.0),
                step,
            }
        }
        "int" => {
            required(&p.min, "min", &mut issues);
            required(&p.max, "max", &mut issues);
            let get = |v: &Spanned<toml::Value>, issues: &mut Vec<Issue>| {
                let n = integer(v.get_ref());
                if n.is_none() {
                    issues.push(err("param.not_an_integer", v.span()));
                }
                n.unwrap_or(0)
            };
            let default = get(&p.default, &mut issues);
            let min = p.min.as_ref().map(|v| get(v, &mut issues));
            let max = p.max.as_ref().map(|v| get(v, &mut issues));
            let step = p.step.as_ref().map(|v| get(v, &mut issues));
            if let (Some(min), Some(max)) = (min, max) {
                check_bounds(
                    min as f64,
                    max as f64,
                    default as f64,
                    step.map(|s| s as f64),
                    &p,
                    &err,
                    &mut issues,
                );
            }
            ParamKind::Int {
                default,
                min: min.unwrap_or(0),
                max: max.unwrap_or(0),
                step,
            }
        }
        "bool" | "choice" => {
            for (v, name) in [(&p.min, "min"), (&p.max, "max"), (&p.step, "step")] {
                if let Some(v) = v {
                    issues.push(err("param.unexpected_field", v.span()).detail(name.to_string()));
                }
            }
            if p.kind.get_ref() == "choice" {
                let Some(opts) = &p.options else {
                    issues.push(
                        Issue::error("param.missing_field")
                            .field(field.clone())
                            .detail("options"),
                    );
                    return Err(issues);
                };
                let options: Vec<ChoiceOption> = opts
                    .get_ref()
                    .iter()
                    .map(|o| match o {
                        RawOption::Value(v) => ChoiceOption {
                            value: v.clone(),
                            label: None,
                        },
                        RawOption::Labelled { value, label } => ChoiceOption {
                            value: value.clone(),
                            label: label.clone(),
                        },
                    })
                    .collect();
                if options.is_empty()
                    || options
                        .iter()
                        .any(|o| o.value.is_empty() || o.value.chars().any(char::is_control))
                {
                    issues.push(err("param.invalid_options", opts.span()));
                }
                for (i, o) in options.iter().enumerate() {
                    if options[..i].iter().any(|x| x.value == o.value) {
                        issues.push(
                            err("param.duplicate_option", opts.span()).detail(o.value.clone()),
                        );
                    }
                }
                let default = match p.default.get_ref() {
                    toml::Value::String(s) => s.clone(),
                    _ => String::new(),
                };
                if !options.iter().any(|o| o.value == default) {
                    issues.push(
                        err("param.unknown_option", p.default.span()).detail(default.clone()),
                    );
                }
                if !issues.is_empty() {
                    return Err(issues);
                }
                return Ok(ParamDef {
                    label: p.label,
                    unit: p.unit,
                    help: p.help,
                    advanced: p.advanced,
                    kind: ParamKind::Choice { default, options },
                });
            }
            let default = match p.default.get_ref() {
                toml::Value::Boolean(b) => *b,
                _ => {
                    issues.push(err("param.not_a_boolean", p.default.span()));
                    false
                }
            };
            ParamKind::Bool { default }
        }
        other => {
            issues.push(err("param.unknown_type", p.kind.span()).detail(other.to_string()));
            return Err(issues);
        }
    };
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(ParamDef {
        label: p.label,
        unit: p.unit,
        help: p.help,
        advanced: p.advanced,
        kind,
    })
}

/// `[a, b]` de deux nombres finis (bornes d'une courbe).
fn pair(v: &toml::Value) -> Option<[f64; 2]> {
    match v.as_array()?.as_slice() {
        [a, b] => {
            let (a, b) = (number(a)?, number(b)?);
            (a.is_finite() && b.is_finite()).then_some([a, b])
        }
        _ => None,
    }
}

fn convert_curve(
    p: &RawParam,
    err: &dyn Fn(&'static str, std::ops::Range<usize>) -> Issue,
    issues: &mut Vec<Issue>,
) -> ParamKind {
    use crate::curve::{check, points_from_json, CurveBounds, MAX_POINTS};
    if let Some(s) = &p.step {
        issues.push(err("param.unexpected_field", s.span()).detail("step".to_string()));
    }
    let mut bound = |v: &Option<Spanned<toml::Value>>, name: &str| match v {
        None => {
            issues.push(err("param.missing_field", p.kind.span()).detail(name.to_string()));
            [0.0; 2]
        }
        Some(v) => pair(v.get_ref()).unwrap_or_else(|| {
            issues.push(err("param.not_a_number", v.span()));
            [0.0; 2]
        }),
    };
    let (min, max) = (bound(&p.min, "min"), bound(&p.max, "max"));
    let points = match &p.points {
        None => [2, 32],
        Some(v) => match pair(v.get_ref()) {
            Some([a, b])
                if a.fract() == 0.0
                    && b.fract() == 0.0
                    && a >= 1.0
                    && a <= b
                    && b <= MAX_POINTS as f64 =>
            {
                [a as usize, b as usize]
            }
            _ => {
                issues.push(err("param.invalid_bounds", v.span()).detail("points".to_string()));
                [2, 32]
            }
        },
    };
    if min[0] >= max[0] || min[1] >= max[1] {
        issues.push(err(
            "param.invalid_bounds",
            p.max.as_ref().map_or(0..0, Spanned::span),
        ));
    }
    let bounds = CurveBounds { min, max, points };
    // Points par défaut et silhouettes : mêmes contrôles qu'une valeur.
    let json = |v: &toml::Value| serde_json::to_value(v).ok();
    let default = json(p.default.get_ref()).and_then(|v| points_from_json(&v));
    let default = match default {
        None => {
            issues.push(err("param.not_a_number", p.default.span()));
            Vec::new()
        }
        Some(d) => {
            if issues.is_empty() {
                if let Err(e) = check(&d, &bounds) {
                    issues.push(
                        err("param.default_out_of_range", p.default.span())
                            .detail(e.code.to_string()),
                    );
                }
            }
            d
        }
    };
    let presets = p
        .presets
        .as_ref()
        .map_or_else(Vec::new, |v| v.get_ref().clone());
    if issues.is_empty() {
        for (i, pr) in presets.iter().enumerate() {
            if check(&pr.points, &bounds).is_err() || pr.name.has_control() {
                let span = p.presets.as_ref().map_or(0..0, Spanned::span);
                issues.push(err("param.invalid_preset", span).count(i + 1));
            }
        }
    }
    ParamKind::Curve {
        default,
        min,
        max,
        points,
        smooth: p.smooth.as_ref().is_none_or(|s| *s.get_ref()),
        presets,
    }
}

fn check_bounds(
    min: f64,
    max: f64,
    default: f64,
    step: Option<f64>,
    p: &RawParam,
    err: &dyn Fn(&'static str, std::ops::Range<usize>) -> Issue,
    issues: &mut Vec<Issue>,
) {
    if !issues.is_empty() {
        return;
    }
    let span = |v: &Option<Spanned<toml::Value>>| v.as_ref().map_or(0..0, Spanned::span);
    if min >= max {
        issues.push(err("param.invalid_bounds", span(&p.max)));
    } else if default < min || default > max {
        issues.push(err("param.default_out_of_range", p.default.span()).value(default));
    }
    if let Some(s) = step {
        if s <= 0.0 {
            issues.push(err("param.invalid_step", span(&p.step)).value(s));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"[meta]
name = "Test"
category = "Essais"

[params.radius]
type = "float"
label = { fr = "Rayon", en = "Radius" }
unit = "mm"
default = 15
min = 5
max = 50

[params.turns]
type = "int"
label = "Tours"
default = 6
min = 1
max = 20

[params.closed]
type = "bool"
label = "Fermé"
default = true
---
let p = new_path();
p
"#;

    #[test]
    fn reads_header_and_script() {
        let m = parse(MINIMAL).unwrap();
        assert_eq!(m.meta.name.get("en"), "Test");
        let ids: Vec<_> = m.params.keys().cloned().collect();
        assert_eq!(ids, ["radius", "turns", "closed"]);
        assert_eq!(m.params["radius"].label.get("en"), "Radius");
        assert_eq!(
            m.params["radius"].kind,
            ParamKind::Float {
                default: 15.0,
                min: 5.0,
                max: 50.0,
                step: None
            }
        );
        assert_eq!(m.script_line, 25);
        assert!(m.script.starts_with("let p = new_path();"));
    }

    #[test]
    fn leading_frontmatter_separator_is_accepted() {
        let src = format!("---\n{MINIMAL}");
        let m = parse(&src).unwrap();
        assert_eq!(m.script_line, 26);
    }

    #[test]
    fn missing_separator_is_an_error() {
        let e = parse("[meta]\nname = \"x\"\ncategory = \"y\"\n").unwrap_err();
        assert_eq!(e[0].code, "model.missing_separator");
    }

    #[test]
    fn unknown_field_is_reported_with_its_line() {
        let src = MINIMAL.replace("default = 6", "defualt = 6");
        let e = parse(&src).unwrap_err();
        assert_eq!(e[0].code, "model.invalid_header");
        assert_eq!(e[0].line, Some(16));
        assert!(e[0].detail.as_deref().unwrap().contains("defualt"));
    }

    #[test]
    fn inconsistent_bounds_are_errors() {
        let e = parse(&MINIMAL.replace("default = 15", "default = 60")).unwrap_err();
        assert_eq!(e[0].code, "param.default_out_of_range");
        assert_eq!(e[0].field.as_deref(), Some("params.radius"));
        assert_eq!(e[0].line, Some(9));
        let e = parse(&MINIMAL.replace("min = 5", "min = 80")).unwrap_err();
        assert_eq!(e[0].code, "param.invalid_bounds");
        let e = parse(&MINIMAL.replace("type = \"int\"", "type = \"integer\"")).unwrap_err();
        assert_eq!(e[0].code, "param.unknown_type");
        let e = parse(&MINIMAL.replace("default = 6", "default = 6.5")).unwrap_err();
        assert_eq!(e[0].code, "param.not_an_integer");
    }

    #[test]
    fn choice_parameters() {
        let src = MINIMAL.replace(
            "[params.closed]",
            "[params.pattern]\ntype = \"choice\"\nlabel = \"Motif\"\noptions = [\"zigzag\", { value = \"smooth\", label = { fr = \"Lisse\", en = \"Smooth\" } }]\ndefault = \"smooth\"\n\n[params.closed]",
        );
        let m = parse(&src).unwrap();
        match &m.params["pattern"].kind {
            ParamKind::Choice { default, options } => {
                assert_eq!(default, "smooth");
                assert_eq!(options[0].value, "zigzag");
                assert_eq!(options[1].label.as_ref().unwrap().get("en"), "Smooth");
            }
            k => panic!("{k:?}"),
        }
        let e = parse(&src.replace("default = \"smooth\"", "default = \"wavy\"")).unwrap_err();
        assert_eq!(e[0].code, "param.unknown_option");
        let e =
            parse(&src.replace("options = [\"zigzag\", ", "options = [\"smooth\", ")).unwrap_err();
        assert_eq!(e[0].code, "param.duplicate_option");
    }

    #[test]
    fn invalid_identifier_and_version() {
        let e = parse(&MINIMAL.replace("[params.turns]", "[params.Turns]")).unwrap_err();
        assert_eq!(e[0].code, "param.invalid_id");
        let e = parse(&format!("version = 9\n{MINIMAL}")).unwrap_err();
        assert_eq!(e[0].code, "model.unsupported_version");
    }

    #[test]
    fn curve_parameters_need_version_2() {
        let curve = "[params.profile]\ntype = \"curve\"\nlabel = \"Profil\"\ndefault = [[30, 0], [40, 50]]\nmin = [5, 0]\nmax = [80, 200]\npoints = [2, 6]\nsmooth = false\npresets = [{ name = { fr = \"Droit\", en = \"Straight\" }, points = [[30, 0], [30, 80]] }]\n\n[params.closed]";
        let src = MINIMAL.replace("[params.closed]", curve);
        let e = parse(&src).unwrap_err();
        assert_eq!(e[0].code, "param.requires_version");
        let m = parse(&format!("version = 2\n{src}")).unwrap();
        match &m.params["profile"].kind {
            ParamKind::Curve {
                default,
                points,
                smooth,
                presets,
                ..
            } => {
                assert_eq!(default, &vec![[30.0, 0.0], [40.0, 50.0]]);
                assert_eq!(*points, [2, 6]);
                assert!(!smooth);
                assert_eq!(presets[0].points[1], [30.0, 80.0]);
            }
            k => panic!("{k:?}"),
        }
        let v2 = |a: &str, b: &str| {
            parse(&format!("version = 2\n{}", src.replace(a, b))).unwrap_err()[0].code
        };
        assert_eq!(
            v2("[[30, 0], [40, 50]]", "[[30, 0], [40, 0]]"),
            "param.default_out_of_range"
        );
        assert_eq!(
            v2("[[30, 0], [40, 50]]", "[[30, 0], [90, 50]]"),
            "param.default_out_of_range"
        );
        assert_eq!(
            v2("points = [2, 6]", "points = [3, 2]"),
            "param.invalid_bounds"
        );
        assert_eq!(
            v2("[[30, 0], [30, 80]]", "[[30, 0], [30, 300]]"),
            "param.invalid_preset"
        );
        assert_eq!(v2("min = [5, 0]", "min = [5]"), "param.not_a_number");
        // Champs d'une courbe sur un autre type : refusés.
        let e = parse(&MINIMAL.replace("default = 15", "default = 15\nsmooth = true")).unwrap_err();
        assert_eq!(e[0].code, "param.unexpected_field");
    }

    #[test]
    fn image_parameters() {
        let img = "[params.logo]\ntype = \"image\"\nlabel = \"Logo\"\nsize = 40\ndefault = '<svg><rect x=\"0\" y=\"0\" width=\"10\" height=\"5\"/></svg>'\n\n[params.closed]";
        let src = MINIMAL.replace("[params.closed]", img);
        assert_eq!(parse(&src).unwrap_err()[0].code, "param.requires_version");
        let m = parse(&format!("version = 2\n{src}")).unwrap();
        match &m.params["logo"].kind {
            ParamKind::Image { size, parsed, .. } => {
                assert_eq!(*size, 40.0);
                assert!((parsed.width - 40.0).abs() < 1e-9 && (parsed.height - 20.0).abs() < 1e-9);
            }
            k => panic!("{k:?}"),
        }
        let v2 = |a: &str, b: &str| {
            parse(&format!("version = 2\n{}", src.replace(a, b))).unwrap_err()[0].code
        };
        assert_eq!(v2("size = 40\n", ""), "param.missing_field");
        assert_eq!(v2("size = 40", "size = 0"), "param.invalid_bounds");
        assert_eq!(v2("<rect", "<blob"), "param.default_out_of_range");
        assert_eq!(
            v2("size = 40", "size = 40\nmin = 1"),
            "param.unexpected_field"
        );
    }
}
