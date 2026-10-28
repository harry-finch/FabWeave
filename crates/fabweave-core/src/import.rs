//! Import de profils depuis les trancheurs : PrusaSlicer (`.ini`, export
//! d'une configuration ou paquet de préréglages), Bambu Studio et
//! OrcaSlicer (`.json`).
//!
//! Chaque profil trouvé part d'un profil FabWeave de base (même firmware
//! pour une imprimante, même matière pour un filament) ; les valeurs
//! présentes dans le fichier le remplacent. Le rapport liste les valeurs
//! reprises (avec leur clé d'origine), les valeurs absentes (gardées de la
//! base, laissées vides ou estimées) et les clés ignorées : rien n'est
//! complété en silence.
//!
//! Les gabarits de démarrage et de fin sont traduits de la syntaxe des
//! trancheurs (`[var]`, `{expr}`, `{if}…{elsif}…{else}…{endif}`) vers les
//! expressions Rhai de FabWeave ; ce qui ne se traduit pas reste tel quel
//! (le profil signale alors l'erreur à sa ligne) et est listé. Une
//! imprimante importée porte `review = ["gcode.start", "gcode.end"]` :
//! aucun G-code n'est exporté avant relecture.

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::Value;

use crate::issue::Issue;
use crate::model::Text;
use crate::profile::{
    same_diameter, BedShape, FilamentProfile, Firmware, Origin, PrinterProfile, Zone,
    FILAMENT_FORMAT, FILAMENT_VERSION, PRINTER_FORMAT, PRINTER_VERSION, REVIEW_FIELDS,
};

/// Valeur absente du fichier : ce que devient le champ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Fallback {
    /// Valeur du profil de base gardée.
    Base,
    /// Champ laissé vide (propre à la machine ou au filament de base).
    Empty,
    /// Valeur déduite d'une autre (hauteur de couche d'après la buse).
    Estimated,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Taken {
    pub field: &'static str,
    /// Clé du trancheur.
    pub key: String,
    /// Valeur telle qu'écrite dans le fichier (abrégée).
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Missing {
    pub field: &'static str,
    pub fallback: Fallback,
}

/// Morceau de gabarit non traduit, laissé tel quel.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Untranslated {
    pub field: &'static str,
    /// Ligne dans le gabarit (à partir de 1).
    pub line: usize,
    pub text: String,
    /// Raison (variable inconnue, fonction, expression régulière…).
    pub reason: String,
}

/// Réglage du fichier remplacé par sa valeur dans un gabarit.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Constant {
    pub field: &'static str,
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Report {
    /// Trancheur d'origine (« PrusaSlicer 2.9.2 », « Bambu Studio / OrcaSlicer »).
    pub slicer: String,
    /// Nom du préréglage dans le trancheur.
    pub name: String,
    /// Préréglage parent introuvable dans le fichier : ses valeurs manquent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherits: Option<String>,
    /// Identifiant du profil FabWeave de base.
    pub base: String,
    pub taken: Vec<Taken>,
    pub missing: Vec<Missing>,
    /// Clés du fichier sans équivalent dans FabWeave.
    pub ignored: Vec<String>,
    /// Gabarits non vides du trancheur que FabWeave n'utilise pas
    /// (changement de couche, de filament…).
    pub ignored_gcode: Vec<String>,
    pub untranslated: Vec<Untranslated>,
    pub constants: Vec<Constant>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "profile", rename_all = "snake_case")]
pub enum ImportedProfile {
    Printer(Box<PrinterProfile>),
    Filament(Box<FilamentProfile>),
}

#[derive(Clone, Debug, Serialize)]
pub struct Imported {
    #[serde(flatten)]
    pub profile: ImportedProfile,
    pub report: Report,
}

/// Profils trouvés dans un fichier de trancheur. `printers` et `filaments`
/// sont les profils de base possibles (le premier sert à défaut).
pub fn import(
    source: &str,
    printers: &[PrinterProfile],
    filaments: &[FilamentProfile],
) -> Result<Vec<Imported>, Vec<Issue>> {
    let sets = if source.trim_start().starts_with('{') {
        json_sets(source)?
    } else {
        ini_sets(source)
    };
    let mut out = Vec::new();
    // Clés employées, mises en commun pour un export à plat (imprimante et
    // filament lus dans les mêmes réglages).
    let mut flat_used: Vec<String> = Vec::new();
    let mut built: Vec<(Imported, Vec<String>, &Set)> = Vec::new();
    for set in &sets {
        let (imported, used) = match set.kind {
            Kind::Printer => {
                let Some(base) = printers
                    .iter()
                    .find(|p| p.printer.firmware == firmware(&set.src).0)
                    .or(printers.first())
                else {
                    continue;
                };
                printer(set, base)
            }
            Kind::Filament => {
                let material = set.src.text("filament_type");
                let Some(base) = filaments
                    .iter()
                    .find(|f| {
                        material
                            .as_deref()
                            .is_some_and(|m| f.filament.material.eq_ignore_ascii_case(m))
                    })
                    .or(filaments.first())
                else {
                    continue;
                };
                filament(set, base)
            }
            Kind::Process => continue,
        };
        if set.flat {
            flat_used.extend(used.iter().cloned());
        }
        built.push((imported, used, set));
    }
    let has_flat_printer = built
        .iter()
        .any(|(i, _, set)| set.flat && matches!(i.profile, ImportedProfile::Printer(_)));
    for (mut imported, used, set) in built {
        let flat = set.flat;
        let used = if flat { &flat_used } else { &used };
        let (ignored, gcode) = unused(&set.src, used);
        let r = &mut imported.report;
        let is_filament = matches!(imported.profile, ImportedProfile::Filament(_));
        if flat && has_flat_printer {
            // À plat : la liste complète va à l'imprimante, le filament ne
            // garde que ses propres gabarits.
            let mine = |k: &String| k.contains("filament");
            r.ignored = if is_filament { Vec::new() } else { ignored };
            r.ignored_gcode = gcode
                .into_iter()
                .filter(|k| mine(k) == is_filament)
                .collect();
        } else {
            r.ignored = ignored;
            r.ignored_gcode = gcode;
        }
        out.push(imported);
    }
    if out.is_empty() {
        return Err(vec![Issue::error("import.no_profile")]);
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Lecture des fichiers

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Printer,
    Filament,
    /// Réglages d'impression (sans équivalent FabWeave).
    Process,
}

/// Réglages d'un préréglage : valeurs déjà déséchappées ; une liste par
/// extrudeur pour le JSON et les chaînes entre guillemets du `.ini`.
#[derive(Clone, Debug, Default)]
struct Source {
    map: IndexMap<String, Vec<String>>,
}

impl Source {
    fn raw(&self, key: &str) -> Option<&str> {
        self.map
            .get(key)
            .and_then(|v| v.first())
            .map(String::as_str)
    }

    /// Élément `i` d'une valeur par extrudeur (`0.4,0.4` ou liste JSON).
    fn element(&self, key: &str, i: usize) -> Option<String> {
        let v = self.map.get(key)?;
        if v.len() > 1 {
            return v.get(i).or(v.first()).cloned();
        }
        let s = v.first()?;
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() > 1 && parts.iter().all(|p| p.trim().parse::<f64>().is_ok()) {
            Some(parts.get(i).unwrap_or(&parts[0]).trim().to_string())
        } else {
            Some(s.clone())
        }
    }

    /// Texte non vide (ni `nil`).
    fn text(&self, key: &str) -> Option<String> {
        self.raw(key)
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "nil")
            .map(String::from)
    }

    fn number(&self, key: &str) -> Option<f64> {
        self.element(key, 0)?
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|v| v.is_finite())
    }
}

struct Set {
    kind: Kind,
    src: Source,
    slicer: String,
    inherits: Option<String>,
    /// Export à plat de PrusaSlicer (imprimante, filament et impression).
    flat: bool,
}

/// Déséchappement des valeurs du `.ini` (`\n`, `\r`, `\t`, `\\`, `\"`).
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some(o) => out.push(o),
            None => out.push('\\'),
        }
    }
    out
}

