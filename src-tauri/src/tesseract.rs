//! Localización del ejecutable de Tesseract y de los modelos de idioma.
//!
//! Orden: la variable `IUREOCR_TESSERACT`, el Tesseract empaquetado con la app
//! (Windows), las rutas habituales de cada sistema y por último el `PATH`. Los
//! modelos empaquetados (`resources/tessdata`) se prefieren siempre que existan,
//! para que el resultado sea el mismo en las tres plataformas.

use anyhow::{anyhow, Result};
use iurefficient_connect::tr;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tesseract {
    pub exe: String,
    /// Carpeta de modelos que se pasa con `--tessdata-dir` (None = la del sistema).
    pub tessdata: Option<String>,
    pub version: String,
    pub languages: Vec<String>,
    /// `true` si es el Tesseract que viene dentro del instalador.
    pub bundled: bool,
}

/// `Command` sin ventana de consola en Windows.
pub fn hidden_command(program: &Path) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "tesseract.exe"
    } else {
        "tesseract"
    }
}

fn candidates(resource_dir: Option<&Path>) -> Vec<(PathBuf, bool)> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("IUREOCR_TESSERACT") {
        if !p.trim().is_empty() {
            v.push((PathBuf::from(p), false));
        }
    }
    if let Some(r) = resource_dir {
        v.push((r.join("resources").join("tesseract").join(exe_name()), true));
    }
    #[cfg(windows)]
    {
        for base in [
            "C:\\Program Files\\Tesseract-OCR",
            "C:\\Program Files (x86)\\Tesseract-OCR",
        ] {
            v.push((PathBuf::from(base).join("tesseract.exe"), false));
        }
        if let Some(local) = dirs::data_local_dir() {
            v.push((
                local
                    .join("Programs")
                    .join("Tesseract-OCR")
                    .join("tesseract.exe"),
                false,
            ));
        }
    }
    #[cfg(target_os = "macos")]
    {
        v.push((PathBuf::from("/opt/homebrew/bin/tesseract"), false));
        v.push((PathBuf::from("/usr/local/bin/tesseract"), false));
        v.push((PathBuf::from("/opt/local/bin/tesseract"), false));
    }
    #[cfg(target_os = "linux")]
    {
        v.push((PathBuf::from("/usr/bin/tesseract"), false));
        v.push((PathBuf::from("/usr/local/bin/tesseract"), false));
    }
    v.push((PathBuf::from(exe_name()), false)); // PATH
    v
}

fn bundled_tessdata(resource_dir: Option<&Path>) -> Option<PathBuf> {
    let dir = resource_dir?.join("resources").join("tessdata");
    if dir.join("spa.traineddata").is_file() {
        Some(dir)
    } else {
        None
    }
}

fn version_of(exe: &Path) -> Option<String> {
    let out = hidden_command(exe).arg("--version").output().ok()?;
    if !out.status.success() && out.stdout.is_empty() {
        return None;
    }
    // "tesseract 5.3.4" en la primera línea (stdout o stderr según versión).
    let text = String::from_utf8_lossy(if out.stdout.is_empty() {
        &out.stderr
    } else {
        &out.stdout
    })
    .into_owned();
    let first = text.lines().next()?.trim();
    Some(
        first
            .strip_prefix("tesseract ")
            .unwrap_or(first)
            .to_string(),
    )
}

fn languages_of(exe: &Path, tessdata: Option<&Path>) -> Vec<String> {
    let mut cmd = hidden_command(exe);
    cmd.arg("--list-langs");
    if let Some(d) = tessdata {
        cmd.arg("--tessdata-dir").arg(d);
    }
    let Ok(out) = cmd.output() else { return vec![] };
    let text =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("List of") && !l.contains(' '))
        .map(str::to_string)
        .collect()
}

/// Busca Tesseract y comprueba que responde.
pub fn locate(resource_dir: Option<&Path>) -> Result<Tesseract> {
    let tessdata = bundled_tessdata(resource_dir);
    for (exe, bundled) in candidates(resource_dir) {
        if exe.components().count() > 1 && !exe.is_file() {
            continue;
        }
        if let Some(version) = version_of(&exe) {
            let languages = languages_of(&exe, tessdata.as_deref());
            return Ok(Tesseract {
                exe: exe.to_string_lossy().into_owned(),
                tessdata: tessdata.as_ref().map(|d| d.to_string_lossy().into_owned()),
                version,
                languages,
                bundled,
            });
        }
    }
    Err(anyhow!(tr!(
        "Tesseract was not found. On Linux: sudo apt install tesseract-ocr tesseract-ocr-spa tesseract-ocr-eng. On macOS: brew install tesseract tesseract-lang. On Windows it comes with the installer.",
        "No se encontró Tesseract. En Linux: sudo apt install tesseract-ocr tesseract-ocr-spa tesseract-ocr-eng. En macOS: brew install tesseract tesseract-lang. En Windows viene dentro del instalador."
    )))
}
