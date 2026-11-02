//! Application de bureau FabWeave : une fenêtre qui affiche l'interface, le
//! moteur restant en WebAssembly comme dans la version web. Les préférences
//! et « Mes modèles » restent dans le stockage de la fenêtre, sur le poste.

// Pas de console sous Windows pour l'application distribuée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use tauri::webview::DownloadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

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

fn main() {
    tauri::Builder::default()
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