/// Valeur du `.ini` : chaînes entre guillemets séparées par `;` (une par
/// extrudeur), sinon la valeur entière.
fn ini_value(v: &str) -> Vec<String> {
    let v = v.trim();
    if !v.starts_with('"') {
        return vec![unescape(v)];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    let (mut quoted, mut escaped) = (false, false);
    for c in v.chars() {
        if escaped {
            cur.push('\\');
            cur.push(c);
            escaped = false;
        } else if quoted && c == '\\' {
            escaped = true;
        } else if c == '"' {
            quoted = !quoted;
        } else if !quoted && c == ';' {
            out.push(unescape(&std::mem::take(&mut cur)));
        } else if quoted {
            cur.push(c);
        }
    }
    out.push(unescape(&cur));
    out
}

fn ini_sets(source: &str) -> Vec<Set> {
    let mut slicer = "PrusaSlicer".to_string();
    let mut sections: IndexMap<String, Source> = IndexMap::new();
    let mut current = String::new();
    sections.insert(String::new(), Source::default());
    for line in source.lines() {
        let line = line.trim_end_matches('\r');
        let t = line.trim();
        if let Some(rest) = t.strip_prefix('#') {
            // « # generated by PrusaSlicer 2.9.2+MacOS-arm64 on … »
            if let Some(g) = rest.trim().strip_prefix("generated by ") {
                let name = g.split(" on ").next().unwrap_or(g);
                slicer = name.split('+').next().unwrap_or(name).trim().to_string();
            }
            continue;
        }
        if t.is_empty() || t.starts_with(';') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            current = t[1..t.len() - 1].to_string();
            sections.entry(current.clone()).or_default();
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        sections
            .entry(current.clone())
            .or_default()
            .map
            .insert(k.trim().to_string(), ini_value(v));
    }
    let mut out = Vec::new();
    // Export d'une configuration : tout à plat, imprimante et filament mêlés.
    let flat = &sections[""];
    if !flat.map.is_empty() {
        for kind in [Kind::Printer, Kind::Filament] {
            let present = match kind {
                Kind::Printer => {
                    flat.map.contains_key("bed_shape") || flat.map.contains_key("start_gcode")
                }
                _ => flat.map.contains_key("filament_type"),
            };
            if present {
                out.push(Set {
                    kind,
                    src: flat.clone(),
                    slicer: slicer.clone(),
                    inherits: None,
                    flat: true,
                });
            }
        }
    }
    // Paquet de préréglages : `[printer:Nom]`, `[filament:Nom]`, héritage
    // dans le fichier (`inherits = A; B`), préréglages abstraits `*nom*`.
    for (name, _) in sections.iter().filter(|(n, _)| !n.is_empty()) {
        let Some((kind, preset)) = name.split_once(':') else {
            continue;
        };
        let kind = match kind {
            "printer" => Kind::Printer,
            "filament" => Kind::Filament,
            _ => continue,
        };
        if preset.starts_with('*') {
            continue;
        }
        let prefix = name.split(':').next().unwrap_or_default();
        let mut missing = None;
        let mut src = resolve(&sections, prefix, preset, &mut missing, 0);
        src.map.shift_remove("inherits");
        // Le nom de la section fait foi : un parent commun peut déclarer
        // `filament_settings_id = ""` (PrusaResearch.ini), hérité par tous.
        src.map
            .insert(format!("{prefix}_settings_id"), vec![preset.to_string()]);
        out.push(Set {
            kind,
            src,
            slicer: slicer.clone(),
            inherits: missing,
            flat: false,
        });
    }
    out
}

/// Préréglage avec ses parents du même fichier, dans l'ordre d'héritage.
fn resolve(
    sections: &IndexMap<String, Source>,
    prefix: &str,
    preset: &str,
    missing: &mut Option<String>,
    depth: usize,
) -> Source {
    let Some(own) = sections.get(&format!("{prefix}:{preset}")) else {
        missing.get_or_insert_with(|| preset.to_string());
        return Source::default();
    };
    let mut out = Source::default();
    if depth < 16 {
        let parents = own.raw("inherits").unwrap_or_default().to_string();
        for p in parents.split(';').map(|p| p.trim().trim_matches('"')) {
            if !p.is_empty() {
                let s = resolve(sections, prefix, p, missing, depth + 1);
                out.map.extend(s.map);
            }
        }
    }
    out.map
        .extend(own.map.iter().map(|(k, v)| (k.clone(), v.clone())));
    out
}

fn json_sets(source: &str) -> Result<Vec<Set>, Vec<Issue>> {
    let v: Value = serde_json::from_str(source).map_err(|e| {
        vec![Issue::error("import.invalid_json")
            .at(Some((e.line(), e.column())))
            .detail(e.to_string())]
    })?;
    let Value::Object(obj) = v else {
        return Err(vec![Issue::error("import.no_profile")]);
    };
    let mut src = Source::default();
    for (k, v) in &obj {
        let list = match v {
            Value::String(s) => vec![s.clone()],
            Value::Number(n) => vec![n.to_string()],
            Value::Bool(b) => vec![u8::from(*b).to_string()],
            Value::Array(a) => a
                .iter()
                .filter_map(|x| match x {
                    Value::String(s) => Some(s.clone()),
                    Value::Number(n) => Some(n.to_string()),
                    _ => None,
                })
                .collect(),
            _ => continue,
        };
        src.map.insert(k.clone(), list);
    }
    let kind = match src.raw("type") {
        Some("machine") => Kind::Printer,
        Some("filament") => Kind::Filament,
        Some("process") => Kind::Process,
        _ if src.map.contains_key("printable_area") => Kind::Printer,
        _ if src.map.contains_key("filament_type") => Kind::Filament,
        _ => return Err(vec![Issue::error("import.no_profile")]),
    };
    if kind == Kind::Process {
        return Err(vec![Issue::error("import.process_profile")]);
    }
    let inherits = src.text("inherits");
    Ok(vec![Set {
        kind,
        src,
        slicer: "Bambu Studio / OrcaSlicer".into(),
        inherits,
        flat: false,
    }])
}

// ---------------------------------------------------------------------------
// Construction des profils

/// Clés d'identification, jamais listées comme ignorées.
const META_KEYS: &[&str] = &[
    "type",
    "name",
    "from",
    "inherits",
    "instantiation",
    "setting_id",
    "version",
    "is_custom_defined",
    "printer_settings_id",
    "filament_settings_id",
    "print_settings_id",
];

struct Builder<'a> {
    src: &'a Source,
    used: Vec<String>,
    report: Report,
}

impl<'a> Builder<'a> {
    fn new(set: &'a Set, base: String) -> Self {
        Self {
            src: &set.src,
            used: META_KEYS.iter().map(|k| k.to_string()).collect(),
            report: Report {
                slicer: set.slicer.clone(),
                inherits: set.inherits.clone(),
                base,
                ..Report::default()
            },
        }
    }

    fn take(&mut self, field: &'static str, key: &str, value: String) {
        let value = if value.chars().count() > 48 {
            format!("{}…", value.chars().take(47).collect::<String>())
        } else {
            value
        };
        self.report.taken.push(Taken {
            field,
            key: key.into(),
            value,
        });
    }

    fn miss(&mut self, field: &'static str, fallback: Fallback) {
        self.report.missing.push(Missing { field, fallback });
    }

    fn mark(&mut self, key: &str) {
        if self.src.map.contains_key(key) && !self.used.iter().any(|k| k == key) {
            self.used.push(key.into());
        }
    }

    /// Premier nombre trouvé parmi `keys` (zéro refusé si `nonzero` : le
    /// trancheur l'emploie pour « non réglé »).
    fn number(&mut self, field: &'static str, keys: &[&str], nonzero: bool) -> Option<f64> {
        for k in keys {
            self.mark(k);
        }
        for k in keys {
            if let Some(v) = self.src.number(k).filter(|v| !nonzero || *v != 0.0) {
                let raw = self.src.element(k, 0).unwrap_or_default();
                self.take(field, k, raw);
                return Some(v);
            }
        }
        None
    }

    fn set(&mut self, field: &'static str, keys: &[&str], nonzero: bool, target: &mut f64) {
        match self.number(field, keys, nonzero) {
            Some(v) => *target = v,
            None => self.miss(field, Fallback::Base),
        }
    }

    fn set_opt(
        &mut self,
        field: &'static str,
        keys: &[&str],
        target: &mut Option<f64>,
        fallback: Fallback,
    ) {
        match self.number(field, keys, true) {
            Some(v) => *target = Some(v),
            None => {
                if fallback == Fallback::Empty {
                    *target = None;
                }
                self.miss(field, fallback);
            }
        }
    }

    fn text(&mut self, field: &'static str, keys: &[&str]) -> Option<String> {
        for k in keys {
            self.mark(k);
        }
        for k in keys {
            if let Some(v) = self.src.text(k) {
                self.take(field, k, v.clone());
                return Some(v);
            }
        }
        None
    }

    fn flag(&mut self, field: &'static str, keys: &[&str]) -> Option<bool> {
        for k in keys {
            self.mark(k);
        }
        for k in keys {
            if let Some(v) = self.src.element(k, 0) {
                let b = matches!(v.trim(), "1" | "true");
                self.take(field, k, v);
                return Some(b);
            }
        }
        None
    }

    fn finish(self) -> (Report, Vec<String>) {
        (self.report, self.used)
    }
}

