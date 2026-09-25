//! Motor de OCR: rasteriza un PDF (o toma una imagen), se lo pasa a Tesseract y
//! deja un PDF con capa de texto buscable más un `.txt` con el texto.
//!
//! El PDF resultante lo genera el propio Tesseract (renderizador `pdf`): cada
//! página lleva la imagen rasterizada y, encima, el texto invisible. Es lo mismo
//! que hace ocrmypdf con `--force-ocr`. Para un escaneo, que es el caso que nos
//! ocupa, el original no tenía nada más que imágenes, así que no se pierde nada.
//! Un PDF con texto ya legible se omite (salvo que se fuerce): volver a
//! rasterizarlo sólo lo haría más pesado y menos nítido.

use crate::tesseract::{hidden_command, Tesseract};
use anyhow::{anyhow, Context, Result};
use iurefficient_connect::{lang, tr};
use serde::Serialize;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "bmp", "webp", "gif"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "pdf", "jpg", "jpeg", "png", "tif", "tiff", "bmp", "webp", "gif",
];

/// Un PDF con al menos estos caracteres en sus tres primeras páginas «ya tiene texto»
/// (mismo umbral que usa el servidor de Iurefficient en `check_if_needs_ocr`).
const TEXT_LAYER_MIN_CHARS: usize = 100;

pub struct Request {
    pub job_id: String,
    pub input: PathBuf,
    pub languages: String,
    pub dpi: u32,
    pub jpeg_quality: u8,
    pub output_dir: PathBuf,
    pub suffix: String,
    /// Procesar aunque el PDF ya tenga capa de texto.
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// `true` si se omitió porque el PDF ya tenía texto (no hay archivos de salida).
    pub skipped: bool,
    pub pdf_path: Option<String>,
    pub txt_path: Option<String>,
    pub pages: usize,
    pub chars: usize,
    pub elapsed_secs: f64,
    pub note: Option<String>,
}

/// Progreso: etapa ("render" | "ocr"), página actual y total.
pub type Progress<'a> = &'a (dyn Fn(&str, usize, usize) + Sync);
/// Callback de progreso que puede cruzar al hilo trabajador.
pub type ProgressBox = Box<dyn Fn(&str, usize, usize) + Send + Sync>;

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| IMAGE_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
        .unwrap_or(false)
}

fn is_pdf(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false)
}

/// Ruta libre en `dir` para `<stem><suffix>.<ext>` (añade « (2)», « (3)»… si existe).
fn free_path(dir: &Path, stem: &str, suffix: &str, ext: &str) -> PathBuf {
    let base = format!("{stem}{suffix}");
    let first = dir.join(format!("{base}.{ext}"));
    if !first.exists() {
        return first;
    }
    for n in 2..1000 {
        let p = dir.join(format!("{base} ({n}).{ext}"));
        if !p.exists() {
            return p;
        }
    }
    first
}

/// Error de cancelación. La interfaz lo reconoce por este texto (en cualquier idioma).
pub fn cancelled_message() -> &'static str {
    lang::pick("Cancelled", "Cancelado")
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(anyhow!(cancelled_message()));
    }
    Ok(())
}

/// Rasteriza las páginas de un PDF a JPEG en `dir`. Devuelve las rutas en orden y el
/// texto que ya tenían las tres primeras páginas (para decidir si se omite).
static PDFIUM: std::sync::OnceLock<pdfium_render::prelude::Pdfium> = std::sync::OnceLock::new();
/// Serializa la inicialización: dos hilos a la vez (OCR y miniaturas) intentarían
/// enlazar pdfium dos veces y el segundo fallaría con «already initialized».
static PDFIUM_INIT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// pdfium sólo se puede enlazar una vez por proceso: se crea a la primera llamada y
/// se comparte (función `thread_safe`) entre el OCR y las miniaturas.
pub fn pdfium(pdfium_dir: Option<&Path>) -> Result<&'static pdfium_render::prelude::Pdfium> {
    use pdfium_render::prelude::*;
    if let Some(p) = PDFIUM.get() {
        return Ok(p);
    }
    let _init = PDFIUM_INIT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(p) = PDFIUM.get() {
        return Ok(p);
    }
    let bindings = match pdfium_dir {
        Some(d) => Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(d))
            .or_else(|_| Pdfium::bind_to_system_library()),
        None => Pdfium::bind_to_system_library(),
    }
    .map_err(|e| {
        anyhow!(tr!(
            "Could not load pdfium (the PDF reading library): {e}",
            "No se pudo cargar pdfium (la biblioteca que lee PDF): {e}"
        ))
    })?;
    let _ = PDFIUM.set(Pdfium::new(bindings));
    Ok(PDFIUM.get().expect("pdfium recién inicializado"))
}

