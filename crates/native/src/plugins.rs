//! Desktop management for ArtCraft Store plug-ins: directory discovery,
//! scanning installed files, and atomic download/installation.

use std::path::PathBuf;

use artcraft_launcher_core::plugins::{InstalledPlugin, Plugin, PluginCatalog};

use crate::http::Http;

/// Maximum size for a single plug-in binary or package (100 MB).
const MAX_PLUGIN_DOWNLOAD: u64 = 100 << 20;

/// Resolves the system directory where plug-ins, scripts, or presets for `app` reside.
pub fn plugin_target_dir(app: &str, file_name: &str) -> Option<PathBuf> {
    let app_lower = app.to_lowercase();
    let is_jsx = file_name.ends_with(".jsx");
    let is_template = file_name.ends_with(".zip") || file_name.ends_with(".svg");
    let is_lut = file_name.ends_with(".cube");

    #[cfg(target_os = "macos")]
    {
        let data = dirs::data_dir()?;
        match app_lower.as_str() {
            "photocraft" => Some(data.join("PhotoCraft").join("Plugins")),
            "effectcraft" => {
                if is_jsx {
                    Some(data.join("EffectCraft").join("Scripts").join("ScriptUI Panels"))
                } else {
                    Some(data.join("EffectCraft").join("Plugins"))
                }
            }
            "vectorcraft" => {
                if is_template {
                    Some(data.join("VectorCraft").join("Templates"))
                } else {
                    Some(data.join("VectorCraft").join("Plugins"))
                }
            }
            "filmcraft" => {
                if is_lut {
                    Some(data.join("FilmCraft").join("LUTs").join("Creative"))
                } else {
                    Some(data.join("FilmCraft").join("Presets"))
                }
            }
            "lightcraft" => Some(data.join("LightCraft").join("Presets")),
            "soundcraft" => dirs::home_dir().map(|h| h.join("Library/Audio/Plug-Ins/CLAP")),
            "cadcraft" => Some(data.join("CADCraft").join("Scripts")),
            "pdfcraft" => Some(data.join("PdfCraft").join("Scripts")),
            "designcraft" => Some(data.join("DesignCraft").join("Templates")),
            _ => Some(data.join(app).join("Plugins")),
        }
    }

    #[cfg(target_os = "windows")]
    {
        let app_data = dirs::config_dir()?;
        match app_lower.as_str() {
            "photocraft" => Some(app_data.join("PhotoCraft").join("Plugins")),
            "effectcraft" => {
                if is_jsx {
                    Some(app_data.join("EffectCraft").join("Scripts").join("ScriptUI Panels"))
                } else {
                    Some(app_data.join("EffectCraft").join("Plugins"))
                }
            }
            "vectorcraft" => {
                if is_template {
                    Some(app_data.join("VectorCraft").join("Templates"))
                } else {
                    Some(app_data.join("VectorCraft").join("Plugins"))
                }
            }
            "filmcraft" => {
                if is_lut {
                    Some(app_data.join("FilmCraft").join("LUTs").join("Creative"))
                } else {
                    Some(app_data.join("FilmCraft").join("Presets"))
                }
            }
            "lightcraft" => Some(app_data.join("LightCraft").join("Presets")),
            "soundcraft" => Some(app_data.join("Common Files").join("CLAP")),
            "cadcraft" => Some(app_data.join("CADCraft").join("Scripts")),
            "pdfcraft" => Some(app_data.join("PdfCraft").join("Scripts")),
            "designcraft" => Some(app_data.join("DesignCraft").join("Templates")),
            _ => Some(app_data.join(app).join("Plugins")),
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let config = dirs::config_dir()?;
        match app_lower.as_str() {
            "photocraft" => Some(config.join("photocraft").join("plugins")),
            "effectcraft" => {
                if is_jsx {
                    Some(config.join("effectcraft").join("Scripts").join("ScriptUI Panels"))
                } else {
                    Some(config.join("effectcraft").join("plugins"))
                }
            }
            "vectorcraft" => {
                if is_template {
                    Some(config.join("vectorcraft").join("templates"))
                } else {
                    Some(config.join("vectorcraft").join("plugins"))
                }
            }
            "filmcraft" => {
                if is_lut {
                    Some(config.join("filmcraft").join("LUTs").join("Creative"))
                } else {
                    Some(config.join("filmcraft").join("Presets"))
                }
            }
            "lightcraft" => Some(config.join("lightcraft").join("presets")),
            "soundcraft" => dirs::home_dir().map(|h| h.join(".clap")).or_else(|| Some(config.join("soundcraft").join("plugins"))),
            "cadcraft" => Some(config.join("cadcraft").join("scripts")),
            "pdfcraft" => Some(config.join("pdfcraft").join("scripts")),
            "designcraft" => {
                dirs::document_dir().map(|d| d.join("DesignCraft").join("Templates")).or_else(|| Some(config.join("designcraft").join("templates")))
            }
            _ => Some(config.join(app).join("plugins")),
        }
    }
}

/// Computes the exact file path where `plugin` should be installed on disk.
pub fn plugin_file_path(plugin: &Plugin) -> Option<PathBuf> {
    let file_name = plugin.file_name()?;
    let dir = plugin_target_dir(&plugin.app, file_name)?;
    Some(dir.join(file_name))
}

/// Scans the local filesystem for plug-ins listed in `catalog`.
pub fn scan_installed_plugins(catalog: &PluginCatalog) -> Vec<InstalledPlugin> {
    let mut installed = Vec::new();
    for p in &catalog.plugins {
        if let Some(path) = plugin_file_path(p)
            && path.is_file()
        {
            let size_bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            installed.push(InstalledPlugin {
                id: p.id.clone(),
                app: p.app.clone(),
                file_name: p.file_name().unwrap_or("").to_string(),
                path: path.to_string_lossy().to_string(),
                size_bytes,
            });
        }
    }
    installed
}

/// Downloads and installs `plugin` to its designated application directory.
pub fn install_plugin(http: &Http, plugin: &Plugin) -> Result<InstalledPlugin, String> {
    let url = plugin.download_url().ok_or_else(|| "no download URL configured for this plug-in".to_string())?;
    let file_name = plugin.file_name().ok_or_else(|| "no artifact filename configured for this plug-in".to_string())?;
    let target_dir =
        plugin_target_dir(&plugin.app, file_name).ok_or_else(|| "could not determine installation directory on this system".to_string())?;
    let target_file = target_dir.join(file_name);

    let bytes = http.get_binary(&url, MAX_PLUGIN_DOWNLOAD)?;
    crate::paths::write_atomic(&target_file, &bytes)?;

    let size_bytes = bytes.len() as u64;
    Ok(InstalledPlugin {
        id: plugin.id.clone(),
        app: plugin.app.clone(),
        file_name: file_name.to_string(),
        path: target_file.to_string_lossy().to_string(),
        size_bytes,
    })
}

/// Uninstalls `plugin` by removing its file from the filesystem.
pub fn uninstall_plugin(plugin: &Plugin) -> Result<(), String> {
    let path = plugin_file_path(plugin).ok_or_else(|| "plug-in installation path unknown".to_string())?;
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|e| format!("couldn't remove {} ({e})", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_installation_directories() {
        let pc_dir = plugin_target_dir("photocraft", "test.wasm");
        assert!(pc_dir.is_some());
        let pc_path = pc_dir.unwrap();
        assert!(pc_path.to_string_lossy().to_lowercase().contains("photocraft"));

        let ec_jsx = plugin_target_dir("effectcraft", "Panel.jsx");
        assert!(ec_jsx.is_some());
        assert!(ec_jsx.unwrap().to_string_lossy().contains("ScriptUI"));

        let ec_wasm = plugin_target_dir("effectcraft", "effect.wasm");
        assert!(ec_wasm.is_some());
        assert!(ec_wasm.unwrap().to_string_lossy().to_lowercase().contains("plugins"));
    }
}