/// Clés sans emploi, et parmi elles les gabarits non vides.
fn unused(src: &Source, used: &[String]) -> (Vec<String>, Vec<String>) {
    let mut keys = Vec::new();
    let mut gcode = Vec::new();
    for (k, v) in &src.map {
        if used.iter().any(|u| u == k) {
            continue;
        }
        keys.push(k.clone());
        // `binary_gcode = 1` est un réglage, pas un gabarit.
        let text =
            |s: &String| !s.trim().is_empty() && s != "nil" && s.trim().parse::<f64>().is_err();
        if k.ends_with("_gcode") && v.iter().any(text) {
            gcode.push(k.clone());
        }
    }
    (keys, gcode)
}

/// Identifiant d'après un nom : minuscules ASCII et `_`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c.to_ascii_lowercase(),
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "profil".into()
    } else {
        out
    }
}

/// Firmware d'après le fichier, et la clé qui l'a donné.
fn firmware(src: &Source) -> (Firmware, Option<&'static str>) {
    let model = src.text("printer_model").unwrap_or_default();
    let named = |s: &str| {
        ["printer_settings_id", "name", "inherits", "printer_model"]
            .iter()
            .any(|k| src.text(k).is_some_and(|v| v.contains(s)))
    };
    if model.starts_with("Bambu Lab") || named("Bambu Lab") || named("@BBL") {
        return (Firmware::Bambu, Some("printer_model"));
    }
    let flavor = src.text("gcode_flavor").unwrap_or_default();
    if flavor == "klipper" {
        return (Firmware::Klipper, Some("gcode_flavor"));
    }
    let notes = src.text("printer_notes").unwrap_or_default();
    if notes.contains("PRINTER_VENDOR_PRUSA3D") {
        return (Firmware::Prusa, Some("printer_notes"));
    }
    // Firmware Buddy des Prusa récentes (MK4S, CORE One, XL, MINI…).
    const BUDDY: &[&str] = &["MK4", "MK3.9", "MK3.5", "COREONE", "XL", "MINI"];
    if flavor == "marlin2" && BUDDY.iter().any(|m| model.starts_with(m)) {
        return (Firmware::Prusa, Some("printer_model"));
    }
    let key = (!flavor.is_empty()).then_some("gcode_flavor");
    (Firmware::Marlin, key)
}

/// Points `XxY` d'un contour (`0x0,250x0,…` ou liste JSON).
fn points(src: &Source, key: &str) -> Vec<[f64; 2]> {
    let Some(v) = src.map.get(key) else {
        return Vec::new();
    };
    v.iter()
        .flat_map(|s| s.split(','))
        .filter_map(|p| {
            let (x, y) = p.trim().split_once('x')?;
            Some([x.trim().parse().ok()?, y.trim().parse().ok()?])
        })
        .collect()
}

fn bounds(pts: &[[f64; 2]]) -> ([f64; 2], [f64; 2]) {
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (lo, hi)
}

fn printer(set: &Set, base: &PrinterProfile) -> (Imported, Vec<String>) {
    let mut b = Builder::new(set, base.printer.id.clone());
    let mut p = base.clone();
    p.format = Some(PRINTER_FORMAT.into());
    p.version = Some(PRINTER_VERSION);
    let name = b
        .text("printer.name", &["printer_settings_id", "name"])
        .unwrap_or_else(|| "Imprimante importée".into());
    b.report.name = name.clone();
    p.printer.id = slug(&name);
    p.printer.name = Text::Plain(name.clone());
    p.printer.source = Some(format!("{} : {name}", set.slicer));
    match b.text("printer.model", &["printer_model"]) {
        Some(m) => p.printer.model = m,
        None => {
            p.printer.model.clear();
            b.miss("printer.model", Fallback::Empty);
        }
    }
    let (fw, key) = firmware(&set.src);
    for k in ["gcode_flavor", "printer_notes"] {
        b.mark(k);
    }
    p.printer.firmware = fw;
    match key {
        Some(k) => {
            let v = set.src.text(k).unwrap_or_default();
            b.take("printer.firmware", k, v);
        }
        None => b.miss("printer.firmware", Fallback::Estimated),
    }

    // Plateau : rectangle (origine au coin, ou au centre s'il est centré
    // sur zéro), ou cercle s'il a au moins 8 sommets à égale distance.
    b.mark("bed_shape");
    b.mark("printable_area");
    let key = ["bed_shape", "printable_area"]
        .into_iter()
        .find(|k| set.src.map.contains_key(*k));
    let pts = key.map(|k| points(&set.src, k)).unwrap_or_default();
    if pts.len() >= 3 {
        let (lo, hi) = bounds(&pts);
        let c = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let r: Vec<f64> = pts
            .iter()
            .map(|q| (q[0] - c[0]).hypot(q[1] - c[1]))
            .collect();
        let round = pts.len() >= 8 && r.iter().all(|x| (x - r[0]).abs() < 0.01 * r[0]);
        let centred = c[0].abs() < 1.0 && c[1].abs() < 1.0;
        p.bed.shape = if round {
            BedShape::Circle
        } else {
            BedShape::Rectangle
        };
        if centred {
            p.bed.origin = Origin::Center;
            p.bed.size = [hi[0] - lo[0], hi[1] - lo[1]];
        } else {
            // Marge négative (ligne de purge hors plateau) : ignorée.
            p.bed.origin = Origin::Corner;
            p.bed.size = [hi[0], hi[1]];
        }
        if round {
            let d = 2.0 * r[0];
            p.bed.size = [d, d];
        }
        let k = key.unwrap_or_default();
        let v = set.src.raw(k).unwrap_or_default().to_string();
        b.take("bed.size", k, v);
    } else {
        b.miss("bed.size", Fallback::Base);
    }
    b.set(
        "bed.max_height",
        &["max_print_height", "printable_height"],
        true,
        &mut p.bed.max_height,
    );
    if fw != Firmware::Bambu {
        p.bed.plate.clear();
    }
    // Zones exclues (Bambu Studio, OrcaSlicer) : rectangles de 4 sommets.
    b.mark("bed_exclude_area");
    let ex = points(&set.src, "bed_exclude_area");
    p.bed.exclude = if !ex.is_empty() && ex.len().is_multiple_of(4) {
        ex.chunks(4)
            .map(bounds)
            .filter(|(lo, hi)| hi[0] > lo[0] && hi[1] > lo[1])
            .enumerate()
            .map(|(i, (min, max))| Zone {
                name: Text::Plain(format!("Zone {}", i + 1)),
                min,
                max,
            })
            .collect()
    } else if !ex.is_empty() {
        let (min, max) = bounds(&ex);
        vec![Zone {
            name: Text::Plain("Zone 1".into()),
            min,
            max,
        }]
    } else {
        Vec::new()
    };
    if p.bed.exclude.is_empty() {
        b.miss("bed.exclude", Fallback::Empty);
    } else {
        let v = set
            .src
            .raw("bed_exclude_area")
            .unwrap_or_default()
            .to_string();
        b.take("bed.exclude", "bed_exclude_area", v);
    }

    // Buse.
    b.set(
        "nozzle.diameter",
        &["nozzle_diameter"],
        true,
        &mut p.nozzle.diameter,
    );
    p.nozzle.high_flow = b
        .flag("nozzle.high_flow", &["nozzle_high_flow"])
        .unwrap_or(false);
    b.mark("nozzle_type");
    p.nozzle.abrasive = match set.src.text("nozzle_type") {
        Some(t) => {
            b.take("nozzle.abrasive", "nozzle_type", t.clone());
            t.contains("hardened") || t.contains("tungsten")
        }
        None => false,
    };
    p.nozzle.cone_angle = None;
    p.nozzle.cone_height = None;
    p.nozzle.choices.clear();
    p.nozzle.high_flow_available = false;
    p.nozzle.abrasive_available = false;
    b.set_opt(
        "nozzle.clearance_radius",
        &["extruder_clearance_radius"],
        &mut p.nozzle.clearance_radius,
        Fallback::Empty,
    );
    b.set_opt(
        "nozzle.clearance_height",
        &[
            "extruder_clearance_height",
            "extruder_clearance_height_to_rod",
        ],
        &mut p.nozzle.clearance_height,
        Fallback::Empty,
    );

    // Cinématique : vitesses maximales de la machine (mode normal, la
    // première valeur), accélérations des réglages d'impression s'ils sont
    // dans le fichier, sinon les maximales.
    let x = b.number(
        "limits.max_print_speed",
        &["machine_max_feedrate_x", "machine_max_speed_x"],
        true,
    );
    let y = set
        .src
        .number("machine_max_feedrate_y")
        .or(set.src.number("machine_max_speed_y"));
    b.mark("machine_max_feedrate_y");
    b.mark("machine_max_speed_y");
    match x {
        Some(x) => p.limits.max_print_speed = y.map_or(x, |y| x.min(y)),
        None => b.miss("limits.max_print_speed", Fallback::Base),
    }
    b.set(
        "limits.travel_speed",
        &["travel_speed"],
        true,
        &mut p.limits.travel_speed,
    );
    b.set(
        "limits.max_z_speed",
        &["machine_max_feedrate_z", "machine_max_speed_z"],
        true,
        &mut p.limits.max_z_speed,
    );
    b.set_opt(
        "limits.acceleration",
        &[
            "external_perimeter_acceleration",
            "outer_wall_acceleration",
            "default_acceleration",
            "machine_max_acceleration_extruding",
        ],
        &mut p.limits.acceleration,
        Fallback::Base,
    );
    b.set_opt(
        "limits.travel_acceleration",
        &["travel_acceleration", "machine_max_acceleration_travel"],
        &mut p.limits.travel_acceleration,
        Fallback::Base,
    );
    b.set_opt(
        "limits.first_layer_acceleration",
        &["first_layer_acceleration", "initial_layer_acceleration"],
        &mut p.limits.first_layer_acceleration,
        Fallback::Base,
    );
    b.set_opt(
        "limits.jerk",
        &["machine_max_jerk_x"],
        &mut p.limits.jerk,
        Fallback::Base,
    );
    if p.limits.acceleration.is_none() {
        p.limits.travel_acceleration = None;
        p.limits.first_layer_acceleration = None;
    }

    // Couches : réglages d'impression si présents ; sinon celles de la base
    // pour la même buse, ou la moitié de la buse.
    p.layers.by_nozzle.clear();
    let same = same_diameter(p.nozzle.diameter, base.nozzle.diameter);
    match b.number("layers.layer_height", &["layer_height"], true) {
        Some(v) => p.layers.layer_height = v,
        None if same => b.miss("layers.layer_height", Fallback::Base),
        None => {
            p.layers.layer_height = (p.nozzle.diameter * 50.0).round() / 100.0;
            b.miss("layers.layer_height", Fallback::Estimated);
        }
    }
    b.mark("first_layer_height");
    b.mark("initial_layer_print_height");
    let first = ["first_layer_height", "initial_layer_print_height"]
        .into_iter()
        .find_map(|k| set.src.text(k).map(|v| (k, v)));
    match first {
        Some((k, v)) => {
            let n = match v.strip_suffix('%') {
                Some(pc) => pc
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .map(|x| x / 100.0 * p.layers.layer_height),
                None => v.parse::<f64>().ok(),
            };
            match n {
                Some(n) => {
                    p.layers.first_layer_height = n;
                    b.take("layers.first_layer_height", k, v);
                }
                None => b.miss("layers.first_layer_height", Fallback::Base),
            }
        }
        None if same => b.miss("layers.first_layer_height", Fallback::Base),
        None => {
            p.layers.first_layer_height = p.layers.layer_height;
            b.miss("layers.first_layer_height", Fallback::Estimated);
        }
    }

    // Vignettes : `16x16/QOI, 313x173/QOI` (PrusaSlicer 2.6+), ou tailles
    // seules et `thumbnails_format`.
    b.mark("thumbnails");
    b.mark("thumbnails_format");
    let format = set
        .src
        .text("thumbnails_format")
        .unwrap_or_else(|| "PNG".into())
        .to_uppercase();
    let thumbs: Vec<String> = set
        .src
        .map
        .get("thumbnails")
        .into_iter()
        .flatten()
        .flat_map(|s| s.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.contains('/') {
                s.to_string()
            } else {
                format!("{s}/{format}")
            }
        })
        .collect();
    if thumbs.is_empty() {
        p.gcode.thumbnails.clear();
        b.miss("gcode.thumbnails", Fallback::Empty);
    } else {
        let v = thumbs.join(", ");
        b.take("gcode.thumbnails", "thumbnails", v);
        p.gcode.thumbnails = thumbs;
    }

    // Gabarits, traduits puis marqués à relire.
    for (field, keys) in [
        ("gcode.start", ["start_gcode", "machine_start_gcode"]),
        ("gcode.end", ["end_gcode", "machine_end_gcode"]),
        ("gcode.pause", ["pause_print_gcode", "machine_pause_gcode"]),
    ] {
        for k in keys {
            b.mark(k);
        }
        let found = keys.iter().find_map(|k| {
            set.src
                .raw(k)
                .filter(|s| !s.trim().is_empty())
                .map(|s| (*k, s))
        });
        match found {
            Some((k, text)) => {
                let t = translate(text, &set.src, field);
                b.take(field, k, format!("{} lignes", text.lines().count()));
                b.report.untranslated.extend(t.untranslated);
                b.report.constants.extend(t.constants);
                match field {
                    "gcode.start" => p.gcode.start = t.text,
                    "gcode.end" => p.gcode.end = t.text,
                    _ => p.gcode.pause = t.text,
                }
            }
            None => b.miss(field, Fallback::Base),
        }
    }
    p.printer.review = REVIEW_FIELDS.iter().map(|s| s.to_string()).collect();

    let (report, used) = b.finish();
    let imported = Imported {
        profile: ImportedProfile::Printer(Box::new(p)),
        report,
    };
    (imported, used)
}

