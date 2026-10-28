//! API du script (v0.2) : listes de points natives, transformations,
//! modulation, ponts, annotations, messages du modèle, paramètres `choice`.

use fabweave_core::{MoveKind, Workbench};
use serde_json::{json, Map, Value};

const MK4S: &str = include_str!("../../../profiles/printers/prusa_mk4s.toml");
const PLA: &str = include_str!("../../../profiles/filaments/generic_pla.toml");

fn bench(header: &str, body: &str) -> Workbench {
    let mut w = Workbench::default();
    w.set_printer(MK4S).unwrap();
    w.set_filament(PLA).unwrap();
    w.load_model(&format!(
        "[meta]\nname = \"t\"\ncategory = \"t\"\n{header}\n---\n{body}"
    ))
    .unwrap_or_else(|e| panic!("{e:?}"));
    w
}

fn eval(body: &str) -> fabweave_core::Evaluation {
    bench("", body).evaluate(&Map::new())
}

fn values(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

#[test]
fn shapes_have_the_expected_points() {
    let ev = eval(
        "let p = new_path();
let c = circle(0, 0, 0.2, 10, 0, 8);
let a = arc(0, 0, 0.2, 10, 0, PI, 4);
let g = polygon(0, 0, 0.2, 10, 6, 0);
let r = rectangle(-5, -5, 0.2, 10, 10);
let h = helix(0, 0, 0.2, 10, 10, 0, 2, 0.5, 36);
let s = spiral(0, 0, 0.2, 2, 12, 0, 3, 36);
let z = zigzag(-10, -10, 0.2, 20, 1, 5);
let w = sine(-10, 0, 0.2, 20, 2, 5, 8);
let l = line(0, 0, 0.2, 10, 0, 0.2, 5);
print(`${c.len()} ${a.len()} ${g.len()} ${r.len()} ${h.len()} ${s.len()} ${z.len()} ${w.len()} ${l.len()}`);
let last = h.point(-1);
print(`${last[2]}`);
p.travel_to(c);
p.print_points(c, 0.45, 0.2);
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    assert_eq!(ev.log[0], "9 5 7 5 73 109 10 33 6");
    // Hélice : 2 tours au pas de 0,5 mm.
    assert_eq!(ev.log[1], "1.2");
    let pts = &ev.path.unwrap().points;
    // Déplacement au premier point, puis les 8 autres (le premier, confondu, est sauté).
    assert_eq!(pts.len(), 9);
    assert_eq!(pts[0].kind, MoveKind::Travel);
    // Cercle fermé : le dernier point revient au premier.
    assert!((pts[8].x - pts[0].x).abs() < 1e-12 && (pts[8].y - pts[0].y).abs() < 1e-12);
}

#[test]
fn transforms_are_exact() {
    let ev = eval(
        "let p = new_path();
let r = rectangle(0, 0, 0.2, 10, 5);
let t = r.translate(5, 0, 0.1).point(0);
let q = r.rotate(PI / 2, 0, 0).point(1);
let m = r.mirror_x(0).point(1);
let k = r.scale(2, 0, 0).point(2);
let c = r.copies(3, 0, 0, 0.2);
let z = r.repeat_z(4, 0.2);
let o = r.polar_copies(4, 0, 0);
let d = line(0, 0, 0.2, 10, 0, 0.2, 1).subdivide(1.5);
let rev = r.reverse().point(1);
let both = r + r;
print(`${t} ${q[0].round()} ${q[1].round()} ${m[0]} ${k} ${c.len()} ${z.point(-1)[2]} ${o.len()} ${d.len()} ${rev} ${both.len()}`);
p.print_points(r, 0.45, 0.2);
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let l = &ev.log[0];
    assert_eq!(l, "[5.0, 0.0, 0.30000000000000004] 0.0 10.0 -10.0 [20.0, 10.0, 0.2] 15 0.8 20 8 [0.0, 5.0, 0.2] 10");
}

#[test]
fn modulation_calls_back_the_script() {
    let ev = eval(
        "let p = new_path();
let c = circle(0, 0, 0.2, 10, 0, 4).modulate_r(0, 0, |t, a, z| 2.0).modulate_z(|t, x, y, z| t);
let a = c.point(0);
let b = c.point(-1);
print(`${a[0]} ${b[2]}`);
p.print_points(c, 0.45, 0.2);
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    assert_eq!(ev.log[0], "12.0 1.2");
}

#[test]
fn bridges_notes_warnings_and_failures() {
    let ev = eval(
        "let p = new_path();
p.travel_to(0, 0, 0.2);
p.print_to(5, 0, 0.2, 0.45, 0.2);
p.note(\"Départ\");
p.bridge_to(15, 0, 0.2, 0.45, 0.2, 0.85);
p.note_at(10, 0, 0.2, #{ fr: \"Pont\", en: \"Bridge\" });
warn(\"Pont long\");
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let path = ev.path.as_ref().unwrap();
    assert_eq!(path.points[2].kind, MoveKind::Bridge);
    assert_eq!(path.points[2].flow, 0.85);
    assert_eq!(path.notes.len(), 2);
    assert_eq!(path.notes[0].x, 5.0);
    assert_eq!(path.notes[1].text.get("en"), "Bridge");
    let w = ev
        .issues
        .iter()
        .find(|i| i.code == "model.warning")
        .unwrap();
    // Le script commence ligne 6 (en-tête de 3 lignes, ligne vide, `---`).
    assert_eq!(w.line, Some(12));
    assert_eq!(w.text.as_ref().unwrap().get("fr"), "Pont long");
    // Pont : ventilateur forcé du filament (100 % en PLA), débit × 0,85.
    let plan = ev.plan.unwrap();
    assert_eq!(plan.moves[2].fan, 100);

    let ev = eval(
        "if true { fail(#{ fr: \"Pores trop petits\", en: \"Pores too small\" }); }\nnew_path()",
    );
    assert_eq!(ev.issues[0].code, "model.failure");
    assert_eq!(ev.issues[0].line, Some(6));
    assert_eq!(
        ev.issues[0].text.as_ref().unwrap().get("en"),
        "Pores too small"
    );

    let ev = eval("let p = new_path();\np.bridge_to(1, 0, 0.2, 0.45, 0.2, 3);\np");
    assert_eq!(ev.issues[0].code, "path.invalid_flow");
}

#[test]
fn choice_parameters_reach_the_script() {
    let w = bench(
        "[params.pattern]\ntype = \"choice\"\nlabel = \"Motif\"\noptions = [\"zigzag\", \"smooth\"]\ndefault = \"zigzag\"",
        "let p = new_path();\nprint(params.pattern);\nlet n = if params.pattern == \"smooth\" { 12 } else { 2 };\np.print_points(line(0, 0, 0.2, 10, 0, 0.2, n), 0.45, 0.2);\np",
    );
    let ev = w.evaluate(&Map::new());
    assert_eq!(ev.log, ["zigzag"]);
    assert_eq!(ev.path.unwrap().points.len(), 3);
    let ev = w.evaluate(&values(json!({ "pattern": "smooth" })));
    assert_eq!(ev.path.unwrap().points.len(), 13);
    let ev = w.evaluate(&values(json!({ "pattern": "wavy" })));
    assert_eq!(ev.issues[0].code, "param.unknown_option");
}

#[test]
fn native_shapes_respect_the_point_limit() {
    let ev = eval("let p = new_path();\nlet h = helix(0, 0, 0.2, 10, 10, 0, 100000, 0.2, 36);\np");
    assert_eq!(ev.issues[0].code, "path.too_many_points");
    let ev = eval("let p = new_path();\nlet c = circle(0, 0, 0.2, -1, 0, 8);\np");
    assert_eq!(ev.issues[0].code, "points.invalid_value");
}

/// L'exemple de la spécification (`docs/WEAVE_FORMAT.md`) tourne tel quel.
#[test]
fn specification_example_runs() {
    let spec = include_str!("../../../docs/WEAVE_FORMAT.md");
    // Chaque bloc `rhai` complet de la spécification se calcule sans erreur.
    let blocks: Vec<&str> = spec
        .split("```rhai\n")
        .skip(1)
        .map(|b| &b[..b.find("```").unwrap()])
        .collect();
    assert!(blocks.len() >= 2);
    for b in &blocks {
        let ev = eval(b);
        assert!(
            ev.issues.iter().all(|i| !i.is_error()),
            "{b}\n{:?}",
            ev.issues
        );
    }
    // Le tamis carré : 12 couches de 66 points en pont.
    let sieve = blocks.iter().find(|b| b.contains("bridge_points")).unwrap();
    let ev = eval(sieve);
    let path = ev.path.unwrap();
    assert_eq!(
        path.points
            .iter()
            .filter(|p| p.kind == MoveKind::Bridge)
            .count(),
        12 * 66
    );
    assert!(ev.plan.is_some());
}

/// Les modèles complets du prompt pour IA (`docs/PROMPT_MODELE_IA.md`) se
/// lisent et se calculent sans message bloquant.
#[test]
fn ai_prompt_examples_run() {
    let doc = include_str!("../../../docs/PROMPT_MODELE_IA.md");
    let mut n = 0;
    for block in doc.split("```weave\n").skip(1) {
        let src = &block[..block.find("```").unwrap()];
        let mut w = Workbench::default();
        w.set_printer(MK4S).unwrap();
        w.set_filament(PLA).unwrap();
        w.load_model(src).unwrap_or_else(|e| panic!("{e:?}"));
        let ev = w.evaluate(&Map::new());
        assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
        assert!(w.gcode(&ev).is_ok());
        n += 1;
    }
    assert_eq!(n, 2);
}

/// Modèles écrits par une IA à partir du seul prompt (essai du 24/10/2026,
/// v0.2 étape 2) : ils restent valides tant que l'API ne casse pas.
#[test]
fn ai_trial_models_still_run() {
    for (name, src) in [
        (
            "abat-jour",
            include_str!("fixtures/ai-trial/abat-jour.weave"),
        ),
        ("ressort", include_str!("fixtures/ai-trial/ressort.weave")),
        ("panier", include_str!("fixtures/ai-trial/panier.weave")),
    ] {
        let mut w = Workbench::default();
        w.set_printer(MK4S).unwrap();
        w.set_filament(PLA).unwrap();
        w.load_model(src)
            .unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let ev = w.evaluate(&Map::new());
        assert!(
            ev.issues.iter().all(|i| !i.is_error()),
            "{name}: {:?}",
            ev.issues
        );
    }
}

#[test]
fn fill_lines_slice_and_integer_rounding() {
    let ev = eval(
        "let p = new_path();
let sq = rectangle(-10, -10, 0.2, 20, 20);
let chains = sq.fill_lines(0.5, 0, 0.25);
let a = sq.fill_lines(0.5, PI / 2);
let c = chains[0];
let s = c.slice(1, c.len());
let t = c.slice(-2, -1);
print(`${chains.len()} ${c.len()} ${a[0].len()} ${s.len()} ${t.len()} ${floor(200 / 3)} ${round(7).to_int()}`);
p.travel_to(c);
p.print_points(c, 0.5, 0.2);
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    // Carré de 20 mm : 40 lignes d'un seul tenant, à 0,25 mm des bords.
    assert_eq!(ev.log[0], "1 80 80 79 1 66.0 7");
    let pts = &ev.path.unwrap().points;
    assert!(pts
        .iter()
        .all(|q| q.x.abs() <= 9.75 + 1e-9 && q.y.abs() <= 9.75 + 1e-9));

    for (body, code) in [
        (
            "rectangle(0, 0, 0, 5, 5).fill_lines(0, 0)",
            "points.invalid_value",
        ),
        (
            "rectangle(0, 0, 0, 5, 5).fill_lines(0.5, 0, -1)",
            "points.invalid_value",
        ),
        (
            "rectangle(0, 0, 0, 5, 5).slice(2, 9)",
            "points.index_out_of_range",
        ),
    ] {
        let ev = eval(&format!("let x = {body};\nnew_path()"));
        assert!(
            ev.issues.iter().any(|i| i.code == code),
            "{body}: {:?}",
            ev.issues
        );
    }
}

#[test]
fn fill_rings_from_the_script() {
    let ev = eval(
        "let p = new_path();
let sq = rectangle(-10, -10, 0.2, 20, 20);
let rings = sq.fill_rings(1, 0.5);
let all = sq.fill_rings(1);
print(`${rings.len()} ${rings[0].len()} ${all.len()}`);
for r in rings {
    p.travel_to(r);
    p.print_points(r, 1, 0.2);
}
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    // Carré de 20 mm : anneaux à 0,5 ; 1,5 ; … ; 9,5 mm du bord, fermés ;
    // sans retrait, le premier est le contour lui-même.
    assert_eq!(ev.log[0], "10 5 10");
    let pts = &ev.path.unwrap().points;
    assert!(pts
        .iter()
        .all(|q| q.x.abs() <= 9.5 + 1e-6 && q.y.abs() <= 9.5 + 1e-6));
    for (body, code) in [
        (
            "rectangle(0, 0, 0, 5, 5).fill_rings(0)",
            "points.invalid_value",
        ),
        (
            "rectangle(0, 0, 0, 5, 5).fill_rings(0.5, -1)",
            "points.invalid_value",
        ),
        (
            "rectangle(0, 0, 0, 200, 200).fill_rings(0.0001)",
            "path.too_many_points",
        ),
    ] {
        let ev = eval(&format!("let x = {body};\nnew_path()"));
        assert!(
            ev.issues.iter().any(|i| i.code == code),
            "{body}: {:?}",
            ev.issues
        );
    }
}

#[test]
fn blobs_extrude_without_moving() {
    let w = bench(
        "",
        "let p = new_path();
p.travel_to(0, 0, 1);
p.blob(2, 0.5);
p.travel_to(5, 0, 1);
p.blob(20, 0.5);
p.print_to(10, 0, 1, 0.5, 0.3);
p",
    );
    let ev = w.evaluate(&Map::new());
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let path = ev.path.as_ref().unwrap();
    let plan = ev.plan.as_ref().unwrap();
    let blob = &path.points[1];
    assert_eq!(blob.kind, MoveKind::Blob);
    assert_eq!(
        (blob.x, blob.y, blob.volume, blob.duration),
        (0.0, 0.0, 2.0, 0.5)
    );
    // Diamètre de la sphère de 2 mm³ (aperçu).
    assert!((blob.w - (12.0 / std::f64::consts::PI).cbrt()).abs() < 1e-12);
    // Filament poussé : volume × multiplicateur ÷ section du filament.
    let area = std::f64::consts::PI * 1.75 * 1.75 / 4.0;
    let flow_mult = plan.moves[1].e * area / 2.0;
    assert!((flow_mult - 1.0).abs() < 0.1, "multiplicateur {flow_mult}");
    assert!((plan.moves[1].time - 0.5).abs() < 1e-12);
    // 20 mm³ en 0,5 s dépasse le débit max (15 mm³/s) : la goutte dure plus
    // longtemps, et c'est compté.
    assert!((plan.moves[3].time - 20.0 * flow_mult / 15.0).abs() < 1e-9);
    assert_eq!(plan.stats.limited.flow, 1);
    assert_eq!(plan.stats.blobs, 2);
    assert!(ev.issues.iter().any(|i| i.code == "speed.limited_flow"));
    assert!(plan.stats.time.is_finite() && plan.stats.time > 0.5 + plan.moves[3].time);

    // G-code : extrusion seule, sans coordonnées.
    let g = w.gcode(&ev).unwrap();
    let body = &g[g.find("; --- FabWeave : parcours").unwrap()..];
    let lines: Vec<&str> = body
        .lines()
        .filter(|l| l.starts_with("G1 E") && !l.contains("E-"))
        .collect();
    assert!(lines.len() >= 2, "{lines:?}");
    let feed = plan.moves[1].e / 0.5 * 60.0;
    assert!(
        body.contains(&format!("F{}", feed.round())),
        "F{} : {lines:?}",
        feed.round()
    );

    for (body, code) in [
        (
            "let p = new_path();\np.blob(1, 1);\np",
            "path.blob_without_position",
        ),
        (
            "let p = new_path();\np.travel_to(0, 0, 1);\np.blob(0, 1);\np",
            "path.invalid_blob",
        ),
        (
            "let p = new_path();\np.travel_to(0, 0, 1);\np.blob(1, 0);\np",
            "path.invalid_blob",
        ),
        (
            "let p = new_path();\np.travel_to(0, 0, 1);\np.blob(1, 90);\np",
            "path.invalid_blob",
        ),
    ] {
        let ev = eval(body);
        assert!(
            ev.issues.iter().any(|i| i.code == code),
            "{body}: {:?}",
            ev.issues
        );
    }
}

#[test]
fn a_path_of_blobs_only_is_not_empty() {
    let ev = eval("let p = new_path();\np.travel_to(0, 0, 1);\np.blob(2, 1);\np");
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    assert!(ev.plan.is_some());
}

#[test]
fn noise_is_continuous_and_reproducible() {
    let ev = eval(
        "let a = noise(1.3, 2.7, 0.4);
let b = noise(1.3, 2.7, 0.4);
let c = noise(1.3001, 2.7, 0.4);
print(`${noise(0, 0, 0)} ${a == b} ${abs(a - c) < 0.001} ${abs(a) <= 1.0}`);
let p = new_path();
p.print_to(0, 0, 0.2, 0.4, 0.2);
p.print_to(10, 0, 0.2, 0.4, 0.2);
p",
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    assert_eq!(ev.log[0], "0.0 true true true");
}

#[test]
fn a_wipe_retracts_along_the_last_bead() {
    let w = bench(
        "",
        "let p = new_path();
p.travel_to(0, 0, 0.2);
p.print_to(10, 0, 0.2, 0.45, 0.2);
p.wipe(4);
p.travel_to(10, 20, 0.2);
p.print_to(0, 20, 0.2, 0.45, 0.2);
p",
    );
    let ev = w.evaluate(&Map::new());
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let path = ev.path.as_ref().unwrap();
    let wipe = &path.points[2];
    assert_eq!(wipe.kind, MoveKind::Wipe);
    assert!((wipe.x - 6.0).abs() < 1e-9 && wipe.y == 0.0 && wipe.z == 0.2);
    let plan = ev.plan.as_ref().unwrap();
    // PLA : 0,7 mm rétractés sur les 4 mm, filament à 35 mm/s au plus.
    assert!((plan.moves[2].e + 0.7).abs() < 1e-9, "{}", plan.moves[2].e);
    let v = (35.0 * 4.0 / 0.7_f64).min(plan.moves[3].speed);
    assert!(
        (plan.moves[2].speed - v).abs() < 1e-9,
        "{}",
        plan.moves[2].speed
    );
    let g = w.gcode(&ev).unwrap();
    let body = &g[g.find("; --- FabWeave : parcours").unwrap()..];
    let at = body
        .lines()
        .position(|l| l.starts_with("G1 X") && l.contains(" E-0.7 "))
        .expect(body);
    // Le déplacement suivant ne rétracte pas une seconde fois ; le filament
    // revient avant d'imprimer.
    let after: Vec<&str> = body.lines().skip(at + 1).collect();
    let print = after
        .iter()
        .position(|l| l.contains(" E0.") && l.contains('X'))
        .unwrap();
    assert!(
        !after[..print].iter().any(|l| l.contains("E-")),
        "{after:?}"
    );
    assert!(
        after[..print].iter().any(|l| l.contains("E0.7")),
        "{after:?}"
    );

    for (body, code) in [
        ("let p = new_path();\np.travel_to(0, 0, 0.2);\np.wipe(2);\np", "path.wipe_without_print"),
        ("let p = new_path();\np.travel_to(0, 0, 0.2);\np.print_to(5, 0, 0.2, 0.4, 0.2);\np.wipe(0);\np", "path.invalid_wipe"),
    ] {
        let ev = eval(body);
        assert!(ev.issues.iter().any(|i| i.code == code), "{body}: {:?}", ev.issues);
    }
}

#[test]
fn a_pause_stops_the_printer_with_its_message() {
    let src = "let p = new_path();
p.travel_to(-5, 0, 0.2);
p.print_to(5, 0, 0.2, 0.45, 0.2);
p.travel_to(5, 0, 0.4);
p.pause(#{ fr: \"Insérer l'écrou\", en: \"Insert the nut\" });
p.print_to(-5, 0, 0.4, 0.45, 0.2);
p";
    let w = bench("", src);
    let ev = w.evaluate(&Map::new());
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let path = ev.path.as_ref().unwrap();
    assert_eq!(path.points[3].kind, MoveKind::Pause);
    assert!(path.notes.iter().any(|n| matches!(&n.text, fabweave_core::model::Text::Localized(m) if m["fr"] == "Pause : Insérer l'écrou")));
    assert_eq!(ev.plan.as_ref().unwrap().stats.pauses, 1);
    let g = w.gcode(&ev).unwrap();
    // Texte à l'écran sans accents, filament rétracté avant l'arrêt.
    let at = g.find("M117 Inserer l'ecrou\nM601\n").expect("pause Prusa");
    assert!(
        g[..at].trim_end().ends_with("E-0.7 F2100"),
        "{}",
        &g[at - 80..at]
    );

    // Copies pièce par pièce : une pause par pièce.
    let mut w2 = bench("", src);
    w2.set_setup(serde_json::from_value(json!({ "placement": { "copies": [2, 1] } })).unwrap())
        .unwrap();
    let g = w2.gcode(&w2.evaluate(&Map::new())).unwrap();
    assert_eq!(g.matches("M601").count(), 2);

    // Profil sans gabarit de pause : export refusé.
    let mut w3 = bench("", src);
    let mk4s = MK4S.replace("M117 {pause_text}\nM601\n", "");
    w3.set_printer(&mk4s).unwrap();
    let e = w3.gcode(&w3.evaluate(&Map::new())).unwrap_err();
    assert_eq!(e[0].code, "gcode.pause_missing");
}

/// Réglages d'essai (ADR-0007) : réservés aux modèles `calibration = true`,
/// bornés, écrits dans le G-code ; le filament retiré est remis à l'identique
/// même si la rétraction d'essai change entre-temps.
#[test]
fn calibration_settings_are_gated_bounded_and_written() {
    let body = "let p = new_path();
p.travel_to(0, 0, 0.2);
p.set_temperature_offset(-10);
p.set_retraction_length(2);
p.print_to(10, 0, 0.2, 0.45, 0.2);
p.travel_to(10, 10, 0.2);
p.set_retraction_length(0.5);
p.print_to(0, 10, 0.2, 0.45, 0.2);
p";
    // Modèle ordinaire : refusé.
    let ev = eval(body);
    assert!(
        ev.issues.iter().any(|i| i.code == "model.calibration_only"),
        "{:?}",
        ev.issues
    );
    assert!(ev.path.is_none());

    // Modèle d'essai : averti, écrit.
    let w = bench("calibration = true", body);
    let ev = w.evaluate(&Map::new());
    assert!(
        ev.issues.iter().any(|i| i.code == "model.calibration"),
        "{:?}",
        ev.issues
    );
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
    let g = w.gcode(&ev).unwrap();
    // PLA : première couche à 215 °C, moins 10.
    assert!(g.contains("M104 S205 ; essai : -10 °C"), "consigne d'essai");
    let body_g = &g[g.find("; --- FabWeave : parcours").unwrap()..];
    let lines: Vec<&str> = body_g.lines().collect();
    let r = lines
        .iter()
        .position(|l| *l == "G1 E-2 F2100")
        .expect("rétraction de 2 mm");
    let u = lines[r..]
        .iter()
        .position(|l| l.starts_with("G1 E2 "))
        .expect("remise de 2 mm");
    assert!(!lines[r..r + u].iter().any(|l| l.contains("E0.5")));

    for (src, code) in [
        (
            "let p = new_path();\np.set_temperature_offset(40);\np",
            "path.invalid_temperature_offset",
        ),
        (
            "let p = new_path();\np.set_retraction_length(9);\np",
            "path.invalid_retraction",
        ),
        (
            "let p = new_path();\np.set_retraction_speed(2);\np",
            "path.invalid_retraction",
        ),
        (
            "let p = new_path();\np.set_z_hop(3);\np",
            "path.invalid_retraction",
        ),
    ] {
        let ev = bench("calibration = true", src).evaluate(&Map::new());
        assert!(
            ev.issues.iter().any(|i| i.code == code),
            "{src}: {:?}",
            ev.issues
        );
    }
}
