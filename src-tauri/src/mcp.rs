//! Servidor MCP local: `IureOCR --mcp`.
//!
//! Claude Desktop, Claude Code, Copilot en VS Code o cualquier cliente MCP lanza este
//! mismo ejecutable con `--mcp` y le habla por stdin/stdout: JSON-RPC 2.0, un mensaje
//! por línea (transporte stdio de la spec). No se abre ventana ni se arranca Tauri:
//! sólo el motor de OCR y las herramientas de PDF.
//!
//! El reparto es el que hace útil esto: el OCR corre aquí, en el equipo, y el modelo
//! del cliente (el que el usuario ya paga) lee el texto y hace el resto. IureOCR no
//! necesita ningún LLM propio. Lo que sale de la máquina es sólo el texto que el
//! cliente decida leer.
//!
//! El protocolo (stdio, cancelación, progreso) lo pone `iurefficient_connect::mcp_server`;
//! aquí sólo están las herramientas. El registro va a stderr y a `iureocr-mcp.log`,
//! nunca a stdout. Los OCR se hacen de uno en uno.

use crate::{ocr, pdf, settings::Settings, tesseract};
use anyhow::{anyhow, Result};
use iurefficient_connect::mcp_server::{self, cut, Call, CallError, Handler, Reply, ServerInfo};
use iurefficient_connect::{lang, tr};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// Texto que se devuelve por omisión en una respuesta. Un documento largo se lee por
/// partes con `extract_text`.
const DEFAULT_MAX_CHARS: usize = 20_000;
const MAX_MAX_CHARS: usize = 200_000;

struct Server {
    settings: Settings,
    resource_dir: Option<PathBuf>,
    /// Un OCR a la vez: Tesseract ya usa varios hilos por su cuenta.
    ocr_lock: Mutex<()>,
    seq: AtomicU64,
}

pub fn serve(settings: Settings, resource_dir: Option<PathBuf>) -> i32 {
    mcp_server::serve(Server {
        settings,
        resource_dir,
        ocr_lock: Mutex::new(()),
        seq: AtomicU64::new(1),
    })
}

impl Handler for Server {
    fn info(&self) -> ServerInfo {
        ServerInfo {
            name: "iureocr",
            title: "IureOCR",
            version: env!("CARGO_PKG_VERSION"),
            instructions: lang::pick(
                "Local OCR (Tesseract) and PDF page tools. Everything runs on this computer. \
                 Paths must be absolute. Results are written as new files next to the original; \
                 originals are never modified. For long documents, read the text in parts with extract_text.",
                "OCR local (Tesseract) y herramientas de páginas PDF. Todo corre en este equipo. \
                 Las rutas deben ser absolutas. Los resultados se escriben como archivos nuevos junto al \
                 original; nunca se modifica el original. En documentos largos, lee el texto por partes con extract_text.",
            )
            .into(),
        }
    }

    fn tools(&self) -> Value {
        tools()
    }

    fn call(&self, name: &str, args: &Value, call: &Call) -> Result<Reply, CallError> {
        let data = match name {
            "ocr_file" => self.ocr_file(args, call)?,
            "extract_text" => self.extract_text(args)?,
            "pdf_info" => self.pdf_info(args)?,
            "render_page" => return Ok(Reply::Raw(self.render_page(args)?)),
            "edit_pdf_pages" => self.edit_pdf_pages(args)?,
            "ocr_languages" => self.ocr_languages()?,
            _ => return Err(CallError::Params(format!("unknown tool: {name}"))),
        };
        Ok(Reply::Data(data))
    }
}

impl Server {
    // ------------------------------------------------------------------ tools