fn filament(set: &Set, base: &FilamentProfile) -> (Imported, Vec<String>) {
    let mut b = Builder::new(set, base.filament.id.clone());
    let mut f = base.clone();
    f.format = Some(FILAMENT_FORMAT.into());
    f.version = Some(FILAMENT_VERSION);
    let name = b
        .text("filament.name", &["filament_settings_id", "name"])
        .unwrap_or_else(|| "Filament importé".into());
    b.report.name = name.clone();
    f.filament.id = slug(&name);
    f.filament.name = Text::Plain(name);
    match b.text("filament.type", &["filament_type"]) {
        Some(t) => f.filament.material = t,
        None => b.miss("filament.type", Fallback::Base),
    }
    b.set(
        "filament.diameter",
        &["filament_diameter"],
        true,
        &mut f.filament.diameter,
    );
    b.set(
        "filament.density",
        &["filament_density"],
        true,
        &mut f.filament.density,
    );
    b.set(
        "filament.flow_multiplier",
        &["extrusion_multiplier", "filament_flow_ratio"],
        true,
        &mut f.filament.flow_multiplier,
    );
    // Avance de pression : réglage de Bambu Studio / OrcaSlicer s'il est
    // activé, sinon `M572 S…` (ou `M900 K…`) du gabarit de filament de
    // PrusaSlicer, valeur de la buse 0,4 mm.
    // Le gabarit de filament lui-même n'est pas repris (listé ignoré).
    b.mark("enable_pressure_advance");
    let enabled = set.src.element("enable_pressure_advance", 0).as_deref() == Some("1");
    let pa = if enabled {
        b.number("filament.pressure_advance", &["pressure_advance"], false)
    } else {
        b.mark("pressure_advance");
        None
    };
    let pa = pa.or_else(|| {
        let g = set.src.raw("start_filament_gcode")?;
        let (cmd, v) = gcode_pressure_advance(g)?;
        b.take(
            "filament.pressure_advance",
            &format!("start_filament_gcode ({cmd})"),
            num(v),
        );
        Some(v)
    });
    match pa {
        Some(v) => f.filament.pressure_advance = v,
        None => b.miss("filament.pressure_advance", Fallback::Base),
    }
    f.filament.abrasive = match b.flag("filament.abrasive", &["filament_abrasive"]) {
        Some(a) => a,
        None => match b.number("filament.abrasive", &["required_nozzle_HRC"], false) {
            // Bambu Lab : dureté de buse exigée (acier trempé : 40 et plus).
            Some(hrc) => hrc >= 40.0,
            None => {
                b.miss("filament.abrasive", Fallback::Empty);
                false
            }
        },
    };

    let t = &mut f.temperature;
    b.set(
        "temperature.first_layer",
        &[
            "first_layer_temperature",
            "nozzle_temperature_initial_layer",
        ],
        true,
        &mut t.first_layer,
    );
    b.set(
        "temperature.nozzle",
        &["temperature", "nozzle_temperature"],
        true,
        &mut t.nozzle,
    );
    b.set(
        "temperature.bed_first_layer",
        &[
            "first_layer_bed_temperature",
            "textured_plate_temp_initial_layer",
            "hot_plate_temp_initial_layer",
        ],
        false,
        &mut t.bed_first_layer,
    );
    b.set(
        "temperature.bed",
        &["bed_temperature", "textured_plate_temp", "hot_plate_temp"],
        false,
        &mut t.bed,
    );
    b.miss("temperature.probe", Fallback::Base);
    b.set_opt(
        "temperature.idle",
        &["idle_temperature"],
        &mut t.idle,
        Fallback::Base,
    );
    b.set(
        "temperature.chamber",
        &["chamber_temperature", "chamber_temperatures"],
        true,
        &mut t.chamber,
    );
    b.set(
        "temperature.chamber_min",
        &["chamber_minimal_temperature"],
        true,
        &mut t.chamber_min,
    );

    b.miss("speed.nominal", Fallback::Base);
    b.miss("speed.first_layer", Fallback::Base);
    b.set(
        "speed.max_volumetric",
        &["filament_max_volumetric_speed"],
        true,
        &mut f.speed.max_volumetric,
    );
    f.speed.by_nozzle.clear();
    b.miss("speed.by_nozzle", Fallback::Empty);

    // Ventilateur : vitesse maximale (couches courtes, le cas des parois
    // uniques), coupé sur la première couche si le trancheur le coupe.
    b.set(
        "cooling.fan",
        &["max_fan_speed", "fan_max_speed"],
        false,
        &mut f.cooling.fan,
    );
    match b.number(
        "cooling.first_layer_fan",
        &["disable_fan_first_layers", "close_fan_the_first_x_layers"],
        false,
    ) {
        Some(n) => f.cooling.first_layer_fan = if n >= 1.0 { 0.0 } else { f.cooling.fan },
        None => b.miss("cooling.first_layer_fan", Fallback::Base),
    }
    b.set(
        "cooling.bridge_fan",
        &["bridge_fan_speed", "overhang_fan_speed"],
        false,
        &mut f.cooling.bridge_fan,
    );

    // Rétraction : celle du filament si elle est réglée, sinon celle de
    // l'imprimante (export à plat de PrusaSlicer).
    let r = &mut f.retraction;
    b.set(
        "retraction.length",
        &[
            "filament_retract_length",
            "filament_retraction_length",
            "retract_length",
            "retraction_length",
        ],
        false,
        &mut r.length,
    );
    b.set(
        "retraction.speed",
        &[
            "filament_retract_speed",
            "filament_retraction_speed",
            "retract_speed",
            "retraction_speed",
        ],
        true,
        &mut r.speed,
    );
    b.set(
        "retraction.z_hop",
        &[
            "filament_retract_lift",
            "filament_z_hop",
            "retract_lift",
            "z_hop",
        ],
        false,
        &mut r.z_hop,
    );
    b.set(
        "retraction.min_travel",
        &[
            "filament_retract_before_travel",
            "filament_retraction_minimum_travel",
            "retract_before_travel",
            "retraction_minimum_travel",
        ],
        false,
        &mut r.min_travel,
    );

    let (report, used) = b.finish();
    let imported = Imported {
        profile: ImportedProfile::Filament(Box::new(f)),
        report,
    };
    (imported, used)
}

