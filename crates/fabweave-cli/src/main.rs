//! `fabweave` : vérifier, mesurer et exporter un modèle `.weave` sans
//! interface. Pensé pour les auteurs de modèles et pour une IA qui vient
//! d'écrire un fichier : `fabweave check modele.weave` dit, ligne par ligne,
//! ce qui ne va pas, et sort avec le code 1 tant qu'il reste une erreur.
//!
//! ```text
//! fabweave check  <modèle.weave> [options]   calcul aux valeurs par défaut et aux bornes
//! fabweave stats  <modèle.weave> [options]   mesures (points, durée, filament, encombrement)
//! fabweave export <modèle.weave> -o <sortie.gcode> [options]
//!
//! options : --printer <profil.toml>  --filament <profil.toml>
//!           --set <id>=<valeur> (répétable)  --json
//! ```
//!
//! Profils par défaut : Prusa MK4S et PLA générique (livrés avec FabWeave).

use std::process::ExitCode;

use fabweave_core::model::ParamKind;
use fabweave_core::plan::Stats;
use fabweave_core::{Issue, Severity, Workbench};
use serde_json::{json, Map, Value};

const MK4S: &str = include_str!("../../../profiles/printers/prusa_mk4s.toml");
const PLA: &str = include_str!("../../../profiles/filaments/generic_pla.toml");

const USAGE: &str = "usage :
  fabweave check  <modèle.weave> [options]   calcul aux valeurs par défaut et aux bornes de chaque paramètre
  fabweave stats  <modèle.weave> [options]   mesures aux valeurs données
  fabweave export <modèle.weave> -o <sortie.gcode> [options]
options :
  --printer <profil.toml>   profil imprimante (défaut : Prusa MK4S)
  --filament <profil.toml>  profil filament (défaut : PLA générique)
  --set <id>=<valeur>       valeur d'un paramètre (répétable)
  --json                    sortie JSON
codes de sortie : 0 aucun message bloquant, 1 erreurs, 2 mauvaise utilisation";

struct Args {
    command: String,
    model: String,
    printer: Option<String>,
    filament: Option<String>,
    output: Option<String>,
    values: Map<String, Value>,
    json: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut it = std::env::args().skip(1);
    let command = it.next().ok_or("commande manquante")?;
    if !matches!(command.as_str(), "check" | "stats" | "export") {
        return Err(format!("commande inconnue : {command}"));
    }
    let mut a = Args {
        command,
        model: String::new(),
        printer: None,
        filament: None,
        output: None,
        values: Map::new(),
        json: false,
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} : valeur manquante"));
        match arg.as_str() {
            "--printer" => a.printer = Some(value("--printer")?),
            "--filament" => a.filament = Some(value("--filament")?),
            "-o" | "--output" => a.output = Some(value("-o")?),
            "--json" => a.json = true,
            "--set" => {
                let kv = value("--set")?;
                let (k, v) = kv
                    .split_once('=')
                    .ok_or(format!("--set {kv} : id=valeur attendu"))?;
                let v = serde_json::from_str::<Value>(v).unwrap_or(Value::String(v.to_string()));
                a.values.insert(k.to_string(), v);
            }
            s if s.starts_with('-') => return Err(format!("option inconnue : {s}")),
            s if a.model.is_empty() => a.model = s.to_string(),
            s => return Err(format!("argument en trop : {s}")),
        }
    }
    if a.model.is_empty() {
        return Err("fichier modèle manquant".into());
    }
    if a.command == "export" && a.output.is_none() {
        return Err("export : -o <sortie.gcode> manquant".into());
    }
    Ok(a)
}

