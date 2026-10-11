//! ArtCraft Store plug-in catalog and extension integration:
//! `GET https://akkk09.github.io/artcraft-store/catalog.json`.
//!
//! Community extensions, WASM filters, ScriptUI panels, LUTs and presets
//! for the Craft App Suite.

use serde::{Deserialize, Serialize};

/// The default ArtCraft Store catalog URL.
pub const STORE_CATALOG_URL: &str = "https://akkk09.github.io/artcraft-store/catalog.json";

/// The default ArtCraft Store website root.
pub const STORE_BASE_URL: &str = "https://akkk09.github.io/artcraft-store/";

/// The default ArtCraft Store downloads base directory.
pub const STORE_DOWNLOADS_URL: &str = "https://akkk09.github.io/artcraft-store/downloads/";

/// An application defined in the store catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreApp {
    pub id: String,
    pub name: String,
    #[serde(default, rename = "extensionTypes")]
    pub extension_types: Vec<String>,
}

/// A plug-in, filter, tool or template listing from the ArtCraft Store catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub compatibility: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default, rename = "sourceUrl")]
    pub source_url: Option<String>,
    #[serde(default, rename = "downloadUrl")]
    pub download_url: Option<String>,
    #[serde(default)]
    pub artifact: Option<String>,
    #[serde(default, rename = "releaseAsset")]
    pub release_asset: Option<String>,
    #[serde(default)]
    pub abi: Option<u32>,
}

impl Plugin {
    /// Resolves the direct download URL for the plugin.
    /// Prefers explicit `downloadUrl`, then resolves `releaseAsset` or `artifact`
    /// relative to the store's `downloads/` endpoint.
    pub fn download_url(&self) -> Option<String> {
        if let Some(url) = &self.download_url
            && !url.is_empty()
        {
            return Some(url.clone());
        }
        let artifact = self.file_name()?;
        if artifact.is_empty() {
            return None;
        }
        Some(format!("{STORE_DOWNLOADS_URL}{artifact}"))
    }

    /// The published filename of the artifact.
    pub fn file_name(&self) -> Option<&str> {
        if let Some(f) = self.release_asset.as_deref()
            && !f.is_empty()
        {
            return Some(f);
        }
        if let Some(f) = self.artifact.as_deref()
            && !f.is_empty()
        {
            return Some(f);
        }
        None
    }

    /// Returns the source repository link or store tree URL.
    pub fn source_link(&self) -> String {
        if let Some(url) = &self.source_url
            && !url.is_empty()
        {
            return url.clone();
        }
        if let Some(path) = &self.source
            && !path.is_empty()
        {
            return format!("https://github.com/akkk09/artcraft-store/tree/main/{path}");
        }
        "https://github.com/akkk09/artcraft-store".to_string()
    }
}

/// The complete ArtCraft Store catalog.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCatalog {
    #[serde(default, rename = "schemaVersion")]
    pub schema_version: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub apps: Vec<StoreApp>,
    #[serde(default)]
    pub plugins: Vec<Plugin>,
}

impl PluginCatalog {
    /// Parses the JSON catalog leniently. Returns `None` on invalid JSON.
    pub fn parse(json: &str) -> Option<Self> {
        serde_json::from_str(json).ok()
    }

    /// Finds a plugin by its unique ID.
    pub fn by_id(&self, id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    /// Returns all plugins compatible with the given app id (e.g., `photocraft`).
    pub fn for_app(&self, app_id: &str) -> Vec<&Plugin> {
        self.plugins.iter().filter(|p| p.app.eq_ignore_ascii_case(app_id)).collect()
    }
}

/// An installed plugin found on the local system.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub app: String,
    pub file_name: String,
    pub path: String,
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CATALOG: &str = r#"{
      "schemaVersion": 2,
      "name": "ArtCraft Store",
      "description": "Community extensions for ArtCraft creative applications.",
      "apps": [
        { "id": "photocraft", "name": "PhotoCraft", "extensionTypes": ["plugin", "filter"] },
        { "id": "effectcraft", "name": "EffectCraft", "extensionTypes": ["plugin", "tool"] }
      ],
      "plugins": [
        {
          "id": "org.photocraft.community.film-emulation",
          "name": "Film Emulation Toolkit",
          "version": "0.1.0",
          "author": "ArtCraft Store community",
          "description": "Applies compact film-inspired color looks with deterministic grain.",
          "kind": "filter",
          "abi": 1,
          "source": "plugins/film-emulation",
          "artifact": "photocraft_plugin_film_emulation.wasm",
          "releaseAsset": "photocraft_plugin_film_emulation.wasm",
          "tags": ["film", "grain"],
          "app": "photocraft",
          "compatibility": "PhotoCraft ABI v1"
        },
        {
          "id": "org.effectcraft.community.depthcraft",
          "name": "DepthCraft",
          "version": "1.0.0",
          "author": "Community",
          "kind": "plugin",
          "app": "effectcraft",
          "downloadUrl": "https://custom.example.com/depthcraft.wasm"
        }
      ]
    }"#;

    #[test]
    fn parses_catalog_and_filters_by_app() {
        let catalog = PluginCatalog::parse(SAMPLE_CATALOG).unwrap();
        assert_eq!(catalog.schema_version, 2);
        assert_eq!(catalog.name, "ArtCraft Store");
        assert_eq!(catalog.plugins.len(), 2);

        let photocraft_plugins = catalog.for_app("photocraft");
        assert_eq!(photocraft_plugins.len(), 1);
        assert_eq!(photocraft_plugins[0].name, "Film Emulation Toolkit");
        assert_eq!(
            photocraft_plugins[0].download_url(),
            Some("https://akkk09.github.io/artcraft-store/downloads/photocraft_plugin_film_emulation.wasm".to_string())
        );

        let effectcraft_plugins = catalog.for_app("effectcraft");
        assert_eq!(effectcraft_plugins.len(), 1);
        assert_eq!(effectcraft_plugins[0].name, "DepthCraft");
        assert_eq!(effectcraft_plugins[0].download_url(), Some("https://custom.example.com/depthcraft.wasm".to_string()));
    }

    #[test]
    fn handles_empty_or_malformed_json() {
        assert_eq!(PluginCatalog::parse(""), None);
        assert_eq!(PluginCatalog::parse("not json"), None);
        assert_eq!(PluginCatalog::parse("{}"), Some(PluginCatalog::default()));
    }
}