#[allow(clippy::too_many_arguments)]
fn rasterize(
    input: &Path,
    dir: &Path,
    dpi: u32,
    quality: u8,
    pdfium: &pdfium_render::prelude::Pdfium,
    force: bool,
    progress: Progress,
    cancel: &AtomicBool,
) -> Result<(Vec<PathBuf>, usize)> {
    use pdfium_render::prelude::*;

    let doc = pdfium.load_pdf_from_file(input, None).map_err(|e| {
        anyhow!(tr!(
            "Could not open the PDF: {e}",
            "No se pudo abrir el PDF: {e}"
        ))
    })?;
    let total = doc.pages().len() as usize;
    if total == 0 {
        return Err(anyhow!(tr!(
            "The PDF has no pages",
            "El PDF no tiene páginas"
        )));
    }

    // ¿Ya tiene texto? Se mira antes de rasterizar nada.
    let mut existing = 0usize;
    for page in doc.pages().iter().take(3) {
        if let Ok(t) = page.text() {
            existing += t.all().chars().filter(|c| !c.is_whitespace()).count();
        }
    }
    if existing >= TEXT_LAYER_MIN_CHARS && !force {
        return Ok((vec![], existing));
    }

    let mut files = Vec::with_capacity(total);
    for (i, page) in doc.pages().iter().enumerate() {
        check_cancel(cancel)?;
        let width_pt = page.width().value;
        let target_w = ((width_pt * dpi as f32) / 72.0).round().max(200.0) as i32;
        let cfg = PdfRenderConfig::new().set_target_width(target_w);
        let bitmap = page.render_with_config(&cfg).map_err(|e| {
            anyhow!(tr!(
                "Could not rasterize page {}: {e}",
                "No se pudo rasterizar la página {}: {e}",
                i + 1
            ))
        })?;
        let rgb = bitmap
            .as_image()
            .map_err(|e| {
                anyhow!(tr!(
                    "Could not convert page {}: {e}",
                    "No se pudo convertir la página {}: {e}",
                    i + 1
                ))
            })?
            .to_rgb8();
        let out = dir.join(format!("p{:05}.jpg", i + 1));
        let mut f = std::fs::File::create(&out)?;
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, quality);
        enc.encode_image(&rgb).map_err(|e| {
            anyhow!(tr!(
                "Could not save page {}: {e}",
                "No se pudo guardar la página {}: {e}",
                i + 1
            ))
        })?;
        files.push(out);
        progress("render", i + 1, total);
    }
    Ok((files, existing))
}

/// Ejecuta Tesseract sobre `inputs` (imágenes) y deja `<outbase>.pdf` y `<outbase>.txt`.
fn run_tesseract(
    tess: &Tesseract,
    inputs: &[PathBuf],
    outbase: &Path,
    languages: &str,
    dpi: u32,
    progress: Progress,
    cancel: &AtomicBool,
) -> Result<()> {
    let total = inputs.len();
    // Varias imágenes: Tesseract acepta un archivo de texto con una ruta por línea
    // y produce un solo PDF de varias páginas.
    let list_path = outbase.with_extension("lista.txt");
    let input_arg: PathBuf = if total == 1 {
        inputs[0].clone()
    } else {
        let list = inputs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&list_path, list + "\n")?;
        list_path.clone()
    };

    let mut cmd = hidden_command(Path::new(&tess.exe));
    cmd.arg(&input_arg)
        .arg(outbase)
        .arg("-l")
        .arg(languages)
        .arg("--dpi")
        .arg(dpi.to_string());
    if let Some(td) = &tess.tessdata {
        cmd.arg("--tessdata-dir").arg(td);
        // El pdf.ttf (fuente sin glifos de la capa de texto) vive junto a los modelos.
        cmd.env("TESSDATA_PREFIX", td);
    }
    // Los nombres «pdf» y «txt» son archivos de configuración de `tessdata/configs/`,
    // que no existen en la carpeta de modelos empaquetada; se piden como variables.
    cmd.args(["-c", "tessedit_create_pdf=1", "-c", "tessedit_create_txt=1"]);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    cmd.env("OMP_THREAD_LIMIT", "4");
    let mut child = cmd.spawn().with_context(|| {
        tr!(
            "Could not run Tesseract at {}",
            "No se pudo ejecutar Tesseract en {}",
            tess.exe
        )
    })?;

    let stderr = child.stderr.take();
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let reader = {
        let log = log.clone();
        let progress_pages = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pp = progress_pages.clone();
        std::thread::spawn(move || {
            if let Some(e) = stderr {
                for line in BufReader::new(e).lines().map_while(Result::ok) {
                    // "Page 3" (con lista) o "Page 1 of 3" según versión.
                    if let Some(rest) = line.trim().strip_prefix("Page ") {
                        if let Some(n) = rest
                            .split_whitespace()
                            .next()
                            .and_then(|s| s.parse::<usize>().ok())
                        {
                            pp.store(n, Ordering::Relaxed);
                        }
                    }
                    let mut l = log.lock().unwrap();
                    if l.len() < 8000 {
                        l.push_str(&line);
                        l.push('\n');
                    }
                }
            }
            progress_pages.load(Ordering::Relaxed)
        })
    };

    let mut last_reported = 0usize;
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            return Err(anyhow!(cancelled_message()));
        }
        match child.try_wait()? {
            Some(st) => break st,
            None => std::thread::sleep(Duration::from_millis(150)),
        }
        // Progreso aproximado por el número de páginas que Tesseract va anunciando.
        // (Un solo candado por vuelta: dos `lock()` en la misma expresión se bloquean
        // entre sí, porque el primer guardián vive hasta el final de la sentencia.)
        let done = {
            let l = log.lock().unwrap();
            l.matches("\nPage ").count() + usize::from(l.starts_with("Page "))
        };
        if done != last_reported {
            last_reported = done;
            progress("ocr", done.min(total), total);
        }
    };
    let _ = reader.join();
    let _ = std::fs::remove_file(&list_path);
    if !status.success() {
        let l = log.lock().unwrap();
        let tail: String = l
            .lines()
            .rev()
            .take(6)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join(" · ");
        return Err(anyhow!(tr!(
            "Tesseract failed ({status}). {tail}",
            "Tesseract terminó con error ({status}). {tail}"
        )));
    }
    progress("ocr", total, total);
    Ok(())
}