    fn pdfium(&self) -> Result<&'static pdfium_render::prelude::Pdfium> {
        ocr::pdfium(crate::pdfium_dir_in(self.resource_dir.as_deref()).as_deref())
    }

    fn ocr_file(&self, args: &Value, call: &Call) -> Result<Value> {
        let input = input_path(args)?;
        let tess = tesseract::locate(self.resource_dir.as_deref())?;
        let languages = match str_arg(args, "languages") {
            Some(l) => {
                let missing: Vec<&str> = l
                    .split('+')
                    .filter(|x| !tess.languages.iter().any(|t| t == x))
                    .collect();
                if !missing.is_empty() {
                    return Err(anyhow!(tr!(
                        "Language not installed: {}. Available: {}",
                        "Idioma no instalado: {}. Disponibles: {}",
                        missing.join(", "),
                        tess.languages.join(", ")
                    )));
                }
                l.to_string()
            }
            None => self.settings.languages.clone(),
        };
        let output_dir = match str_arg(args, "output_dir") {
            Some(d) => absolute(d)?,
            None => crate::output_dir_for(&self.settings, &input),
        };
        let force = args.get("force").and_then(Value::as_bool).unwrap_or(false);
        let include_text = args
            .get("include_text")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let max_chars = max_chars(args);

        let req = ocr::Request {
            job_id: format!(
                "mcp-{}-{}",
                std::process::id(),
                self.seq.fetch_add(1, Ordering::Relaxed)
            ),
            input: input.clone(),
            languages,
            dpi: self.settings.dpi,
            jpeg_quality: self.settings.jpeg_quality,
            output_dir,
            suffix: self.settings.suffix.clone(),
            force,
        };
        // Rasterizar y reconocer cuentan como dos pasadas sobre las mismas páginas.
        let progress = |stage: &str, current: usize, total: usize| {
            let (done, message) = if stage == "render" {
                (
                    current,
                    tr!(
                        "Rasterizing page {current} of {total}",
                        "Rasterizando página {current} de {total}"
                    ),
                )
            } else {
                (
                    total + current,
                    tr!(
                        "Recognizing page {current} of {total}",
                        "Reconociendo página {current} de {total}"
                    ),
                )
            };
            call.progress(done as f64, Some((total * 2) as f64), &message);
        };
        let outcome = {
            let _one_at_a_time = self.ocr_lock.lock().unwrap_or_else(|e| e.into_inner());
            ocr::run(
                &req,
                &tess,
                crate::pdfium_dir_in(self.resource_dir.as_deref()).as_deref(),
                &progress,
                call.cancel,
            )?
        };

        let mut v = serde_json::to_value(&outcome)?;
        if outcome.skipped {
            // La nota de la app habla de su menú; aquí lo que existe es `force`.
            v["note"] = json!(tr!(
                "The PDF already has a text layer, so no OCR was needed; its text is returned. Use force: true to run OCR anyway.",
                "El PDF ya tiene capa de texto, así que no hizo falta OCR; se devuelve ese texto. Usa force: true para hacerlo de todos modos."
            ));
        }
        if include_text {
            // Si se omitió porque ya tenía texto, se devuelve ése: el cliente quería el texto.
            let text = match &outcome.txt_path {
                Some(p) => std::fs::read_to_string(p).unwrap_or_default(),
                None if outcome.skipped => pdf::page_texts(self.pdfium()?, &input, 1, usize::MAX)?
                    .1
                    .into_iter()
                    .map(|(_, t)| t)
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                None => String::new(),
            };
            let (text, truncated) = cut(&text, max_chars);
            v["text"] = json!(text);
            v["textTruncated"] = json!(truncated);
        }
        Ok(v)
    }

    fn extract_text(&self, args: &Value) -> Result<Value> {
        let input = input_path(args)?;
        let max_chars = max_chars(args);
        let ext = extension(&input);
        if ext == "txt" {
            let text = std::fs::read_to_string(&input)?;
            let (text, truncated) = cut(&text, max_chars);
            return Ok(json!({"path": input, "text": text, "truncated": truncated}));
        }
        if ext != "pdf" {
            return Err(anyhow!(tr!(
                "Only PDF and .txt files have text to extract; use ocr_file for images.",
                "Sólo los PDF y los .txt tienen texto que extraer; para imágenes usa ocr_file."
            )));
        }
        let from = u64_arg(args, "start_page").unwrap_or(1) as usize;
        let to = u64_arg(args, "end_page")
            .map(|n| n as usize)
            .unwrap_or(usize::MAX);
        let (total, pages) = pdf::page_texts(self.pdfium()?, &input, from, to)?;
        let mut text = String::new();
        let mut last = 0;
        let mut truncated = false;
        for (n, t) in &pages {
            let block = format!(
                "--- {} {n} ---\n{}\n\n",
                lang::pick("page", "página"),
                t.trim()
            );
            if text.chars().count() + block.chars().count() > max_chars && last > 0 {
                truncated = true;
                break;
            }
            text.push_str(&block);
            last = *n;
        }
        let chars = pages
            .iter()
            .map(|(_, t)| t.chars().filter(|c| !c.is_whitespace()).count())
            .sum::<usize>();
        let mut v = json!({
            "path": input,
            "totalPages": total,
            "fromPage": from.max(1).min(total),
            "toPage": last,
            "text": text,
            "truncated": truncated,
        });
        if truncated {
            v["next"] = json!(tr!(
                "Call again with start_page = {} to continue.",
                "Vuelve a llamar con start_page = {} para seguir.",
                last + 1
            ));
        }
        if chars == 0 {
            v["note"] = json!(tr!(
                "These pages have no text layer: it is probably a scan. Use ocr_file.",
                "Estas páginas no tienen capa de texto: seguramente es un escaneo. Usa ocr_file."
            ));
        }
        Ok(v)
    }

    fn pdf_info(&self, args: &Value) -> Result<Value> {
        let input = input_path(args)?;
        let size = std::fs::metadata(&input)?.len();
        if extension(&input) != "pdf" {
            let (w, h) = pdf::image_size(&input)?[0];
            return Ok(
                json!({"path": input, "kind": "image", "bytes": size, "widthPx": w, "heightPx": h,
                             "hasTextLayer": false}),
            );
        }
        let pdfium = self.pdfium()?;
        let sizes = pdf::page_sizes(pdfium, &input)?;
        let (_, first) = pdf::page_texts(pdfium, &input, 1, 3)?;
        let chars: usize = first
            .iter()
            .map(|(_, t)| t.chars().filter(|c| !c.is_whitespace()).count())
            .sum();
        Ok(json!({
            "path": input,
            "kind": "pdf",
            "bytes": size,
            "pages": sizes.len(),
            // Tamaño en puntos (1/72 de pulgada); carta = 612×792, A4 ≈ 595×842.
            "pageSizesPt": sizes.iter().take(50).map(|(w, h)| [w.round(), h.round()]).collect::<Vec<_>>(),
            "textCharsFirstPages": chars,
            "hasTextLayer": chars >= 100,
        }))
    }

    /// Devuelve la página como imagen para que el modelo la vea (firmas, sellos, tablas).
    fn render_page(&self, args: &Value) -> Result<Value> {
        let input = input_path(args)?;
        let page = u64_arg(args, "page").unwrap_or(1).max(1) as usize;
        let width = u64_arg(args, "width").unwrap_or(1000).clamp(200, 2000) as u32;
        let url = if extension(&input) == "pdf" {
            pdf::thumbnails(self.pdfium()?, &input, &[page - 1], width)?.remove(0)
        } else {
            pdf::image_thumbnail(&input, width)?
        };
        let data = url.split_once(',').map(|(_, d)| d).unwrap_or("");
        Ok(json!({
            "content": [{"type": "image", "data": data, "mimeType": "image/jpeg"}],
            "isError": false,
        }))
    }

    fn edit_pdf_pages(&self, args: &Value) -> Result<Value> {
        let input = input_path(args)?;
        let op = str_arg(args, "operation").unwrap_or("");
        let pages: Vec<u32> = args
            .get("pages")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_u64)
                    .map(|n| n as u32)
                    .collect()
            })
            .unwrap_or_default();
        let degrees = args.get("degrees").and_then(Value::as_i64);
        if let Some(d) = degrees {
            if ![90, 180, 270].contains(&d) {
                return Err(anyhow!(tr!(
                    "degrees must be 90, 180 or 270",
                    "degrees debe ser 90, 180 o 270"
                )));
            }
        }
        let (output, n) = pdf::edit(&input, op, &pages, degrees)?;
        log::info!(
            "MCP: PDF {op} {} → {} ({n} páginas)",
            input.display(),
            output.display()
        );
        Ok(json!({"path": output, "pages": n}))
    }

    fn ocr_languages(&self) -> Result<Value> {
        let t = tesseract::locate(self.resource_dir.as_deref())?;
        Ok(json!({
            "tesseract": t.version,
            "languages": t.languages,
            "default": self.settings.languages,
        }))
    }
}

