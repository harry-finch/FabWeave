//! `fabweave check | stats | export` : codes de sortie et messages.

use std::process::Command;

fn fabweave(args: &[&str]) -> (i32, String) {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let out = Command::new(env!("CARGO_BIN_EXE_fabweave"))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    let text =
        String::from_utf8_lossy(&out.stdout).to_string() + &String::from_utf8_lossy(&out.stderr);
    (out.status.code().unwrap_or(-1), text)
}

#[test]
fn check_accepts_the_catalogue_and_rejects_errors() {
    let (code, out) = fabweave(&["check", "models/vases/ripple_vase.weave"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("OK : 29 calculs"), "{out}");

    let dir = std::env::temp_dir().join(format!("fabweave-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.weave");
    std::fs::write(&bad, "[meta]\nname = \"x\"\ncategory = \"y\"\n---\nlet p = new_path();\nlet r = params.radius;\np\n").unwrap();
    let (code, out) = fabweave(&["check", bad.to_str().unwrap()]);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains("ERREUR ligne 6") && out.contains("[script.unknown_property]"),
        "{out}"
    );

    let (code, out) = fabweave(&["check", bad.to_str().unwrap(), "--json"]);
    assert_eq!(code, 1);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["ok"], false);
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn export_writes_gcode_and_usage_errors_exit_2() {
    let dir = std::env::temp_dir().join(format!("fabweave-cli-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out_file = dir.join("vase.gcode");
    let (code, out) = fabweave(&[
        "export",
        "models/vases/ripple_vase.weave",
        "-o",
        out_file.to_str().unwrap(),
        "--printer",
        "profiles/printers/prusa_core_one.toml",
        "--set",
        "star_tips=6",
    ]);
    assert_eq!(code, 0, "{out}");
    let g = std::fs::read_to_string(&out_file).unwrap();
    assert!(g.contains("; param star_tips = 6") && g.contains("M862.3 P \"COREONE\""));
    std::fs::remove_dir_all(&dir).ok();

    let (code, out) = fabweave(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(out.contains("usage"));
}