pub fn run(
    req: &Request,
    tess: &Tesseract,
    pdfium_dir: Option<&Path>,
    progress: Progress,
    cancel: &AtomicBool,
) -> Result<Outcome> {
    let started = Instant::now();
    if !req.input.is_file() {
        return Err(anyhow!(tr!(
            "File not found: {}",
            "No existe el archivo {}",
            req.input.display()
        )));
    }
    let stem = req
        .input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| lang::pick("document", "documento").into());
    std::fs::create_dir_all(&req.output_dir).with_context(|| {
        tr!(
            "Could not create the folder {}",
            "No se pudo crear la carpeta {}",
            req.output_dir.display()
        )
    })?;

    let work = std::env::temp_dir().join(format!("iureocr-{}", req.job_id));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let result = run_in(req, tess, pdfium_dir, &work, &stem, progress, cancel);
    let _ = std::fs::remove_dir_all(&work);
    let mut out = result?;
    out.elapsed_secs = started.elapsed().as_secs_f64();
    Ok(out)
}

fn run_in(
    req: &Request,
    tess: &Tesseract,
    pdfium_dir: Option<&Path>,
    work: &Path,
    stem: &str,
    progress: Progress,
    cancel: &AtomicBool,
) -> Result<Outcome> {
    let inputs: Vec<PathBuf> = if is_pdf(&req.input) {
        let (files, existing) = rasterize(
            &req.input,
            work,
            req.dpi,
            req.jpeg_quality,
            pdfium(pdfium_dir)?,
            req.force,
            progress,
            cancel,
        )?;
        if files.is_empty() {
            return Ok(Outcome {
                skipped: true,
                pdf_path: None,
                txt_path: None,
                pages: 0,
                chars: existing,
                elapsed_secs: 0.0,
                note: Some(tr!(
                    "The PDF already has a text layer; no OCR needed. You can force it from the file's options.",
                    "El PDF ya tiene capa de texto; no hace falta OCR. Puedes forzarlo desde el menú del archivo."
                )),
            });
        }
        files
    } else if is_image(&req.input) {
        vec![req.input.clone()]
    } else {
        return Err(anyhow!(tr!(
            "Unsupported format: only PDF and images (JPG, PNG, TIFF, BMP, WEBP)",
            "Formato no admitido: sólo PDF e imágenes (JPG, PNG, TIFF, BMP, WEBP)"
        )));
    };

    let outbase = work.join("salida");
    run_tesseract(
        tess,
        &inputs,
        &outbase,
        &req.languages,
        req.dpi,
        progress,
        cancel,
    )?;

    let pdf_tmp = outbase.with_extension("pdf");
    let txt_tmp = outbase.with_extension("txt");
    if !pdf_tmp.is_file() {
        return Err(anyhow!(tr!(
            "Tesseract did not produce the PDF",
            "Tesseract no produjo el PDF"
        )));
    }
    let text = std::fs::read_to_string(&txt_tmp).unwrap_or_default();
    let chars = text.chars().filter(|c| !c.is_whitespace()).count();

    let pdf_path = free_path(&req.output_dir, stem, &req.suffix, "pdf");
    let txt_path = pdf_path.with_extension("txt");
    move_file(&pdf_tmp, &pdf_path)?;
    std::fs::write(&txt_path, &text)?;

    let note = if chars < 40 {
        Some(tr!(
            "Very little text was recognized: it may be a blurry or handwritten photo, or in another language.",
            "Se reconoció muy poco texto: puede ser una foto borrosa, manuscrita o en otro idioma."
        ))
    } else {
        None
    };
    Ok(Outcome {
        skipped: false,
        pdf_path: Some(pdf_path.to_string_lossy().into_owned()),
        txt_path: Some(txt_path.to_string_lossy().into_owned()),
        pages: inputs.len(),
        chars,
        elapsed_secs: 0.0,
        note,
    })
}