fn num(v: f64) -> String {
    crate::format::num(v, 4)
}

/// Avance de pression d'un gabarit de filament : `M572 S0.036`, ou la
/// branche de la buse 0,4 mm de `M572 S{if nozzle_diameter[0]==0.4}0.036…`.
fn gcode_pressure_advance(g: &str) -> Option<(&'static str, f64)> {
    for (cmd, pat) in [("M572", "M572 S"), ("M900", "M900 K")] {
        for (i, _) in g.match_indices(pat) {
            let rest = &g[i + pat.len()..];
            let lead: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(v) = lead.parse::<f64>() {
                return Some((cmd, v));
            }
            if let Some(p) = rest.find("==0.4}") {
                let after = &rest[p + 6..];
                let v: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(v) = v.parse::<f64>() {
                    return Some((cmd, v));
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Traduction des gabarits

struct Translation {
    text: String,
    untranslated: Vec<Untranslated>,
    constants: Vec<Constant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ty {
    Num,
    Str,
    Bool,
}

#[derive(Debug)]
enum Node {
    Text(String),
    /// `{expr}` ou `[var]` : texte entre délimiteurs, ligne, texte d'origine.
    Expr {
        inner: String,
        line: usize,
        raw: String,
    },
    If {
        branches: Vec<(Option<String>, Vec<Node>)>,
        line: usize,
        raw: String,
    },
    /// Bloc `{if}` sans `{endif}` : gardé tel quel et signalé.
    Unclosed {
        line: usize,
        raw: String,
    },
}

/// Découpe : texte, `{…}` (hors chaînes), `[identifiant]`.
fn tokens(src: &str) -> Vec<(String, bool, usize)> {
    let mut out = Vec::new();
    let mut text = String::new();
    let chars: Vec<char> = src.chars().collect();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '{' {
            let mut j = i + 1;
            let mut quoted = false;
            while j < chars.len() && (quoted || chars[j] != '}') {
                if chars[j] == '"' {
                    quoted = !quoted;
                }
                j += 1;
            }
            if j < chars.len() {
                if !text.is_empty() {
                    out.push((std::mem::take(&mut text), false, line));
                }
                let raw: String = chars[i..=j].iter().collect();
                let start = line;
                line += raw.matches('\n').count();
                out.push((raw, true, start));
                i = j + 1;
                continue;
            }
        } else if c == '[' {
            let end = chars[i + 1..]
                .iter()
                .position(|c| *c == ']' || *c == '\n' || *c == ' ')
                .map(|p| i + 1 + p);
            // `[nom]`, `[nom_0]`, `[nom[0]]`
            let end = end.map(|e| {
                if chars[e] == ']' && chars[i + 1..e].contains(&'[') {
                    e + 1
                } else {
                    e
                }
            });
            if let Some(e) = end.filter(|e| *e < chars.len() && chars[*e] == ']') {
                let inner: String = chars[i + 1..e].iter().collect();
                let ident = inner.split('[').next().unwrap_or_default();
                if !ident.is_empty()
                    && ident
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphabetic())
                    && ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                {
                    if !text.is_empty() {
                        out.push((std::mem::take(&mut text), false, line));
                    }
                    out.push((chars[i..=e].iter().collect(), true, line));
                    i = e + 1;
                    continue;
                }
            }
        }
        if c == '\n' {
            line += 1;
        }
        text.push(c);
        i += 1;
    }
    if !text.is_empty() {
        out.push((text, false, line));
    }
    out
}

/// Arbre des blocs `{if}…{endif}` ; un bloc mal fermé reste en texte brut.
fn tree(src: &str) -> Vec<Node> {
    enum Frame {
        Root(Vec<Node>),
        If {
            branches: Vec<(Option<String>, Vec<Node>)>,
            line: usize,
            raw: String,
        },
    }
    fn push(stack: &mut [Frame], n: Node) {
        match stack.last_mut() {
            Some(Frame::Root(v)) => v.push(n),
            Some(Frame::If { branches, .. }) => {
                branches.last_mut().expect("branche").1.push(n);
            }
            None => {}
        }
    }
    fn raw_of(stack: &mut [Frame], s: &str) {
        for f in stack.iter_mut() {
            if let Frame::If { raw, .. } = f {
                raw.push_str(s);
            }
        }
    }
    let mut stack = vec![Frame::Root(Vec::new())];
    for (s, is_macro, line) in tokens(src) {
        raw_of(&mut stack, &s);
        if !is_macro {
            push(&mut stack, Node::Text(s));
            continue;
        }
        let legacy = s.starts_with('[');
        let inner = s[1..s.len() - 1].trim().to_string();
        let word = inner
            .split(|c: char| !c.is_ascii_alphabetic())
            .next()
            .unwrap_or("");
        match (legacy, word) {
            (false, "if") => stack.push(Frame::If {
                branches: vec![(Some(inner[2..].trim().to_string()), Vec::new())],
                line,
                raw: s.clone(),
            }),
            (false, "elsif") | (false, "else") | (false, "endif")
                if matches!(stack.last(), Some(Frame::If { .. })) =>
            {
                if word == "endif" {
                    let Some(Frame::If {
                        branches,
                        line,
                        raw,
                    }) = stack.pop()
                    else {
                        unreachable!()
                    };
                    push(
                        &mut stack,
                        Node::If {
                            branches,
                            line,
                            raw,
                        },
                    );
                } else if let Some(Frame::If { branches, .. }) = stack.last_mut() {
                    let cond = (word == "elsif").then(|| inner[5..].trim().to_string());
                    branches.push((cond, Vec::new()));
                }
            }
            _ => push(
                &mut stack,
                Node::Expr {
                    inner,
                    line,
                    raw: s,
                },
            ),
        }
    }
    // Blocs non fermés : texte d'origine (déjà repris par les blocs
    // englobants, seul le plus extérieur est gardé).
    while stack.len() > 1 {
        if let Some(Frame::If { raw, line, .. }) = stack.pop() {
            if stack.len() == 1 {
                push(&mut stack, Node::Unclosed { line, raw });
            }
        }
    }
    match stack.pop() {
        Some(Frame::Root(v)) => v,
        _ => Vec::new(),
    }
}

struct Translator<'a> {
    src: &'a Source,
    field: &'static str,
    untranslated: Vec<Untranslated>,
    constants: Vec<Constant>,
}

/// Variables FabWeave des gabarits, par nom du trancheur.
fn known(name: &str) -> Option<(&'static str, Ty)> {
    Some(match name {
        "first_layer_temperature" | "nozzle_temperature_initial_layer" => {
            ("first_layer_temperature", Ty::Num)
        }
        "temperature" | "nozzle_temperature" => ("temperature", Ty::Num),
        "first_layer_bed_temperature"
        | "bed_temperature_initial_layer"
        | "bed_temperature_initial_layer_single"
        | "bed_temperature_initial_layer_vector" => ("first_layer_bed_temperature", Ty::Num),
        "bed_temperature" => ("bed_temperature", Ty::Num),
        "chamber_temperature" | "overall_chamber_temperature" | "chamber_temperatures" => {
            ("chamber_temperature", Ty::Num)
        }
        "chamber_minimal_temperature" => ("chamber_minimal_temperature", Ty::Num),
        "idle_temperature" => ("idle_temperature", Ty::Num),
        "nozzle_diameter" => ("nozzle_diameter", Ty::Num),
        "max_print_height" | "printable_height" => ("max_print_height", Ty::Num),
        "max_layer_z" => ("max_z", Ty::Num),
        "retract_length" | "retraction_length" => ("retraction_length", Ty::Num),
        "retract_speed" | "retraction_speed" => ("retraction_speed", Ty::Num),
        // Débits des lignes de purge et de calibration des Bambu Lab (réglages
        // d'impression et de purge, absents d'un profil machine) : débit
        // maximal du filament, comme dans le profil X1E livré.
        "filament_max_volumetric_speed"
        | "outer_wall_volumetric_speed"
        | "flush_volumetric_speeds" => ("max_volumetric_speed", Ty::Num),
        // Température de purge : celle du filament.
        "flush_temperatures" => ("temperature", Ty::Num),
        "total_layer_count" => ("total_layer_count", Ty::Num),
        "pressure_advance" => ("pressure_advance", Ty::Num),
        "printer_model" => ("printer_model", Ty::Str),
        "filament_type" => ("filament_type", Ty::Str),
        "curr_bed_type" => ("bed_type", Ty::Str),
        "printer_preset" => ("printer_name", Ty::Str),
        "filament_preset" => ("filament_name", Ty::Str),
        "input_filename_base" => ("model_name", Ty::Str),
        "nozzle_high_flow" => ("nozzle_high_flow", Ty::Bool),
        "filament_abrasive" => ("filament_abrasive", Ty::Bool),
        // Une seule tête : l'extrudeur 0, toujours employé.
        "initial_tool"
        | "initial_extruder"
        | "current_extruder"
        | "filament_extruder_id"
        | "initial_no_support_extruder" => ("0.0", Ty::Num),
        "is_extruder_used" => ("true", Ty::Bool),
        "has_wipe_tower" => ("false", Ty::Bool),
        // FabWeave n'emploie pas le mode vase du trancheur (marqueurs de
        // couche toujours écrits) ; couche après couche pour l'appareil photo.
        "spiral_mode" => ("false", Ty::Bool),
        "print_sequence" => ("\"by layer\"", Ty::Str),
        _ => return None,
    })
}

/// Variables indexées par axe (`first_layer_print_min[0]`…).
fn axis(name: &str, i: usize) -> Option<String> {
    let xy = ["x", "y"].get(i)?;
    Some(match name {
        "first_layer_print_min" => format!("first_layer_min_{xy}"),
        "first_layer_print_max" => format!("first_layer_max_{xy}"),
        "first_layer_print_size" => format!("(first_layer_max_{xy} - first_layer_min_{xy})"),
        "first_layer_center_no_wipe_tower" => {
            format!("((first_layer_min_{xy} + first_layer_max_{xy}) / 2.0)")
        }
        "print_bed_max" | "print_bed_size" => format!("bed_size_{xy}"),
        "print_bed_min" => "0.0".into(),
        _ => return None,
    })
}

fn float_literal(s: &str) -> String {
    if s.contains(['.', 'e', 'E']) {
        if s.starts_with('.') {
            format!("0{s}")
        } else {
            s.to_string()
        }
    } else {
        format!("{s}.0")
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(String),
    Str(String),
    Ident(String),
    Regex(String),
    Op(&'static str),
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let c: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    const OPS: &[&str] = &[
        "==", "!=", "<>", "<=", ">=", "=~", "!~", "&&", "||", "<", ">", "+", "-", "*", "/", "%",
        "(", ")", "[", "]", ",", "?", ":", "!",
    ];
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch.is_ascii_digit()
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let start = i;
            while i < c.len() && (c[i].is_ascii_digit() || c[i] == '.') {
                i += 1;
            }
            out.push(Tok::Num(c[start..i].iter().collect()));
        } else if ch == '"' {
            let start = i + 1;
            i += 1;
            while i < c.len() && c[i] != '"' {
                i += 1;
            }
            out.push(Tok::Str(c[start..i.min(c.len())].iter().collect()));
            i += 1;
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let start = i;
            while i < c.len() && (c[i].is_ascii_alphanumeric() || c[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(c[start..i].iter().collect()));
        } else if ch == '/' && matches!(out.last(), Some(Tok::Op("=~" | "!~"))) {
            let start = i + 1;
            i += 1;
            while i < c.len() && c[i] != '/' {
                if c[i] == '\\' {
                    i += 1;
                }
                i += 1;
            }
            out.push(Tok::Regex(c[start..i.min(c.len())].iter().collect()));
            i += 1;
        } else {
            let rest: String = c[i..c.len().min(i + 2)].iter().collect();
            let op = OPS
                .iter()
                .find(|o| rest.starts_with(**o))
                .ok_or_else(|| format!("« {ch} »"))?;
            out.push(Tok::Op(op));
            i += op.len();
        }
    }
    Ok(out)
}

/// Code Rhai et type d'une expression traduite.
type Code = (String, Ty);

struct Parser<'t, 'a> {
    toks: Vec<Tok>,
    pos: usize,
    tr: &'t mut Translator<'a>,
}

impl Parser<'_, '_> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn eat(&mut self, op: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Op(o)) if *o == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn eat_word(&mut self, w: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Ident(i)) if i == w) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, op: &str) -> Result<(), String> {
        if self.eat(op) {
            Ok(())
        } else {
            Err(format!("« {op} » attendu"))
        }
    }

    fn ternary(&mut self) -> Result<Code, String> {
        let c = self.or()?;
        if self.eat("?") {
            let a = self.ternary()?;
            self.expect(":")?;
            let b = self.ternary()?;
            // `a ? b : c ? d : e` en une seule chaîne `if … else if …`
            // (moins de profondeur pour Rhai).
            let b_code = unwrap(&b.0);
            let tail = if b_code.starts_with("if ") {
                format!("else {b_code}")
            } else {
                format!("else {{ {b_code} }}")
            };
            let c = cond(&c);
            return Ok((
                format!("(if {} {{ {} }} {tail})", unwrap(&c), unwrap(&a.0)),
                a.1,
            ));
        }
        Ok(c)
    }

    fn or(&mut self) -> Result<Code, String> {
        let mut l = self.and()?;
        while self.eat("||") || self.eat_word("or") {
            let r = self.and()?;
            l = (format!("{} || {}", cond(&l), cond(&r)), Ty::Bool);
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Code, String> {
        let mut l = self.not()?;
        while self.eat("&&") || self.eat_word("and") {
            let r = self.not()?;
            l = (format!("{} && {}", cond(&l), cond(&r)), Ty::Bool);
        }
        Ok(l)
    }

    fn not(&mut self) -> Result<Code, String> {
        if self.eat("!") || self.eat_word("not") {
            let x = self.not()?;
            return Ok((format!("!{}", paren(&cond(&x))), Ty::Bool));
        }
        self.cmp()
    }

    fn cmp(&mut self) -> Result<Code, String> {
        let l = self.add()?;
        for (op, rhai) in [
            ("==", "=="),
            ("!=", "!="),
            ("<>", "!="),
            ("<=", "<="),
            (">=", ">="),
            ("<", "<"),
            (">", ">"),
        ] {
            if self.eat(op) {
                let r = self.add()?;
                return Ok((format!("{} {rhai} {}", l.0, r.0), Ty::Bool));
            }
        }
        for (op, neg) in [("=~", false), ("!~", true)] {
            if self.eat(op) {
                let Some(Tok::Regex(re)) = self.peek().cloned() else {
                    return Err("expression régulière attendue".into());
                };
                self.pos += 1;
                let m = regex(&l.0, &re)?;
                return Ok((if neg { format!("!({m})") } else { m }, Ty::Bool));
            }
        }
        Ok(l)
    }

    fn add(&mut self) -> Result<Code, String> {
        let mut l = self.mul()?;
        loop {
            let op = if self.eat("+") {
                "+"
            } else if self.eat("-") {
                "-"
            } else {
                return Ok(l);
            };
            let r = self.mul()?;
            let ty = if l.1 == Ty::Str && r.1 == Ty::Str {
                Ty::Str
            } else {
                Ty::Num
            };
            l = (format!("{} {op} {}", l.0, r.0), ty);
        }
    }

    fn mul(&mut self) -> Result<Code, String> {
        let mut l = self.unary()?;
        loop {
            let op = if self.eat("*") {
                "*"
            } else if self.eat("/") {
                "/"
            } else if self.eat("%") {
                "%"
            } else {
                return Ok(l);
            };
            let r = self.unary()?;
            l = (format!("{} {op} {}", l.0, r.0), Ty::Num);
        }
    }

    fn unary(&mut self) -> Result<Code, String> {
        if self.eat("-") {
            let x = self.unary()?;
            return Ok((format!("-{}", x.0), Ty::Num));
        }
        if self.eat("+") {
            return self.unary();
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Code, String> {
        let Some(t) = self.peek().cloned() else {
            return Err("expression incomplète".into());
        };
        self.pos += 1;
        match t {
            Tok::Num(n) => Ok((float_literal(&n), Ty::Num)),
            Tok::Str(s) => Ok((quote(&s), Ty::Str)),
            Tok::Op("(") => {
                let e = self.ternary()?;
                self.expect(")")?;
                Ok((format!("({})", unwrap(&e.0)), e.1))
            }
            Tok::Ident(w) if w == "true" || w == "false" => Ok((w, Ty::Bool)),
            Tok::Ident(name) => {
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.ternary()?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    let a: Vec<&str> = args.iter().map(|a| a.0.as_str()).collect();
                    return match (name.as_str(), a.len()) {
                        ("min" | "max", 2) => Ok((format!("{name}({}, {})", a[0], a[1]), Ty::Num)),
                        ("int", 1) => Ok((format!("to_int({})", a[0]), Ty::Num)),
                        ("round" | "abs", 1) => Ok((format!("{name}({})", a[0]), Ty::Num)),
                        _ => Err(format!("fonction {name}()")),
                    };
                }
                let index = if self.eat("[") {
                    let i = match self.peek().cloned() {
                        Some(Tok::Num(n)) => n.parse::<usize>().ok(),
                        _ => None,
                    };
                    // Indice d'extrudeur (`initial_tool`…) : une seule tête.
                    let mut depth = 1;
                    while depth > 0 {
                        match self.peek() {
                            Some(Tok::Op("[")) => depth += 1,
                            Some(Tok::Op("]")) => depth -= 1,
                            None => return Err("« ] » attendu".into()),
                            _ => {}
                        }
                        self.pos += 1;
                    }
                    Some(i.unwrap_or(0))
                } else {
                    None
                };
                self.tr.variable(&name, index)
            }
            _ => Err("expression invalide".into()),
        }
    }
}

/// Sans la paire de parenthèses qui entoure toute l'expression.
fn unwrap(s: &str) -> &str {
    let Some(inner) = s.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return s;
    };
    let mut depth = 0i32;
    let mut quoted = false;
    for c in inner.chars() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => {
                depth -= 1;
                if depth < 0 {
                    return s;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        inner
    } else {
        s
    }
}

fn paren(s: &str) -> String {
    if s.starts_with('(') && s.ends_with(')') || !s.contains(' ') {
        s.to_string()
    } else {
        format!("({s})")
    }
}

/// Valeur employée comme condition (`{if wipe}`, `a and b`).
fn cond((code, ty): &Code) -> String {
    match ty {
        Ty::Bool => code.clone(),
        Ty::Num => format!("({code} != 0.0)"),
        Ty::Str => format!("({code} != \"\")"),
    }
}

/// `x =~ /.*PET.*/` et ses variantes simples : contient, commence,
/// finit, égale, avec alternatives `(A|B)`.
fn regex(lhs: &str, re: &str) -> Result<String, String> {
    let re = re.trim_start_matches('^').trim_end_matches('$');
    let (any_start, rest) = match re.strip_prefix(".*") {
        Some(r) => (true, r),
        None => (false, re),
    };
    let (any_end, core) = match rest.strip_suffix(".*") {
        Some(r) => (true, r),
        None => (false, rest),
    };
    let alts: Vec<&str> = match core.strip_prefix('(').and_then(|c| c.strip_suffix(')')) {
        Some(inner) => inner.split('|').collect(),
        None => vec![core],
    };
    let plain = |a: &str| {
        !a.is_empty()
            && a.chars()
                .all(|c| c.is_ascii_alphanumeric() || " _-.".contains(c))
    };
    if !alts.iter().all(|a| plain(a)) {
        return Err(format!("expression régulière /{re}/"));
    }
    let test = |a: &str| match (any_start, any_end) {
        (true, true) => format!("{lhs}.contains({})", quote(a)),
        (false, true) => format!("{lhs}.starts_with({})", quote(a)),
        (true, false) => format!("{lhs}.ends_with({})", quote(a)),
        (false, false) => format!("{lhs} == {}", quote(a)),
    };
    let parts: Vec<String> = alts.iter().map(|a| test(a)).collect();
    Ok(if parts.len() == 1 {
        parts[0].clone()
    } else {
        format!("({})", parts.join(" || "))
    })
}

impl Translator<'_> {
    fn variable(&mut self, name: &str, index: Option<usize>) -> Result<Code, String> {
        // Forme ancienne `[temperature_0]`.
        let (name, index) = match name.rsplit_once('_') {
            Some((n, i)) if index.is_none() && i.parse::<usize>().is_ok() && known(n).is_some() => {
                (n, i.parse().ok())
            }
            _ => (name, index),
        };
        if let Some(v) = axis(name, index.unwrap_or(0)) {
            return Ok((v, Ty::Num));
        }
        // Dans le gabarit de fin, la dernière couche est la plus haute.
        if name == "layer_z" && self.field == "gcode.end" {
            return Ok(("max_z".into(), Ty::Num));
        }
        if let Some((v, ty)) = known(name) {
            return Ok((v.to_string(), ty));
        }
        // Réglage du fichier : constant pendant l'impression, remplacé par
        // sa valeur (et listé).
        if let Some(v) = self.src.element(name, index.unwrap_or(0)) {
            self.constants.push(Constant {
                field: self.field,
                name: name.to_string(),
                value: v.clone(),
            });
            return Ok(match v.trim().parse::<f64>() {
                Ok(n) if n.is_finite() => (float_literal(v.trim()), Ty::Num),
                _ => (quote(&v), Ty::Str),
            });
        }
        Err(format!("variable {name}"))
    }

    fn expr(&mut self, inner: &str) -> Result<Code, String> {
        let toks = lex(inner)?;
        if toks.is_empty() {
            return Err("expression vide".into());
        }
        let mut p = Parser {
            toks,
            pos: 0,
            tr: self,
        };
        let e = p.ternary()?;
        if p.pos < p.toks.len() {
            return Err("instruction".into());
        }
        Ok(e)
    }

    fn fail(&mut self, line: usize, text: &str, reason: String) {
        self.untranslated.push(Untranslated {
            field: self.field,
            line,
            text: text.trim().to_string(),
            reason,
        });
    }

    /// Bloc `if` en expression Rhai dont chaque branche est une chaîne.
    fn block(
        &mut self,
        branches: &[(Option<String>, Vec<Node>)],
    ) -> Result<String, (usize, String)> {
        let mut out = String::new();
        for (i, (c, body)) in branches.iter().enumerate() {
            let mut body = self.in_string(body)?;
            // Rhai saute le saut de ligne qui suit l'accent grave ouvrant.
            if body.starts_with('\n') {
                body.insert(0, '\n');
            }
            match c {
                Some(c) => {
                    let code = self.expr(c).map_err(|e| (0, e))?;
                    if i > 0 {
                        out.push_str(" else ");
                    }
                    out.push_str(&format!("if {} {{ `{body}` }}", cond(&code)));
                }
                None => out.push_str(&format!(" else {{ `{body}` }}")),
            }
        }
        if branches.last().is_some_and(|b| b.0.is_some()) {
            out.push_str(" else { \"\" }");
        }
        Ok(out)
    }

    /// Corps d'une branche : texte, valeurs `${…}`, blocs imbriqués.
    fn in_string(&mut self, nodes: &[Node]) -> Result<String, (usize, String)> {
        let mut out = String::new();
        for n in nodes {
            match n {
                Node::Text(t) => {
                    if t.contains('`') || t.contains("${") {
                        return Err((0, "texte".into()));
                    }
                    out.push_str(t);
                }
                Node::Expr { inner, line, .. } => {
                    let code = self.expr(inner).map_err(|e| (*line, e))?;
                    out.push_str(&format!("${{{}}}", as_text(&code)));
                }
                Node::If { branches, line, .. } => {
                    let b = self
                        .block(branches)
                        .map_err(|(l, e)| (if l == 0 { *line } else { l }, e))?;
                    out.push_str(&format!("${{{b}}}"));
                }
                Node::Unclosed { line, .. } => return Err((*line, "{endif}".into())),
            }
        }
        Ok(out)
    }

    fn top(&mut self, nodes: &[Node]) -> String {
        let mut out = String::new();
        for n in nodes {
            match n {
                Node::Text(t) => out.push_str(&t.replace('{', "{{").replace('}', "}}")),
                Node::Expr { inner, line, raw } => match self.expr(inner) {
                    Ok((code, _)) => out.push_str(&format!("{{{}}}", unwrap(&code))),
                    Err(e) => {
                        self.fail(*line, raw, e);
                        // Forme `[var]` gardée en `{var}` : le contrôle du
                        // profil la signale au lieu de l'écrire telle quelle.
                        out.push_str(&format!("{{{inner}}}"));
                    }
                },
                Node::If {
                    branches,
                    line,
                    raw,
                } => {
                    let saved = self.constants.len();
                    match self.block(branches) {
                        Ok(b) => out.push_str(&format!("{{{b}}}")),
                        Err((l, e)) => {
                            self.constants.truncate(saved);
                            let at = if l == 0 { *line } else { l };
                            let first = raw.lines().next().unwrap_or(raw);
                            self.fail(at, first, e);
                            out.push_str(raw);
                        }
                    }
                }
                Node::Unclosed { line, raw } => {
                    let first = raw.lines().next().unwrap_or(raw);
                    self.fail(*line, first, "{endif}".into());
                    out.push_str(raw);
                }
            }
        }
        out
    }
}

/// Valeur interpolée dans une chaîne : nombres écrits comme le reste du
/// gabarit (`num`), booléens en 1 / 0.
fn as_text((code, ty): &Code) -> String {
    match ty {
        Ty::Num => format!("num({code})"),
        Ty::Bool => format!("if {code} {{ \"1\" }} else {{ \"0\" }}"),
        Ty::Str => code.clone(),
    }
}

fn translate(template: &str, src: &Source, field: &'static str) -> Translation {
    let mut tr = Translator {
        src,
        field,
        untranslated: Vec::new(),
        constants: Vec::new(),
    };
    let nodes = tree(template);
    let mut text = tr.top(&nodes);
    if !text.ends_with('\n') {
        text.push('\n');
    }
    // Une même constante n'est listée qu'une fois.
    let mut seen = Vec::new();
    tr.constants.retain(|c| {
        let k = (c.name.clone(), c.value.clone());
        let new = !seen.contains(&k);
        seen.push(k);
        new
    });
    Translation {
        text,
        untranslated: tr.untranslated,
        constants: tr.constants,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gcode::sample_template_scope;
    use crate::template::TemplateEngine;

    fn src(pairs: &[(&str, &str)]) -> Source {
        Source {
            map: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), vec![v.to_string()]))
                .collect(),
        }
    }

    #[test]
    fn templates_are_translated_to_rhai() {
        let s = src(&[
            ("printer_notes", "PRINTER_VENDOR_PRUSA3D MK4S"),
            ("z_offset", "0"),
        ]);
        let t = translate(
            "M862.1 P[nozzle_diameter] A{(filament_abrasive[0] ? 1 : 0)}\n\
             M140 S[first_layer_bed_temperature]\n\
             {if filament_type[initial_tool]==\"FLEX\"}\nG1 E-4\n{else}\nG1 E-2\n{endif}\n\
             {if is_extruder_used[0]}\n\nM117 x{endif}\n\
             {if printer_notes=~/.*(MK4IS|MK4S).*/}M572 S{pressure_advance}{endif}\n\
             M555 X{min(print_bed_max[0], first_layer_print_min[0] + 32) - 32}\n\
             G1 Z{z_offset+min(max_layer_z+1, max_print_height)}",
            &s,
            "gcode.start",
        );
        assert!(t.untranslated.is_empty(), "{:?}", t.untranslated);
        let e = TemplateEngine::new();
        let mut vars = sample_template_scope();
        vars.set_or_push("filament_type", "FLEX".to_string());
        vars.set_or_push("nozzle_diameter", 0.4_f64);
        vars.set_or_push("first_layer_bed_temperature", 60.0_f64);
        vars.set_or_push("pressure_advance", 0.036_f64);
        vars.set_or_push("bed_size_x", 250.0_f64);
        vars.set_or_push("first_layer_min_x", 100.0_f64);
        vars.set_or_push("max_z", 40.0_f64);
        vars.set_or_push("max_print_height", 220.0_f64);
        e.check(&t.text, &vars).unwrap();
        let out = e.render(&t.text, &vars).unwrap();
        assert_eq!(
            out,
            "M862.1 P0.4 A0\nM140 S60\n\nG1 E-4\n\n\n\nM117 x\nM572 S0.036\nM555 X100\nG1 Z41\n"
        );
        assert_eq!(t.constants.len(), 2);
    }

    #[test]
    fn untranslatable_parts_are_kept_and_listed() {
        let t = translate(
            "G28\n{if layer_num < 2}M117 a{endif}\nM104 S[unknown_option]\n{local x = 1}\n",
            &Source::default(),
            "gcode.start",
        );
        let lines: Vec<usize> = t.untranslated.iter().map(|u| u.line).collect();
        assert_eq!(lines, [2, 3, 4]);
        let e = TemplateEngine::new();
        let err = e.check(&t.text, &sample_template_scope()).unwrap_err();
        assert_eq!(err.line, 2);
    }

    #[test]
    fn pressure_advance_from_prusa_filament_gcode() {
        let g = "{if printer_notes!~/.*(MK4IS).*/}\nM900 K{if nozzle_diameter[0]==0.4}0.05{endif}\n{else}\nM572 S{if nozzle_diameter[0]==0.4}0.036{elsif nozzle_diameter[0]==0.5}0.025{endif}\n{endif}";
        assert_eq!(gcode_pressure_advance(g), Some(("M572", 0.036)));
        assert_eq!(gcode_pressure_advance("M900 K0.05"), Some(("M900", 0.05)));
    }

    #[test]
    fn bambu_process_variables_are_mapped() {
        let t = translate(
            "M620.1 E F{flush_volumetric_speeds[initial_no_support_extruder]/2.4053*60} T{flush_temperatures[initial_no_support_extruder]}\n\
             G0 X240 F{outer_wall_volumetric_speed/(0.3*0.5) * 60}\n\
             G1 Y{first_layer_center_no_wipe_tower[1]}\n\
             {if !spiral_mode && print_sequence != \"by object\"}M971{endif}\n",
            &Source::default(),
            "gcode.end",
        );
        assert!(t.untranslated.is_empty(), "{:?}", t.untranslated);
        let e = TemplateEngine::new();
        let mut vars = sample_template_scope();
        vars.set_or_push("max_volumetric_speed", 12.0_f64);
        vars.set_or_push("temperature", 220.0_f64);
        vars.set_or_push("first_layer_min_y", 100.0_f64);
        vars.set_or_push("first_layer_max_y", 140.0_f64);
        let out = e.render(&t.text, &vars).unwrap();
        assert_eq!(
            out,
            "M620.1 E F299.339 T220\nG0 X240 F4800\nG1 Y120\nM971\n"
        );
    }

    #[test]
    fn bundle_presets_are_named_after_their_section() {
        // Un parent commun déclare un nom vide (comme PrusaResearch.ini).
        let ini = "[filament:*common*]\nfilament_settings_id = \"\"\nfilament_type = PLA\n\n\
                   [filament:Generic FLEX @MK4S]\ninherits = *common*\nfilament_type = FLEX\n";
        let sets = ini_sets(ini);
        assert_eq!(sets.len(), 1);
        assert_eq!(
            sets[0].src.text("filament_settings_id").as_deref(),
            Some("Generic FLEX @MK4S")
        );
        assert_eq!(sets[0].src.text("filament_type").as_deref(), Some("FLEX"));
    }

    #[test]
    fn ini_values_are_unescaped() {
        assert_eq!(
            ini_value(r"G28\nM104 S[temperature]"),
            ["G28\nM104 S[temperature]"]
        );
        assert_eq!(ini_value(r#""; a\nb";"c""#), ["; a\nb", "c"]);
        assert_eq!(slug("Generic PLA @PG (copie)"), "generic_pla_pg_copie");
    }
}
