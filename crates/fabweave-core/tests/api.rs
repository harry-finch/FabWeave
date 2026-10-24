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
    assert_eq!(pts.len(), 10);
    assert_eq!(pts[0].kind, MoveKind::Travel);
    // Cercle fermé : le dernier point revient au premier.
    assert!((pts[9].x - pts[1].x).abs() < 1e-12 && (pts[9].y - pts[1].y).abs() < 1e-12);
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
    let start = spec.find("```rhai\n").expect("exemple rhai") + 8;
    let end = start + spec[start..].find("```").unwrap();
    let ev = eval(&spec[start..end]);
    assert!(ev.issues.iter().all(|i| !i.is_error()), "{:?}", ev.issues);
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
