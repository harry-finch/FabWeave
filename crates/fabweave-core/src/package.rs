//! Paquet `.gcode.3mf` des imprimantes Bambu Lab (ADR-0005) : archive ZIP
//! au format 3MF qui porte le G-code de la plaque (`Metadata/plate_1.gcode`),
//! sa somme MD5 (que le firmware contrôle), les métadonnées de la plaque
//! (modèle d'imprimante, buse, durée, filament) et des vignettes PNG.
//!
//! Structure reprise de l'outil `bambox` (estampo, MIT) et des fichiers de
//! Bambu Studio. Pur et déterministe : dates du ZIP fixées, aucune horloge.

use md5::{Digest, Md5};

use crate::format::num;
use crate::path::{MoveKind, Toolpath};
use crate::plan::Plan;
use crate::profile::{FilamentProfile, PrinterProfile};

/// Version de Bambu Studio annoncée dans les métadonnées (celle de `bambox`,
/// acceptée par le firmware).
const CLIENT_VERSION: &str = "02.05.00.66";

/// Le paquet complet, prêt à écrire sur la carte SD.
pub fn bambu_3mf(
    gcode: &str,
    path: &Toolpath,
    plan: &Plan,
    printer: &PrinterProfile,
    filament: &FilamentProfile,
) -> Vec<u8> {
    let s = &plan.stats;
    let md5 = Md5::digest(gcode.as_bytes())
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<String>();
    let material = &filament.filament.material;
    let nozzle = num(printer.nozzle.diameter, 2);
    let slice_info = format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<config>
  <header>
    <header_item key="X-BBL-Client-Type" value="slicer"/>
    <header_item key="X-BBL-Client-Version" value="{CLIENT_VERSION}"/>
  </header>
  <plate>
    <metadata key="index" value="1"/>
    <metadata key="extruder_type" value="0"/>
    <metadata key="nozzle_volume_type" value="0"/>
    <metadata key="printer_model_id" value="{model}"/>
    <metadata key="nozzle_diameters" value="{nozzle}"/>
    <metadata key="timelapse_type" value="0"/>
    <metadata key="prediction" value="{time}"/>
    <metadata key="weight" value="{weight:.2}"/>
    <metadata key="outside" value="false"/>
    <metadata key="support_used" value="false"/>
    <metadata key="label_object_enabled" value="false"/>
    <metadata key="filament_maps" value="1"/>
    <metadata key="limit_filament_maps" value="0"/>
    <filament id="1" tray_info_idx="{tray}" type="{material}" color="#FFFFFF" used_m="{used_m:.2}" used_g="{weight:.2}" used_for_object="true" used_for_support="false" group_id="0" nozzle_diameter="{nozzle}" volume_type="Standard"/>
  </plate>
</config>
"##,
        model = xml(&printer.printer.model),
        time = s.time.round() as u64,
        weight = s.filament_mass,
        tray = generic_filament_id(material),
        material = xml(material),
        used_m = s.filament_length / 1000.0,
    );
    let plate_json = format!(
        r##"{{"bed_type":"{}","filament_colors":["#FFFFFF"],"filament_ids":[0],"first_extruder":0,"is_seq_print":false,"nozzle_diameter":{nozzle},"version":2}}"##,
        bed_type(&printer.bed.plate),
    );
    let model = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02" xmlns:BambuStudio="http://schemas.bambulab.com/package/2021">
 <metadata name="Application">FabWeave-{}</metadata>
 <metadata name="BambuStudio:3mfVersion">1</metadata>
 <resources>
 </resources>
 <build/>
</model>
"#,
        crate::version()
    );
    let big = thumbnail(path, plan, 512);
    let small = thumbnail(path, plan, 128);

    let mut z = Zip::default();
    z.add("[Content_Types].xml", CONTENT_TYPES.as_bytes());
    z.add("_rels/.rels", RELS.as_bytes());
    z.add("3D/3dmodel.model", model.as_bytes());
    z.add("Metadata/plate_1.gcode", gcode.as_bytes());
    z.add("Metadata/plate_1.gcode.md5", md5.as_bytes());
    z.add("Metadata/model_settings.config", MODEL_SETTINGS.as_bytes());
    z.add(
        "Metadata/_rels/model_settings.config.rels",
        MODEL_SETTINGS_RELS.as_bytes(),
    );
    z.add("Metadata/slice_info.config", slice_info.as_bytes());
    z.add("Metadata/plate_1.json", plate_json.as_bytes());
    z.add("Metadata/plate_1.png", &big);
    z.add("Metadata/plate_no_light_1.png", &big);
    z.add("Metadata/plate_1_small.png", &small);
    z.finish()
}

