//! Projet FabWeave (`.fabweave.json`) : tout ce qu'il faut pour rouvrir une
//! préparation à l'identique, sur ce poste ou un autre. Le modèle (source du
//! `.weave`) et les deux profils (TOML) y sont recopiés, avec les valeurs des
//! paramètres et la préparation (buse, placement, adhésion).
//!
//! Le même projet est embarqué à la fin de chaque G-code exporté, en
//! commentaires (JSON compressé en deflate puis en base64, lignes de 76
//! caractères), et rangé à part dans le paquet `.gcode.3mf` : déposer le
//! G-code ou le paquet rouvre la préparation.
//!
//! Format versionné (`format`, `version`) : une version plus récente que
//! celle du moteur est refusée avec un message, jamais lue à moitié.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::image::{base64, base64_decode};
use crate::issue::Issue;
use crate::setup::Setup;

pub const FORMAT: &str = "fabweave.project";
pub const VERSION: u32 = 1;

/// Bornes du bloc embarqué dans le G-code.
const BEGIN: &str = "; fabweave.project begin";
const END: &str = "; fabweave.project end";
/// Taille maximale d'un projet décompressé (octets) : bien au-delà d'un
/// modèle et de deux profils, assez peu pour refuser un fichier piégé.
pub const MAX_SIZE: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub format: String,
    pub version: u32,
    /// Nom donné par l'utilisateur (vide : sans titre).
    #[serde(default)]
    pub name: String,
    /// Source complète du modèle `.weave`.
    pub model: String,
    /// Valeurs de tous les paramètres au moment de l'enregistrement.
    #[serde(default)]
    pub values: IndexMap<String, Value>,
    /// Profils imprimante et filament (TOML), tels que choisis : la buse
    /// montée est dans `setup`.
    pub printer: String,
    pub filament: String,
    #[serde(default)]
    pub setup: Setup,
}