/// Un trabajo para el hilo trabajador.
struct Job {
    req: Request,
    tess: Tesseract,
    pdfium_dir: Option<PathBuf>,
    progress: ProgressBox,
    cancel: std::sync::Arc<AtomicBool>,
    reply: tokio::sync::oneshot::Sender<Result<Outcome>>,
}

/// Hilo que ejecuta los OCR de uno en uno y conserva pdfium entre trabajos.
pub struct Worker {
    tx: std::sync::Mutex<std::sync::mpsc::Sender<Job>>,
}

impl Default for Worker {
    fn default() -> Self {
        Self::new()
    }
}

impl Worker {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("iureocr-worker".into())
            .spawn(move || {
                for job in rx {
                    let result = run(
                        &job.req,
                        &job.tess,
                        job.pdfium_dir.as_deref(),
                        &*job.progress,
                        &job.cancel,
                    );
                    let _ = job.reply.send(result);
                }
            })
            .expect("hilo de OCR");
        Self {
            tx: std::sync::Mutex::new(tx),
        }
    }

    pub async fn submit(
        &self,
        req: Request,
        tess: Tesseract,
        pdfium_dir: Option<PathBuf>,
        progress: ProgressBox,
        cancel: std::sync::Arc<AtomicBool>,
    ) -> Result<Outcome> {
        let (reply, rx) = tokio::sync::oneshot::channel();
        self.tx
            .lock()
            .unwrap()
            .send(Job {
                req,
                tess,
                pdfium_dir,
                progress,
                cancel,
                reply,
            })
            .map_err(|_| anyhow!(tr!("the OCR thread stopped", "el hilo de OCR se detuvo")))?;
        rx.await.map_err(|_| {
            anyhow!(tr!(
                "the OCR thread did not respond",
                "el hilo de OCR no respondió"
            ))
        })?
    }
}

/// `rename` y, si origen y destino están en discos distintos, copia y borra.
fn move_file(from: &Path, to: &Path) -> Result<()> {
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to)
        .with_context(|| tr!("Could not write {}", "No se pudo escribir {}", to.display()))?;
    let _ = std::fs::remove_file(from);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prueba de extremo a extremo con un PDF real: `IUREOCR_TEST_PDF=/ruta/escaneo.pdf cargo test -- --ignored`.
    #[test]
    #[ignore]
    fn reconoce_un_pdf_escaneado() {
        let Ok(pdf) = std::env::var("IUREOCR_TEST_PDF") else {
            return;
        };
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let tess = crate::tesseract::locate(Some(manifest)).expect("tesseract");
        let out_dir = std::env::temp_dir().join("iureocr-test-out");
        let _ = std::fs::remove_dir_all(&out_dir);
        let req = Request {
            job_id: "test".into(),
            input: PathBuf::from(&pdf),
            languages: "spa+eng".into(),
            dpi: 300,
            jpeg_quality: 80,
            output_dir: out_dir.clone(),
            suffix: " - OCR".into(),
            force: false,
        };
        let cancel = AtomicBool::new(false);
        let progress = |stage: &str, cur: usize, total: usize| eprintln!("{stage} {cur}/{total}");
        let pdfium_dir = manifest.join("resources").join("pdfium");
        let out = run(&req, &tess, Some(&pdfium_dir), &progress, &cancel).expect("ocr");
        eprintln!("{out:?}");
        assert!(
            !out.skipped,
            "el PDF de prueba no tiene texto y no debería omitirse"
        );
        assert_eq!(out.pages, 2);
        assert!(out.chars > 100, "muy poco texto reconocido: {}", out.chars);
        let txt = std::fs::read_to_string(out.txt_path.as_ref().unwrap()).unwrap();
        assert!(txt.contains("CONTRATO"), "no se reconoció el título: {txt}");
        // El PDF resultante tiene capa de texto: al volver a pasar, se omite.
        let req2 = Request {
            input: PathBuf::from(out.pdf_path.as_ref().unwrap()),
            job_id: "test2".into(),
            ..req
        };
        let again = run(&req2, &tess, Some(&pdfium_dir), &progress, &cancel).expect("ocr 2");
        assert!(
            again.skipped,
            "el PDF con OCR debería detectarse como «ya tiene texto»"
        );
    }
}