/// Explication courte des codes les plus fréquents (liste complète :
/// docs/WEAVE_FORMAT.md, section 5).
fn hint(code: &str) -> &'static str {
    match code {
        "model.missing_separator" => "il manque la ligne « --- » entre l'en-tête TOML et le script",
        "model.invalid_header" => "en-tête TOML invalide ou champ inconnu",
        "param.missing_field" => "champ obligatoire manquant",
        "param.unknown_type" => "type inconnu (float, int, bool, choice)",
        "param.default_out_of_range" => "valeur par défaut hors des bornes",
        "param.invalid_bounds" => "min doit être inférieur à max",
        "param.not_an_integer" => "un entier est attendu",
        "param.invalid_id" => "identifiant : minuscules, chiffres et _",
        "param.unknown_option" => "option absente de la liste",
        "script.syntax" => "erreur de syntaxe Rhai",
        "script.unknown_variable" => "variable inconnue (déclarer avec let ; params.x, printer.x)",
        "script.unknown_property" => {
            "propriété inconnue : faute de frappe dans params.<id> ou printer.<champ> ?"
        }
        "script.unknown_function" => "fonction inconnue ou mauvais nombre / type d'arguments",
        "script.wrong_type" => "type inattendu (boucle for sur un flottant ? utiliser .to_int())",
        "script.no_path" => "le script doit se terminer par le parcours créé par new_path()",
        "script.too_many_operations" => "calcul trop long (boucle sans fin ?)",
        "script.constant_assignment" => "params et printer ne se modifient pas",
        "path.empty" => "aucun mouvement ne dépose de matière (division entière i / n ?)",
        "path.duplicates_skipped" => "points confondus ignorés (division entière i / n ?)",
        "path.out_of_bed" => "la pièce dépasse du plateau",
        "path.too_high" => "la pièce dépasse la hauteur d'impression",
        "path.below_bed" => "un point descend sous Z = 0",
        "path.invalid_width" | "path.invalid_height" => {
            "largeur et hauteur du boudin doivent être positives"
        }
        "model.failure" => "le modèle refuse ces valeurs (fail)",
        "model.warning" => "avertissement du modèle (warn)",
        "speed.limited_flow" => "vitesse réduite par le débit max du filament (normal)",
        "speed.limited_first_layer" => "première couche ralentie (normal)",
        "speed.limited_z" => "vitesse réduite par l'axe Z (normal en non planaire)",
        _ => "",
    }
}

fn describe(i: &Issue) -> String {
    let sev = match i.severity {
        Severity::Error => "ERREUR",
        Severity::Warning => "attention",
        Severity::Info => "info",
    };
    let mut s = format!("  {sev}");
    if let Some(l) = i.line {
        s.push_str(&format!(" ligne {l}"));
        if let Some(c) = i.column {
            s.push_str(&format!(", col. {c}"));
        }
    }
    s.push_str(&format!(" [{}]", i.code));
    if let Some(f) = &i.field {
        s.push_str(&format!(" {f}"));
    }
    if let Some(v) = i.value {
        s.push_str(&format!(" = {v}"));
    }
    if let Some(n) = i.count {
        s.push_str(&format!(" ×{n}"));
    }
    let h = hint(i.code);
    if !h.is_empty() {
        s.push_str(&format!(" : {h}"));
    }
    if let Some(t) = &i.text {
        s.push_str(&format!(" « {} »", t.get("fr")));
    }
    if let Some(d) = &i.detail {
        s.push_str(&format!(" — {d}"));
    }
    s
}

fn summary(s: &Stats) -> String {
    let b = &s.bounds;
    format!(
        "{} points, {:.0} min, {:.1} g, {:.2} m de filament, encombrement {:.1} × {:.1} × {:.1} mm, débit max {:.1} mm³/s",
        s.points,
        s.time / 60.0,
        s.filament_mass,
        s.filament_length / 1000.0,
        b.max[0] - b.min[0],
        b.max[1] - b.min[1],
        b.max[2],
        s.max_flow
    )
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{path} : {e}"))
}