/// Identifiant de filament générique de Bambu Studio selon la matière.
fn generic_filament_id(material: &str) -> &'static str {
    match material {
        "PETG" => "GFG99",
        "ABS" => "GFB99",
        "ASA" => "GFB98",
        "PC" => "GFC99",
        "PA" | "PA-CF" => "GFN99",
        "TPU" | "FLEX" => "GFU99",
        _ => "GFL99",
    }
}

/// Plaque, au nom court de `plate_1.json`.
fn bed_type(plate: &str) -> &'static str {
    match plate {
        "Cool Plate" => "cool_plate",
        "Engineering Plate" => "eng_plate",
        "High Temp Plate" => "hot_plate",
        _ => "textured_plate",
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const CONTENT_TYPES: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
 <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
 <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/>
 <Default Extension="png" ContentType="image/png"/>
 <Default Extension="gcode" ContentType="text/x.gcode"/>
</Types>
"#;

const RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/3D/3dmodel.model" Id="rel-1" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/>
 <Relationship Target="/Metadata/plate_1.png" Id="rel-2" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail"/>
 <Relationship Target="/Metadata/plate_1.png" Id="rel-4" Type="http://schemas.bambulab.com/package/2021/cover-thumbnail-middle"/>
 <Relationship Target="/Metadata/plate_1_small.png" Id="rel-5" Type="http://schemas.bambulab.com/package/2021/cover-thumbnail-small"/>
</Relationships>
"#;

const MODEL_SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<config>
  <plate>
    <metadata key="plater_id" value="1"/>
    <metadata key="plater_name" value=""/>
    <metadata key="locked" value="false"/>
    <metadata key="filament_map_mode" value="Auto For Flush"/>
    <metadata key="filament_maps" value="1"/>
    <metadata key="gcode_file" value="Metadata/plate_1.gcode"/>
    <metadata key="thumbnail_file" value="Metadata/plate_1.png"/>
    <metadata key="thumbnail_no_light_file" value="Metadata/plate_no_light_1.png"/>
    <metadata key="pattern_bbox_file" value="Metadata/plate_1.json"/>
  </plate>
</config>
"#;

const MODEL_SETTINGS_RELS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
 <Relationship Target="/Metadata/plate_1.gcode" Id="rel-1" Type="http://schemas.bambulab.com/package/2021/gcode"/>
</Relationships>
"#;

// --- Vignette ---------------------------------------------------------------

/// Vignette PNG `size` × `size` du parcours imprimé, fond transparent, en
/// vue oblique (comme les vignettes du catalogue), teinte claire dégradée
/// selon la hauteur ; traits semi-transparents, dont le recouvrement dessine
/// le relief.
pub fn thumbnail(path: &Toolpath, plan: &Plan, size: usize) -> Vec<u8> {
    let pts = &path.points;
    let proj = |p: &crate::path::WayPoint| {
        (
            p.x * 0.91 + p.y * 0.41,
            -p.z * 0.8 + (p.y * 0.91 - p.x * 0.41) * 0.45,
        )
    };
    let mut lo = [f64::INFINITY; 3];
    let mut hi = [f64::NEG_INFINITY; 3];
    for p in pts {
        let (u, v) = proj(p);
        for (k, c) in [u, v, p.z].into_iter().enumerate() {
            lo[k] = lo[k].min(c);
            hi[k] = hi[k].max(c);
        }
    }
    let mut rgba = vec![0u8; size * size * 4];
    if pts.len() >= 2 && lo[0].is_finite() {
        let pad = 0.08 * size as f64;
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-6);
        let k = (size as f64 - 2.0 * pad) / span;
        let ou = pad + ((size as f64 - 2.0 * pad) - (hi[0] - lo[0]) * k) / 2.0 - lo[0] * k;
        let ov = pad + ((size as f64 - 2.0 * pad) - (hi[1] - lo[1]) * k) / 2.0 - lo[1] * k;
        let zspan = (hi[2] - lo[2]).max(1e-6);
        for (w, m) in pts.windows(2).zip(&plan.moves[1..]) {
            let (a, b) = (&w[0], &w[1]);
            if b.kind == MoveKind::Travel || m.skipped {
                continue;
            }
            let (ua, va) = proj(a);
            let (ub, vb) = proj(b);
            let t = ((b.z - lo[2]) / zspan).clamp(0.0, 1.0);
            let shade = (125.0 + 117.0 * t) as u8; // gris moyen en bas, presque blanc en haut
            segment(
                &mut rgba,
                size,
                (ou + ua * k, ov + va * k),
                (ou + ub * k, ov + vb * k),
                shade,
            );
        }
    }
    png(&rgba, size)
}