impl Project {
    pub fn new(
        name: &str,
        model: &str,
        values: IndexMap<String, Value>,
        printer: &str,
        filament: &str,
        setup: &Setup,
    ) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            name: name.into(),
            model: model.into(),
            values,
            printer: printer.into(),
            filament: filament.into(),
            setup: setup.clone(),
        }
    }

    /// Texte du fichier `.fabweave.json` (indenté, clés dans l'ordre).
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("projet sérialisable");
        s.push('\n');
        s
    }

    /// Lit un `.fabweave.json`.
    pub fn parse(text: &str) -> Result<Self, Vec<Issue>> {
        let v: Value = serde_json::from_str(text)
            .map_err(|e| vec![Issue::error("project.invalid").detail(e.to_string())])?;
        if v.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return Err(vec![Issue::error("project.wrong_format").field("format")]);
        }
        match v.get("version").and_then(Value::as_u64) {
            Some(n) if n > u64::from(VERSION) => {
                return Err(vec![Issue::error("project.unsupported_version")
                    .field("version")
                    .value(n as f64)])
            }
            Some(n) if n >= 1 => {}
            _ => return Err(vec![Issue::error("project.invalid").field("version")]),
        }
        serde_json::from_value(v)
            .map_err(|e| vec![Issue::error("project.invalid").detail(e.to_string())])
    }

    /// Bloc de commentaires ajouté à la fin du G-code.
    pub fn embed(&self) -> String {
        let json = serde_json::to_string(self).expect("projet sérialisable");
        let packed = miniz_oxide::deflate::compress_to_vec(json.as_bytes(), 9);
        let b64 = base64(&packed);
        let mut out = String::with_capacity(b64.len() * 80 / 76 + 128);
        out.push_str(BEGIN);
        out.push_str(" (deflate, base64)\n");
        for line in b64.as_bytes().chunks(76) {
            out.push_str("; ");
            out.push_str(std::str::from_utf8(line).expect("base64 ASCII"));
            out.push('\n');
        }
        out.push_str(END);
        out.push('\n');
        out
    }

    /// Projet embarqué dans un G-code (ou sa fin) ; `None` s'il n'y en a pas.
    pub fn extract(gcode: &str) -> Option<Result<Self, Vec<Issue>>> {
        let start = gcode.rfind(BEGIN)?;
        let body = &gcode[start..];
        let body = &body[body.find('\n')? + 1..];
        let Some(end) = body.find(END) else {
            return Some(Err(vec![Issue::error("project.truncated")]));
        };
        let b64: String = body[..end]
            .lines()
            .map(|l| l.trim().trim_start_matches(';').trim())
            .collect();
        let json = base64_decode(&b64)
            .and_then(|z| miniz_oxide::inflate::decompress_to_vec_with_limit(&z, MAX_SIZE).ok())
            .and_then(|j| String::from_utf8(j).ok());
        Some(match json {
            Some(j) => Self::parse(&j),
            None => Err(vec![Issue::error("project.invalid").field("gcode")]),
        })
    }

    /// Lit un projet quel que soit le fichier qui le porte : `.fabweave.json`,
    /// G-code exporté par FabWeave (ou sa fin), paquet `.gcode.3mf`.
    pub fn read(bytes: &[u8]) -> Result<Self, Vec<Issue>> {
        if bytes.starts_with(b"PK\x03\x04") {
            let json =
                crate::package::read_zip_entry(bytes, crate::package::PROJECT_ENTRY, MAX_SIZE)
                    .ok_or_else(|| vec![Issue::error("project.not_found")])?;
            let text =
                String::from_utf8(json).map_err(|_| vec![Issue::error("project.invalid")])?;
            return Self::parse(&text);
        }
        // Une fin de G-code coupée au milieu d'un caractère reste lisible.
        let text = String::from_utf8_lossy(bytes);
        if text.trim_start().starts_with('{') {
            return Self::parse(&text);
        }
        Self::extract(&text).unwrap_or_else(|| Err(vec![Issue::error("project.not_found")]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::{Adhesion, Placement};

    fn sample() -> Project {
        let mut values = IndexMap::new();
        values.insert("height".into(), serde_json::json!(80.0));
        values.insert("lobes".into(), serde_json::json!(6));
        values.insert("base".into(), serde_json::json!("rings"));
        let setup = Setup {
            placement: Placement {
                x: Some(100.0),
                rotation: 15.0,
                ..Placement::default()
            },
            adhesion: Adhesion::Brim {
                width: 4.0,
                gap: 0.1,
            },
            ..Setup::default()
        };
        Project::new(
            "Vase « bleu »",
            "[meta]\nname = \"v\"\n---\n",
            values,
            "[printer]\n",
            "[filament]\n",
            &setup,
        )
    }

    #[test]
    fn json_round_trips() {
        let p = sample();
        let text = p.to_json();
        assert!(text.starts_with("{\n  \"format\": \"fabweave.project\",\n  \"version\": 1,"));
        assert_eq!(Project::parse(&text).unwrap(), p);
        assert_eq!(Project::read(text.as_bytes()).unwrap(), p);
    }

    #[test]
    fn embedded_block_round_trips_from_the_tail() {
        let p = sample();
        let block = p.embed();
        assert!(block.lines().all(|l| l.starts_with("; ") && l.len() <= 78));
        let gcode = format!("G1 X1\nM84\n{block}");
        assert_eq!(Project::read(gcode.as_bytes()).unwrap(), p);
        // La fin du fichier suffit (lecture des derniers octets d'un gros G-code).
        let tail = &gcode.as_bytes()[gcode.find(BEGIN).unwrap()..];
        assert_eq!(Project::read(tail).unwrap(), p);
    }

    #[test]
    fn foreign_or_damaged_files_are_refused_with_a_code() {
        let code = |b: &[u8]| Project::read(b).unwrap_err()[0].code;
        assert_eq!(code(b"G28\nG1 X1\n"), "project.not_found");
        assert_eq!(
            code(br#"{"format":"fabvents.project","version":1}"#),
            "project.wrong_format"
        );
        assert_eq!(
            code(br#"{"format":"fabweave.project","version":9}"#),
            "project.unsupported_version"
        );
        assert_eq!(
            code(br#"{"format":"fabweave.project","version":1}"#),
            "project.invalid"
        );
        assert_eq!(code(b"{ pas du json"), "project.invalid");
        let block = sample().embed();
        let cut = &block[..block.len() / 2];
        assert_eq!(code(cut.as_bytes()), "project.truncated");
        let broken = block.replacen("\n; ", "\n; @@", 1);
        assert_eq!(code(broken.as_bytes()), "project.invalid");
        assert_eq!(code(b"PK\x03\x04 pas une archive"), "project.not_found");
    }
}