fn run() -> Result<bool, String> {
    let a = parse_args()?;
    let mut w = Workbench::default();
    let printer = a.printer.as_deref().map(read).transpose()?;
    let filament = a.filament.as_deref().map(read).transpose()?;
    let fail_load = |what: &str, e: Vec<Issue>| -> String {
        let lines: Vec<String> = e.iter().map(describe).collect();
        format!("{what} illisible :\n{}", lines.join("\n"))
    };
    w.set_printer(printer.as_deref().unwrap_or(MK4S))
        .map_err(|e| fail_load("profil imprimante", e))?;
    w.set_filament(filament.as_deref().unwrap_or(PLA))
        .map_err(|e| fail_load("profil filament", e))?;
    let source = read(&a.model)?;
    let model = match w.load_model(&source) {
        Ok(m) => m.clone(),
        Err(e) => {
            if a.json {
                println!("{}", json!({ "ok": false, "issues": e }));
            } else {
                println!("{} : modèle illisible", a.model);
                for i in &e {
                    println!("{}", describe(i));
                }
            }
            return Ok(false);
        }
    };

    match a.command.as_str() {
        "export" => {
            let ev = w.evaluate(&a.values);
            match w.gcode(&ev) {
                Ok(g) => {
                    let out = a.output.as_deref().unwrap_or_default();
                    std::fs::write(out, g).map_err(|e| format!("{out} : {e}"))?;
                    if !a.json {
                        for i in &ev.issues {
                            println!("{}", describe(i));
                        }
                        if let Some(p) = &ev.plan {
                            println!("{}", summary(&p.stats));
                        }
                        println!("→ {out}");
                    }
                    Ok(true)
                }
                Err(e) => {
                    for i in &e {
                        println!("{}", describe(i));
                    }
                    Ok(false)
                }
            }
        }
        "stats" => {
            let ev = w.evaluate(&a.values);
            let ok = !ev.issues.iter().any(Issue::is_error);
            if a.json {
                println!(
                    "{}",
                    json!({ "ok": ok, "params": ev.params, "stats": ev.plan.as_ref().map(|p| &p.stats), "issues": ev.issues, "log": ev.log })
                );
            } else {
                for l in &ev.log {
                    println!("  print : {l}");
                }
                for i in &ev.issues {
                    println!("{}", describe(i));
                }
                if let Some(p) = &ev.plan {
                    println!("{}", summary(&p.stats));
                }
            }
            Ok(ok)
        }
        _ => {
            // check : valeurs par défaut (et --set), puis chaque borne de chaque paramètre.
            let mut runs: Vec<(String, Map<String, Value>)> =
                vec![("valeurs par défaut".into(), a.values.clone())];
            for (id, def) in &model.params {
                let ends: Vec<Value> = match &def.kind {
                    ParamKind::Float { min, max, .. } => vec![json!(min), json!(max)],
                    ParamKind::Int { min, max, .. } => vec![json!(min), json!(max)],
                    ParamKind::Bool { .. } => vec![json!(false), json!(true)],
                    ParamKind::Choice { options, .. } => {
                        options.iter().map(|o| json!(o.value)).collect()
                    }
                };
                for v in ends {
                    let mut values = a.values.clone();
                    values.insert(id.clone(), v.clone());
                    runs.push((format!("{id} = {v}"), values));
                }
            }
            let mut ok = true;
            let mut report = Vec::new();
            if !a.json {
                println!(
                    "{} : « {} », {} paramètres",
                    a.model,
                    model.meta.name.get("fr"),
                    model.params.len()
                );
            }
            for (k, (label, values)) in runs.iter().enumerate() {
                let ev = w.evaluate(values);
                // Aux bornes, une pièce trop grande pour le plateau n'est pas une faute du modèle.
                let blocking: Vec<&Issue> = ev
                    .issues
                    .iter()
                    .filter(|i| {
                        i.is_error()
                            && (k == 0 || !matches!(i.code, "path.out_of_bed" | "path.too_high"))
                    })
                    .collect();
                ok &= blocking.is_empty();
                if a.json {
                    report.push(json!({ "run": label, "ok": blocking.is_empty(), "issues": ev.issues, "stats": ev.plan.as_ref().map(|p| &p.stats) }));
                    continue;
                }
                let shown: Vec<&Issue> = if k == 0 {
                    ev.issues.iter().collect()
                } else {
                    ev.issues
                        .iter()
                        .filter(|i| i.severity != Severity::Info)
                        .collect()
                };
                if k == 0 || !shown.is_empty() {
                    let mark = if blocking.is_empty() { "✓" } else { "✗" };
                    match &ev.plan {
                        Some(p) if k == 0 => println!("{mark} {label} : {}", summary(&p.stats)),
                        _ => println!("{mark} {label}"),
                    }
                    for i in shown {
                        println!("{}", describe(i));
                    }
                }
            }
            if a.json {
                println!("{}", json!({ "ok": ok, "runs": report }));
            } else {
                println!(
                    "{} : {} calculs, {}",
                    if ok { "OK" } else { "ÉCHEC" },
                    runs.len(),
                    if ok {
                        "aucun message bloquant"
                    } else {
                        "corriger les ERREURS ci-dessus"
                    }
                );
            }
            Ok(ok)
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}