/// Segment en pas d'un pixel, mélangé à 45 % d'opacité.
fn segment(rgba: &mut [u8], size: usize, a: (f64, f64), b: (f64, f64), shade: u8) {
    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0) as usize;
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
        if x < 0.0 || y < 0.0 || x >= size as f64 || y >= size as f64 {
            continue;
        }
        let p = ((y as usize) * size + x as usize) * 4;
        let alpha = 0.45;
        let old_a = f64::from(rgba[p + 3]) / 255.0;
        let new_a = alpha + old_a * (1.0 - alpha);
        for c in 0..3 {
            let old = f64::from(rgba[p + c]) * old_a * (1.0 - alpha);
            rgba[p + c] = ((f64::from(shade) * alpha + old) / new_a).round() as u8;
        }
        rgba[p + 3] = (new_a * 255.0).round() as u8;
    }
}

/// PNG RVBA 8 bits, sans filtre de lignes, compressé (zlib).
fn png(rgba: &[u8], size: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + size);
    for row in rgba.chunks(size * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend((size as u32).to_be_bytes());
    ihdr.extend((size as u32).to_be_bytes());
    ihdr.extend([8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(
        &mut out,
        b"IDAT",
        &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6),
    );
    chunk(&mut out, b"IEND", &[]);
    out
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend((data.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let crc = crc32(&out[start..]);
    out.extend(crc.to_be_bytes());
}

// --- ZIP --------------------------------------------------------------------

/// CRC-32 (polynôme 0xEDB88320) du ZIP et du PNG.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in data {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320 & (crc & 1).wrapping_neg());
        }
    }
    !crc
}

/// Écriture ZIP minimale : fichiers compressés (deflate), date fixe
/// (1er janvier 1980) pour un résultat identique d'un export à l'autre.
#[derive(Default)]
struct Zip {
    data: Vec<u8>,
    central: Vec<u8>,
    count: u16,
}