// ------------------------------------------------------------------ argumentos

fn str_arg<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn u64_arg(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(Value::as_u64)
}

fn max_chars(args: &Value) -> usize {
    u64_arg(args, "max_chars")
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_CHARS)
        .min(MAX_MAX_CHARS)
}

/// Rutas absolutas y nada más: el proceso lo lanza el cliente con un directorio de
/// trabajo que nadie controla, y una ruta relativa acabaría en cualquier parte.
fn absolute(p: &str) -> Result<PathBuf> {
    let path = PathBuf::from(p);
    if !path.is_absolute() {
        return Err(anyhow!(tr!(
            "The path must be absolute: {p}",
            "La ruta debe ser absoluta: {p}"
        )));
    }
    Ok(path)
}

fn input_path(args: &Value) -> Result<PathBuf> {
    let p = str_arg(args, "path").ok_or_else(|| anyhow!(tr!("Missing path", "Falta path")))?;
    let path = absolute(p)?;
    if !path.is_file() {
        return Err(anyhow!(tr!(
            "File not found: {p}",
            "No existe el archivo {p}"
        )));
    }
    let ext = extension(&path);
    if ext != "txt" && !ocr::SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(anyhow!(tr!(
            "Unsupported format: only PDF, images (JPG, PNG, TIFF, BMP, WEBP) and .txt",
            "Formato no admitido: sólo PDF, imágenes (JPG, PNG, TIFF, BMP, WEBP) y .txt"
        )));
    }
    Ok(path)
}

