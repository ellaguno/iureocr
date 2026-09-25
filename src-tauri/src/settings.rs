//! Ajustes persistentes (JSON en la carpeta de configuración de la app).

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Idiomas de Tesseract, unidos con `+` (p. ej. `spa+eng`).
    pub languages: String,
    /// Resolución a la que se rasterizan las páginas de un PDF.
    pub dpi: u32,
    /// Calidad JPEG de las páginas rasterizadas (las imágenes van dentro del PDF resultante).
    pub jpeg_quality: u8,
    /// "same" = junto al original, "custom" = `output_dir`.
    pub output_mode: String,
    pub output_dir: Option<String>,
    /// Sufijo del archivo resultante: `Escaneo<sufijo>.pdf`.
    pub suffix: String,
    /// No procesar PDF que ya tengan capa de texto (se puede forzar por archivo).
    pub skip_if_text: bool,
    /// Lanzar el OCR en cuanto se agrega un archivo (si no, queda abierto para verlo o editarlo).
    pub auto_ocr: bool,
    /// "system" | "light" | "dark"
    pub theme: String,
    /// Idioma de la interfaz: "auto" (el del sistema) | "en" | "es".
    pub ui_language: String,
    pub check_updates: bool,
    /// Cuenta de Iurefficient (compartida con las otras apps vía llavero y cuenta activa).
    pub iure_domain: String,
    pub iure_email: String,
    /// Contraseña de aplicación WebDAV, sólo si el llavero no está disponible.
    pub iure_app_password: String,
    pub iure_last_folder: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            languages: "spa+eng".into(),
            dpi: 300,
            jpeg_quality: 80,
            output_mode: "same".into(),
            output_dir: None,
            suffix: " - OCR".into(),
            skip_if_text: true,
            auto_ocr: false,
            theme: "system".into(),
            ui_language: "auto".into(),
            check_updates: true,
            iure_domain: String::new(),
            iure_email: String::new(),
            iure_app_password: String::new(),
            iure_last_folder: None,
        }
    }
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        let mut s = std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str::<Settings>(&t).ok())
            .unwrap_or_default();
        if s.languages.trim().is_empty() {
            s.languages = "spa+eng".into();
        }
        if !(72..=600).contains(&s.dpi) {
            s.dpi = 300;
        }
        if !(30..=100).contains(&s.jpeg_quality) {
            s.jpeg_quality = 80;
        }
        s
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ajustes_antiguos_sin_idioma_de_interfaz() {
        let dir = std::env::temp_dir().join("iureocr-settings-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        std::fs::write(&path, r#"{"languages":"spa","theme":"dark"}"#).unwrap();
        let s = Settings::load(&path);
        assert_eq!(s.ui_language, "auto");
        assert_eq!(s.languages, "spa");
        assert_eq!(s.theme, "dark");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
