//! Application de bureau FabWeave : une fenêtre qui affiche l'interface, le
//! moteur restant en WebAssembly comme dans la version web. Les préférences
//! et « Mes modèles » restent dans le stockage de la fenêtre, sur le poste.
//!
//! Ce que le bureau ajoute à la version web, par des commandes appelées
//! depuis l'interface (`ui/src/lib/desktop.ts`) :
//! - dialogues natifs Ouvrir, Enregistrer sous, Choisir un dossier ;
//! - lecture d'un fichier (ou de sa fin, pour un gros G-code) et écriture
//!   atomique d'un projet, pour ⌘S au même endroit et les fichiers récents ;
//! - dossier de modèles surveillé : ses `.weave` (sous-dossiers compris) sont
//!   envoyés à l'interface au choix du dossier, puis à chaque modification.

// Pas de console sous Windows pour l'application distribuée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use notify::{RecursiveMode, Watcher};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::webview::DownloadEvent;
use tauri::{AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

/// Au plus, dans le dossier surveillé : nombre de modèles, taille d'un
/// modèle, profondeur des sous-dossiers.
const MAX_MODELS: usize = 500;
const MAX_MODEL_SIZE: u64 = 2 * 1024 * 1024;
const MAX_DEPTH: usize = 8;
/// Regroupe les événements d'un enregistrement (écriture, renommage…).
const SETTLE: Duration = Duration::from_millis(250);

/// `dossier/nom.ext`, ou `dossier/nom (n).ext` si le fichier existe déjà : un
/// téléchargement vers un fichier existant échouerait.
fn free_path(dir: &Path, name: &Path) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let stem = name.file_stem().unwrap_or_default().to_string_lossy();
    let ext = name
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (1..)
        .map(|n| dir.join(format!("{stem} ({n}){ext}")))
        .find(|p| !p.exists())
        .expect("nom de fichier libre")
}

/// Fichier à ouvrir : projet, G-code ou paquet exporté, modèle, profil.
#[tauri::command]
async fn pick_open(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter(
            "FabWeave",
            &["json", "gcode", "3mf", "weave", "toml", "ini"],
        )
        .blocking_pick_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Emplacement d'un projet à enregistrer.
#[tauri::command]
async fn pick_save(app: AppHandle, name: String) -> Option<String> {
    app.dialog()
        .file()
        .set_file_name(name)
        .add_filter("FabWeave", &["json"])
        .blocking_save_file()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .blocking_pick_folder()
        .and_then(|p| p.into_path().ok())
        .map(|p| p.to_string_lossy().into_owned())
}

/// Contenu d'un fichier, ou ses `tail` derniers octets (projet embarqué à la
/// fin d'un G-code) ; octets bruts, sans passer par du JSON.
#[tauri::command]
fn read_file(path: String, tail: Option<u64>) -> Result<Response, String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
    if let Some(n) = tail {
        let len = f.metadata().map_err(|e| e.to_string())?.len();
        f.seek(SeekFrom::Start(len.saturating_sub(n)))
            .map_err(|e| e.to_string())?;
    }
    let mut out = Vec::new();
    f.read_to_end(&mut out).map_err(|e| e.to_string())?;
    Ok(Response::new(out))
}

/// Écrit un projet : fichier temporaire à côté, puis renommage, pour ne
/// jamais laisser un projet à moitié écrit.
#[tauri::command]
fn write_file(path: String, text: String) -> Result<(), String> {
    let target = PathBuf::from(&path);
    let name = target.file_name().ok_or("chemin sans nom de fichier")?;
    let tmp = target.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &target).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

#[derive(Serialize)]
struct WeaveFile {
    /// Chemin relatif au dossier, séparé par `/`.
    path: String,
    text: String,
}

/// `.weave` du dossier et de ses sous-dossiers (cachés exclus), triés.
fn scan(root: &Path) -> Result<Vec<WeaveFile>, String> {
    fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<WeaveFile>) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            if out.len() >= MAX_MODELS {
                break;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            let path = e.path();
            if meta.is_dir() {
                if depth < MAX_DEPTH {
                    walk(root, &path, depth + 1, out)?;
                }
            } else if name.to_lowercase().ends_with(".weave") && meta.len() <= MAX_MODEL_SIZE {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let rel = rel
                        .components()
                        .map(|c| c.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/");
                    out.push(WeaveFile { path: rel, text });
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, 0, &mut out).map_err(|e| e.to_string())?;
    Ok(out)
}

/// Surveillance en cours (remplacée au choix d'un autre dossier).
#[derive(Default)]
struct Folder(Mutex<Option<notify::RecommendedWatcher>>);

/// Envoie à l'interface le contenu du dossier, ou l'erreur qui l'empêche.
fn push(app: &AppHandle, root: &Path) {
    let detail = match scan(root) {
        Ok(files) => serde_json::json!({ "files": files }),
        Err(e) => serde_json::json!({ "error": e }),
    };
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval(format!(
            "window.dispatchEvent(new CustomEvent('fabweave:folder', {{ detail: {detail} }}))"
        ));
    }
}

/// Surveille `path` (ou plus rien) ; rend ses `.weave` du moment.
#[tauri::command]
fn watch_folder(app: AppHandle, folder: State<Folder>, path: Option<String>) -> Result<Vec<WeaveFile>, String> {
    let mut slot = folder.0.lock().map_err(|e| e.to_string())?;
    *slot = None;
    let Some(path) = path else { return Ok(Vec::new()) };
    let root = PathBuf::from(path);
    let files = scan(&root)?;
    // Un seul envoi pour une rafale d'événements.
    let pending = Arc::new(AtomicBool::new(false));
    let (app2, root2) = (app.clone(), root.clone());
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_err() || pending.swap(true, Ordering::SeqCst) {
            return;
        }
        let (app, root, pending) = (app2.clone(), root2.clone(), pending.clone());
        std::thread::spawn(move || {
            std::thread::sleep(SETTLE);
            pending.store(false, Ordering::SeqCst);
            push(&app, &root);
        });
    })
    .map_err(|e| e.to_string())?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;
    *slot = Some(watcher);
    Ok(files)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Folder::default())
        .invoke_handler(tauri::generate_handler![
            pick_open,
            pick_save,
            pick_folder,
            read_file,
            write_file,
            watch_folder
        ])
        .setup(|app| {
            let downloads = app.path().download_dir().ok();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("FabWeave")
                .inner_size(1440.0, 900.0)
                .min_inner_size(1024.0, 680.0)
                // Sans cela, Tauri capte les fichiers déposés et l'interface ne
                // reçoit plus le dépôt d'un `.weave`.
                .disable_drag_drop_handler()
                // G-code, images et profils : enregistrés dans Téléchargements.
                .on_download(move |_webview, event| {
                    if let DownloadEvent::Requested { destination, .. } = event {
                        if let (Some(dir), Some(name)) = (&downloads, destination.file_name()) {
                            *destination = free_path(dir, Path::new(name));
                        }
                    }
                    true
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("FabWeave n'a pas pu démarrer");
}