fn extension(p: &Path) -> String {
    p.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

// ------------------------------------------------------------------ esquema

/// Los nombres son identificadores (en inglés, como los de cualquier servidor MCP);
/// las descripciones salen en el idioma de la interfaz, porque son lo que el modelo lee
/// para decidir cuándo usar cada herramienta.
fn tools() -> Value {
    let path = json!({"type": "string", "description": lang::pick(
        "Absolute path of the PDF or image.", "Ruta absoluta del PDF o la imagen.")});
    let read_only = json!({"readOnlyHint": true, "openWorldHint": false});
    // Escriben un archivo nuevo y nunca tocan el original.
    let writes_new = json!({"readOnlyHint": false, "destructiveHint": false, "idempotentHint": false, "openWorldHint": false});
    json!([
        {
            "name": "ocr_file",
            "title": lang::pick("OCR a scan", "OCR de un escaneo"),
            "description": lang::pick(
                "Runs OCR (Tesseract, on this computer) on a scanned PDF or an image. Writes a searchable PDF and a .txt next to the original (the original is not modified) and returns the recognized text. A PDF that already has a text layer is skipped unless force is true; its existing text is returned instead.",
                "Hace OCR (Tesseract, en este equipo) de un PDF escaneado o una imagen. Escribe un PDF buscable y un .txt junto al original (el original no se modifica) y devuelve el texto reconocido. Un PDF que ya tiene capa de texto se omite salvo que force sea true; en ese caso se devuelve el texto que ya tenía."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "languages": {"type": "string", "description": lang::pick(
                        "Tesseract languages joined with +, e.g. spa+eng. Defaults to the app setting. See ocr_languages.",
                        "Idiomas de Tesseract unidos con +, p. ej. spa+eng. Por omisión, el de los ajustes de la app. Ver ocr_languages.")},
                    "force": {"type": "boolean", "description": lang::pick(
                        "Run OCR even if the PDF already has text.", "Hacer OCR aunque el PDF ya tenga texto.")},
                    "output_dir": {"type": "string", "description": lang::pick(
                        "Absolute folder for the results. Defaults to the app setting (usually next to the original).",
                        "Carpeta absoluta para los resultados. Por omisión, la de los ajustes (normalmente junto al original).")},
                    "include_text": {"type": "boolean", "description": lang::pick(
                        "Return the text in the response (default true).", "Devolver el texto en la respuesta (por omisión true).")},
                    "max_chars": {"type": "integer", "description": lang::pick(
                        "Maximum characters of text to return (default 20000).", "Máximo de caracteres de texto a devolver (por omisión 20000).")},
                },
                "required": ["path"],
            },
            "annotations": writes_new,
        },
        {
            "name": "extract_text",
            "title": lang::pick("Read text", "Leer texto"),
            "description": lang::pick(
                "Reads the text layer of a PDF by page range, or a .txt file. Use it to read long documents in parts, or PDFs that already have text (no OCR needed). If the pages have no text, the PDF is a scan: use ocr_file.",
                "Lee la capa de texto de un PDF por rango de páginas, o un .txt. Úsala para leer documentos largos por partes o PDF que ya tienen texto (sin OCR). Si las páginas no tienen texto, el PDF es un escaneo: usa ocr_file."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": lang::pick("Absolute path of the PDF or .txt.", "Ruta absoluta del PDF o el .txt.")},
                    "start_page": {"type": "integer", "description": lang::pick("First page (1-based, default 1).", "Primera página (desde 1, por omisión 1).")},
                    "end_page": {"type": "integer", "description": lang::pick("Last page (default: the last one).", "Última página (por omisión, la última).")},
                    "max_chars": {"type": "integer", "description": lang::pick(
                        "Maximum characters to return (default 20000); the response says where to continue.",
                        "Máximo de caracteres (por omisión 20000); la respuesta dice desde dónde seguir.")},
                },
                "required": ["path"],
            },
            "annotations": read_only,
        },
        {
            "name": "pdf_info",
            "title": lang::pick("PDF info", "Datos del PDF"),
            "description": lang::pick(
                "Page count, page sizes and whether a PDF already has a text layer (that is, whether it needs OCR). Also works for images.",
                "Número de páginas, tamaños y si un PDF ya tiene capa de texto (es decir, si necesita OCR). También sirve para imágenes."),
            "inputSchema": {"type": "object", "properties": {"path": path}, "required": ["path"]},
            "annotations": read_only,
        },
        {
            "name": "render_page",
            "title": lang::pick("View a page", "Ver una página"),
            "description": lang::pick(
                "Returns a page of a PDF (or an image) as a JPEG so you can look at it: signatures, stamps, handwriting, tables or layout that OCR text does not capture.",
                "Devuelve una página de un PDF (o una imagen) como JPEG para mirarla: firmas, sellos, letra manuscrita, tablas o maquetación que el texto del OCR no recoge."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": path,
                    "page": {"type": "integer", "description": lang::pick("Page number (1-based, default 1).", "Número de página (desde 1, por omisión 1).")},
                    "width": {"type": "integer", "description": lang::pick("Width in pixels (200-2000, default 1000).", "Ancho en píxeles (200-2000, por omisión 1000).")},
                },
                "required": ["path"],
            },
            "annotations": read_only,
        },
        {
            "name": "edit_pdf_pages",
            "title": lang::pick("Edit PDF pages", "Editar páginas del PDF"),
            "description": lang::pick(
                "Removes, keeps, rotates or reorders pages of a PDF. Writes a new PDF next to the original and returns its path; the original is not modified.",
                "Quita, conserva, gira o reordena páginas de un PDF. Escribe un PDF nuevo junto al original y devuelve su ruta; el original no se modifica."),
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": lang::pick("Absolute path of the PDF.", "Ruta absoluta del PDF.")},
                    "operation": {"type": "string", "enum": ["delete", "keep", "rotate", "reorder"], "description": lang::pick(
                        "delete: remove the pages; keep: keep only those pages; rotate: rotate them clockwise; reorder: pages lists every page in the new order.",
                        "delete: quitar las páginas; keep: quedarse sólo con ellas; rotate: girarlas en sentido horario; reorder: pages trae todas las páginas en el orden nuevo.")},
                    "pages": {"type": "array", "items": {"type": "integer"}, "description": lang::pick("Page numbers (1-based).", "Números de página (desde 1).")},
                    "degrees": {"type": "integer", "enum": [90, 180, 270], "description": lang::pick("Only for rotate (default 90).", "Sólo para rotate (por omisión 90).")},
                },
                "required": ["path", "operation", "pages"],
            },
            "annotations": writes_new,
        },
        {
            "name": "ocr_languages",
            "title": lang::pick("OCR languages", "Idiomas del OCR"),
            "description": lang::pick(
                "Tesseract version and the OCR languages installed on this computer.",
                "Versión de Tesseract y los idiomas de OCR instalados en este equipo."),
            "inputSchema": {"type": "object", "properties": {}},
            "annotations": read_only,
        },
    ])
}