impl Zip {
    fn add(&mut self, name: &str, content: &[u8]) {
        let packed = miniz_oxide::deflate::compress_to_vec(content, 6);
        let crc = crc32(content);
        let offset = self.data.len() as u32;
        let fields = |out: &mut Vec<u8>| {
            out.extend(20u16.to_le_bytes()); // version nécessaire
            out.extend(0u16.to_le_bytes()); // drapeaux
            out.extend(8u16.to_le_bytes()); // deflate
            out.extend(0u16.to_le_bytes()); // heure
            out.extend(0x21u16.to_le_bytes()); // date : 1980-01-01
            out.extend(crc.to_le_bytes());
            out.extend((packed.len() as u32).to_le_bytes());
            out.extend((content.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes()); // champ supplémentaire
        };
        self.data.extend(0x0403_4b50u32.to_le_bytes());
        fields(&mut self.data);
        self.data.extend_from_slice(name.as_bytes());
        self.data.extend_from_slice(&packed);

        self.central.extend(0x0201_4b50u32.to_le_bytes());
        self.central.extend(20u16.to_le_bytes()); // créé par
        fields(&mut self.central);
        self.central.extend(0u16.to_le_bytes()); // commentaire
        self.central.extend(0u16.to_le_bytes()); // disque
        self.central.extend(0u16.to_le_bytes()); // attributs internes
        self.central.extend(0u32.to_le_bytes()); // attributs externes
        self.central.extend(offset.to_le_bytes());
        self.central.extend_from_slice(name.as_bytes());
        self.count += 1;
    }

    fn finish(mut self) -> Vec<u8> {
        let (offset, len) = (self.data.len() as u32, self.central.len() as u32);
        self.data.append(&mut self.central);
        self.data.extend(0x0605_4b50u32.to_le_bytes());
        self.data.extend([0u8; 4]); // disques
        self.data.extend(self.count.to_le_bytes());
        self.data.extend(self.count.to_le_bytes());
        self.data.extend(len.to_le_bytes());
        self.data.extend(offset.to_le_bytes());
        self.data.extend(0u16.to_le_bytes()); // commentaire
        self.data
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Lit une archive écrite par `Zip` : (nom, contenu décompressé), en
    /// vérifiant chaque CRC.
    pub(crate) fn unzip(z: &[u8]) -> Vec<(String, Vec<u8>)> {
        let u16_at = |p: usize| u16::from_le_bytes([z[p], z[p + 1]]) as usize;
        let u32_at = |p: usize| u32::from_le_bytes([z[p], z[p + 1], z[p + 2], z[p + 3]]);
        let eocd = z.len() - 22;
        assert_eq!(u32_at(eocd), 0x0605_4b50);
        let (count, mut p) = (u16_at(eocd + 10), u32_at(eocd + 16) as usize);
        let mut out = Vec::new();
        for _ in 0..count {
            assert_eq!(u32_at(p), 0x0201_4b50);
            let (crc, packed, name_len) = (u32_at(p + 16), u32_at(p + 20) as usize, u16_at(p + 28));
            let local = u32_at(p + 42) as usize;
            let name = String::from_utf8(z[p + 46..p + 46 + name_len].to_vec()).unwrap();
            let start = local + 30 + u16_at(local + 26) + u16_at(local + 28);
            let data = miniz_oxide::inflate::decompress_to_vec(&z[start..start + packed]).unwrap();
            assert_eq!(crc32(&data), crc, "{name}");
            out.push((name, data));
            p += 46 + name_len;
        }
        out
    }

    #[test]
    fn x1e_package_carries_the_bambu_gcode() {
        let mut w = crate::Workbench::default();
        w.set_printer(include_str!("../../../profiles/printers/bambu_x1e.toml"))
            .unwrap();
        w.set_filament(include_str!("../../../profiles/filaments/generic_pla.toml"))
            .unwrap();
        w.load_model(include_str!("../../../models/mecanique/hex_adapter.weave"))
            .unwrap();
        let ev = w.evaluate(&serde_json::Map::new());
        let g = w.gcode(&ev).unwrap();

        // En-tête lu par le firmware en tête de fichier ; une couche = M73 L + M991.
        assert!(g.starts_with("; HEADER_BLOCK_START\n"));
        let layers = g.matches("\nM73 L").count();
        assert!(layers > 10);
        assert!(g.contains(&format!("; total layer number: {layers}\n")));
        assert_eq!(g.matches("M991 S0 P").count(), layers + 1); // + fin (P-1, indenté)
        assert!(g.contains("\nM73 P100 R0\n"));
        // Gabarits rendus : nivellement sur la zone de la première couche, plaque texturée.
        assert!(g.contains("G29 A X"));
        assert!(g.contains("G29.1 Z-0.04 ; for Textured PEI Plate"));
        assert!(g.contains("M106 P3 S255 ; Prevent PLA from jamming"));
        assert!(!g.contains('{') && !g.contains("[initial"));

        let z = w.gcode_3mf(&ev).unwrap();
        assert_eq!(z, w.gcode_3mf(&ev).unwrap(), "paquet déterministe");
        let files = unzip(&z);
        let get = |n: &str| &files.iter().find(|(k, _)| k == n).unwrap().1;
        assert_eq!(get("Metadata/plate_1.gcode"), g.as_bytes());
        let md5 = Md5::digest(g.as_bytes())
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<String>();
        assert_eq!(get("Metadata/plate_1.gcode.md5"), md5.as_bytes());
        let info = String::from_utf8(get("Metadata/slice_info.config").clone()).unwrap();
        assert!(info.contains(r#"key="printer_model_id" value="C13""#));
        assert!(info.contains(r#"type="PLA""#));
        assert_eq!(&get("Metadata/plate_1.png")[..4], b"\x89PNG");
    }

    #[test]
    fn crc32_matches_the_reference_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn zip_round_trips() {
        let mut z = Zip::default();
        z.add("a.txt", b"bonjour");
        z.add("dossier/b.bin", &[0u8; 1000]);
        let files = unzip(&z.finish());
        assert_eq!(files[0], ("a.txt".into(), b"bonjour".to_vec()));
        assert_eq!(files[1].0, "dossier/b.bin");
        assert_eq!(files[1].1.len(), 1000);
    }

    #[test]
    fn png_has_the_expected_header() {
        let p = png(&[255u8; 4 * 4 * 4], 4);
        assert_eq!(&p[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(&p[12..16], b"IHDR");
        assert_eq!(u32::from_be_bytes([p[16], p[17], p[18], p[19]]), 4);
        assert_eq!(&p[p.len() - 8..p.len() - 4], b"IEND");
    }
}
