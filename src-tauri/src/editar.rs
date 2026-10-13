//! Edición de contenido con el motor de PdfCraft: testado y numeración Bates.
//!
//! Cada operación lee un PDF y escribe otro (en la práctica, pasos de la sesión de edición
//! de `pdf.rs`). Tras un testado se guarda siempre completo (no incremental), porque un
//! guardado incremental conservaría los bytes originales con lo testado dentro.

use anyhow::{anyhow, Context, Result};
use chrono::Datelike;
use iurefficient_connect::tr;
use pdfcraft_cos::{write_full, Document, SaveOptions};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

fn open(path: &Path) -> Result<Document> {
    let bytes = std::fs::read(path)
        .with_context(|| tr!("Could not read {}", "No se pudo leer {}", path.display()))?;
    Document::open(Arc::new(bytes)).map_err(|e| {
        anyhow!(tr!(
            "Could not open the PDF: {e}",
            "No se pudo abrir el PDF: {e}"
        ))
    })
}

fn save(doc: &Document, output: &Path) -> Result<()> {
    let opts = SaveOptions {
        mod_date: Some(pdf_date(chrono::Local::now())),
        ..SaveOptions::default()
    };
    let bytes = write_full(doc, &opts).map_err(|e| {
        anyhow!(tr!(
            "Could not write the PDF: {e}",
            "No se pudo escribir el PDF: {e}"
        ))
    })?;
    std::fs::write(output, bytes)
        .with_context(|| tr!("Could not write {}", "No se pudo escribir {}", output.display()))
}

/// Fecha PDF (`D:AAAAMMDDHHmmSS+HH'mm'`).
fn pdf_date(t: chrono::DateTime<chrono::Local>) -> String {
    let off = t.offset().local_minus_utc();
    let sign = if off < 0 { '-' } else { '+' };
    let off = off.abs() / 60;
    format!(
        "D:{}{sign}{:02}'{:02}'",
        t.format("%Y%m%d%H%M%S"),
        off / 60,
        off % 60
    )
}

// ---------------------------------------------------------------------------
// Testado
// ---------------------------------------------------------------------------

/// Una zona a testar: página 0-based y rectángulo en fracciones de la página tal como se
/// ve (x0, y0, x1, y1 entre 0 y 1, con el origen arriba a la izquierda).
#[derive(Deserialize, Debug)]
pub struct RedactArea {
    pub page: usize,
    pub rect: [f64; 4],
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RedactResult {
    pub path: String,
    pub areas: usize,
    pub pages: usize,
    /// Caracteres de texto quitados (incluida la capa invisible del OCR).
    pub glyphs: usize,
    /// Imágenes quitadas o recortadas (en un escaneo, la propia página).
    pub images: usize,
}

/// Rectángulo de la página vista (fracciones, origen arriba a la izquierda) a espacio de
/// usuario del PDF, teniendo en cuenta CropBox, /Rotate y /UserUnit.
fn to_user_space(doc: &Document, page: &pdfcraft_model::Page, r: [f64; 4]) -> [f64; 4] {
    let (w, h) = page.display_size(doc);
    let [a, b, c, d, e, f] = page.view_matrix(doc);
    let map = |fx: f64, fy: f64| {
        // Espacio de visualización: puntos, origen abajo a la izquierda, y hacia arriba.
        let (x, y) = (fx.clamp(0.0, 1.0) * w, (1.0 - fy.clamp(0.0, 1.0)) * h);
        (a * x + c * y + e, b * x + d * y + f)
    };
    let (x0, y0) = map(r[0], r[1]);
    let (x1, y1) = map(r[2], r[3]);
    [x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1)]
}

/// Marca (privada) de las imágenes que eran JPEG y se descomprimieron para testarlas.
const WAS_JPEG: &[u8] = b"IureWasJPEG";

/// Imágenes JPEG (DCTDecode) dibujadas directamente por la página.
fn jpeg_images(doc: &Document, page: &pdfcraft_model::Page) -> Vec<pdfcraft_cos::ObjRef> {
    let Some(res) = page.dict.get(b"Resources").and_then(|r| doc.dict(r)) else {
        return Vec::new();
    };
    let Some(xo) = res.get(b"XObject").and_then(|x| doc.dict(x)) else {
        return Vec::new();
    };
    xo.iter()
        .filter_map(|(_, o)| o.as_ref())
        .filter(|r| match &*doc.get(*r) {
            pdfcraft_cos::Object::Stream(s) => {
                let dct = |n: &[u8]| n == b"DCTDecode" || n == b"DCT";
                s.dict.name(b"Subtype") == Some(b"Image")
                    && match s.dict.get(b"Filter") {
                        Some(pdfcraft_cos::Object::Name(n)) => dct(n),
                        Some(pdfcraft_cos::Object::Array(a)) => {
                            a.len() == 1 && a[0].as_name().is_some_and(dct)
                        }
                        _ => false,
                    }
            }
            _ => false,
        })
        .collect()
}

/// PdfCraft no decodifica JPEG y, si una marca toca uno, quita la imagen entera (en un
/// escaneo, la página completa). Las descomprimimos aquí para que borre sólo los píxeles
/// marcados. Devuelve las originales, para restaurar las que no se toquen.
fn unpack_jpegs(
    doc: &mut Document,
    pages: &[usize],
) -> Vec<(pdfcraft_cos::ObjRef, pdfcraft_cos::Stream)> {
    let all = pdfcraft_model::pages(doc);
    let mut refs: Vec<_> = pages
        .iter()
        .filter_map(|p| all.get(*p))
        .flat_map(|p| jpeg_images(doc, p))
        .collect();
    refs.sort_by_key(|r| (r.num, r.generation));
    refs.dedup();
    let mut originals = Vec::new();
    for r in refs {
        let pdfcraft_cos::Object::Stream(s) = &*doc.get(r) else { continue };
        // Un /Decode (JPEG CMYK de Adobe, invertido) cambiaría el sentido de los valores:
        // se deja como está y PdfCraft quitará la imagen entera, que es lo seguro.
        if s.dict.contains(b"Decode") {
            log::warn!("testado: JPEG {} con /Decode; se quitará entero", r.num);
            continue;
        }
        let img = match image::load_from_memory_with_format(&s.raw, image::ImageFormat::Jpeg) {
            Ok(i) => i,
            Err(e) => {
                log::warn!("testado: JPEG {} ilegible ({e}); se quitará entero", r.num);
                continue;
            }
        };
        let (cs, data, (w, h)) = match img.color().channel_count() {
            1 | 2 => {
                let g = img.to_luma8();
                ("DeviceGray", g.as_raw().clone(), g.dimensions())
            }
            _ => {
                let c = img.to_rgb8();
                ("DeviceRGB", c.as_raw().clone(), c.dimensions())
            }
        };
        let mut dict = s.dict.clone();
        dict.remove(b"Length");
        dict.set(b"Width".to_vec(), pdfcraft_cos::Object::Int(w as i64));
        dict.set(b"Height".to_vec(), pdfcraft_cos::Object::Int(h as i64));
        dict.set(b"BitsPerComponent".to_vec(), pdfcraft_cos::Object::Int(8));
        dict.set(b"ColorSpace".to_vec(), pdfcraft_cos::Object::name(cs));
        dict.set(WAS_JPEG.to_vec(), pdfcraft_cos::Object::Bool(true));
        originals.push((r, s.clone()));
        doc.set(r, pdfcraft_cos::Object::Stream(pdfcraft_cos::Stream::flate(dict, &data)));
    }
    originals
}

/// Tras testar: las imágenes descomprimidas que no se tocaron vuelven a ser el JPEG
/// original, y las copias con píxeles borrados se recomprimen en JPEG (los píxeles ya
/// están a cero, así que la compresión con pérdida no puede devolver nada de lo quitado).
fn repack_jpegs(
    doc: &mut Document,
    originals: Vec<(pdfcraft_cos::ObjRef, pdfcraft_cos::Stream)>,
) -> Result<()> {
    for (r, s) in originals {
        doc.set(r, pdfcraft_cos::Object::Stream(s));
    }
    let marked: Vec<_> = doc
        .scan_objects()
        .filter(|(_, o)| matches!(&**o, pdfcraft_cos::Object::Stream(s) if s.dict.contains(WAS_JPEG)))
        .map(|(r, _)| r)
        .collect();
    for r in marked {
        let pdfcraft_cos::Object::Stream(s) = &*doc.get(r) else { continue };
        let (w, h) = (
            s.dict.int(b"Width").unwrap_or(0) as u32,
            s.dict.int(b"Height").unwrap_or(0) as u32,
        );
        let (kind, n) = if s.dict.name(b"ColorSpace") == Some(b"DeviceGray") {
            (image::ExtendedColorType::L8, 1)
        } else {
            (image::ExtendedColorType::Rgb8, 3)
        };
        let data = s.decoded().map_err(|e| anyhow!("{e}"))?;
        if data.len() < (w * h * n) as usize {
            continue;
        }
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85)
            .encode(&data[..(w * h * n) as usize], w, h, kind)
            .context("JPEG")?;
        let mut dict = s.dict.clone();
        dict.remove(WAS_JPEG);
        dict.remove(b"Length");
        dict.remove(b"DecodeParms");
        dict.set(b"Filter".to_vec(), pdfcraft_cos::Object::name("DCTDecode"));
        doc.set(r, pdfcraft_cos::Object::Stream(pdfcraft_cos::Stream::from_raw(dict, jpeg)));
    }
    Ok(())
}

/// Testa las zonas indicadas: quita texto, imágenes y trazos que caen debajo y pinta
/// recuadros negros. Si la verificación encuentra restos, falla sin escribir nada.
pub fn redact(input: &Path, areas: &[RedactArea], output: &Path) -> Result<RedactResult> {
    if areas.is_empty() {
        return Err(anyhow!(tr!(
            "Mark at least one area to redact",
            "Marca al menos una zona para testar"
        )));
    }
    let mut doc = open(input)?;
    let mut marked: Vec<usize> = areas.iter().map(|a| a.page).collect();
    marked.sort_unstable();
    marked.dedup();
    let originals = unpack_jpegs(&mut doc, &marked);
    let pages = pdfcraft_model::pages(&doc);
    for area in areas {
        let page = pages.get(area.page).ok_or_else(|| {
            anyhow!(tr!(
                "Page {} does not exist",
                "La página {} no existe",
                area.page + 1
            ))
        })?;
        let rect = to_user_space(&doc, page, area.rect);
        if rect[2] - rect[0] < 0.5 || rect[3] - rect[1] < 0.5 {
            continue;
        }
        let shape = pdfcraft_annot::Shape::Redact {
            quads: vec![pdfcraft_annot::rect_quad(rect)],
            overlay: String::new(),
            look: Default::default(),
        };
        let style = pdfcraft_annot::Style::default_for(&shape);
        pdfcraft_annot::add_annotation(
            &mut doc,
            &pdfcraft_annot::NewAnnotation {
                page: area.page,
                shape,
                style,
                contents: String::new(),
                author: "IureOCR".into(),
            },
            &pdfcraft_annot::Meta::default(),
        )
        .map_err(|e| anyhow!("{e}"))?;
    }
    let report = pdfcraft_redact::apply(&mut doc, None).map_err(|e| {
        anyhow!(tr!(
            "Could not redact: {e}",
            "No se pudo testar: {e}"
        ))
    })?;
    repack_jpegs(&mut doc, originals)?;
    save(&doc, output)?;
    Ok(RedactResult {
        path: output.to_string_lossy().into_owned(),
        areas: report.marks,
        pages: report.pages,
        glyphs: report.glyphs,
        images: report.images_removed + report.images_cleared,
    })
}

// ---------------------------------------------------------------------------
// Numeración Bates
// ---------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BatesOptions {
    pub prefix: String,
    pub suffix: String,
    /// Cifras del número, con ceros a la izquierda (1–15).
    pub digits: usize,
    pub start: u64,
    /// 0–2: encabezado izquierda, centro, derecha; 3–5: pie izquierda, centro, derecha.
    pub position: usize,
    pub font_size: f64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BatesResult {
    pub path: String,
    pub pages: usize,
    pub first: String,
    pub last: String,
}

/// El número Bates tal como se imprime (para la vista previa y el resultado).
pub fn bates_label(o: &BatesOptions, n: u64) -> String {
    let digits = o.digits.clamp(1, 15);
    format!("{}{n:0digits$}{}", o.prefix, o.suffix)
}

/// Numera todas las páginas con un número Bates correlativo.
pub fn bates(input: &Path, o: &BatesOptions, output: &Path) -> Result<BatesResult> {
    // `#` y `>>` delimitan los campos de la plantilla de PdfCraft.
    if [&o.prefix, &o.suffix]
        .iter()
        .any(|s| s.contains('#') || s.contains(">>") || s.contains("<<"))
    {
        return Err(anyhow!(tr!(
            "The prefix and suffix cannot contain #, << or >>",
            "El prefijo y el sufijo no pueden llevar #, << ni >>"
        )));
    }
    let mut doc = open(input)?;
    let count = pdfcraft_model::pages(&doc).len();
    if count == 0 {
        return Err(anyhow!(tr!("The PDF has no pages", "El PDF no tiene páginas")));
    }
    let digits = o.digits.clamp(1, 15);
    let mut hf = pdfcraft_edit::HeaderFooter {
        font_size: if o.font_size.is_finite() && o.font_size > 0.0 {
            o.font_size.clamp(4.0, 48.0)
        } else {
            9.0
        },
        ..Default::default()
    };
    hf.text[o.position.min(5)] = format!(
        "<<Bates Number#{digits}#{}#{}#{}>>",
        o.start, o.prefix, o.suffix
    );
    let today = chrono::Local::now().date_naive();
    let cx = pdfcraft_edit::Context {
        date: (today.year() as i64, today.month(), today.day()),
    };
    let pages: Vec<usize> = (0..count).collect();
    pdfcraft_edit::add_header_footer(&mut doc, &pages, &hf, false, &cx).map_err(|e| {
        anyhow!(tr!(
            "Could not number the pages: {e}",
            "No se pudieron numerar las páginas: {e}"
        ))
    })?;
    save(&doc, output)?;
    Ok(BatesResult {
        path: output.to_string_lossy().into_owned(),
        pages: count,
        first: bates_label(o, o.start),
        last: bates_label(o, o.start + count as u64 - 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PDF mínimo de una página de 200×100 con la rotación dada y una línea de texto.
    fn pdf(rotate: i64) -> Document {
        let content = b"BT /F1 10 Tf 20 50 Td (SECRETO) Tj ET";
        let objs = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Rotate {rotate} /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>"),
            format!("<< /Length {} >>\nstream\n{}\nendstream", content.len(), String::from_utf8_lossy(content)),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offs = Vec::new();
        for (i, o) in objs.iter().enumerate() {
            offs.push(out.len());
            out.extend(format!("{} 0 obj\n{o}\nendobj\n", i + 1).bytes());
        }
        let x = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", objs.len() + 1).bytes());
        for o in offs {
            out.extend(format!("{o:010} 00000 n \n").bytes());
        }
        out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n", objs.len() + 1).bytes());
        Document::open(Arc::new(out)).unwrap()
    }

    fn user(rotate: i64, r: [f64; 4]) -> [f64; 4] {
        let doc = pdf(rotate);
        let page = &pdfcraft_model::pages(&doc)[0];
        to_user_space(&doc, page, r).map(|v| (v * 1000.0).round() / 1000.0)
    }

    #[test]
    fn rectangulo_sin_rotar() {
        // Cuarto superior izquierdo de la página vista = arriba a la izquierda en el PDF.
        assert_eq!(user(0, [0.0, 0.0, 0.5, 0.5]), [0.0, 50.0, 100.0, 100.0]);
    }

    #[test]
    fn rectangulo_en_pagina_girada() {
        // Girada 90° se ve de 100×200; su esquina superior izquierda es la inferior
        // izquierda del PDF sin girar.
        assert_eq!(user(90, [0.0, 0.0, 0.5, 0.5]), [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(user(270, [0.0, 0.0, 0.5, 0.5]), [100.0, 50.0, 200.0, 100.0]);
    }

    /// Página de 100×100 que es un escaneo JPEG gris claro de 200×200 px.
    fn escaneo_jpeg() -> Document {
        let img = image::GrayImage::from_pixel(200, 200, image::Luma([200]));
        let mut jpeg = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 90)
            .encode(img.as_raw(), 200, 200, image::ExtendedColorType::L8)
            .unwrap();
        let content = b"q 100 0 0 100 0 0 cm /Im0 Do Q";
        let mut out = b"%PDF-1.7\n".to_vec();
        let mut offs = Vec::new();
        let mut obj = |out: &mut Vec<u8>, body: &[u8]| {
            offs.push(out.len());
            out.extend(format!("{} 0 obj\n", offs.len()).bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        };
        obj(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>");
        obj(&mut out, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
        obj(&mut out, b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /XObject << /Im0 5 0 R >> >> >>");
        obj(&mut out, &[format!("<< /Length {} >>\nstream\n", content.len()).as_bytes(), content, b"\nendstream"].concat());
        obj(&mut out, &[format!("<< /Type /XObject /Subtype /Image /Width 200 /Height 200 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg.len()).as_bytes(), &jpeg, b"\nendstream"].concat());
        let x = out.len();
        out.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offs.len() + 1).bytes());
        for o in &offs {
            out.extend(format!("{o:010} 00000 n \n").bytes());
        }
        out.extend(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{x}\n%%EOF\n", offs.len() + 1).bytes());
        Document::open(Arc::new(out)).unwrap()
    }

    #[test]
    fn testar_un_escaneo_jpeg_borra_solo_la_zona() {
        let dir = std::env::temp_dir().join(format!("iureocr-jpeg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("escaneo.pdf");
        std::fs::write(&input, write_full(&escaneo_jpeg(), &SaveOptions::default()).unwrap()).unwrap();

        // Mitad superior izquierda de la página.
        let out = dir.join("testado.pdf");
        let r = redact(&input, &[RedactArea { page: 0, rect: [0.0, 0.0, 0.5, 0.5] }], &out).unwrap();
        assert_eq!(r.images, 1);
        let doc = open(&out).unwrap();
        let page = &pdfcraft_model::pages(&doc)[0];
        let imgs = jpeg_images(&doc, page);
        assert_eq!(imgs.len(), 1, "la página sigue teniendo su escaneo, en JPEG");
        let pdfcraft_cos::Object::Stream(s) = &*doc.get(imgs[0]) else { panic!() };
        assert!(!s.dict.contains(WAS_JPEG));
        let px = image::load_from_memory(&s.raw).unwrap().to_luma8();
        assert!(px.get_pixel(40, 40)[0] < 30, "bajo la marca: negro");
        assert!(px.get_pixel(160, 160)[0] > 170, "fuera de la marca: intacto");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn testado_quita_el_texto_y_bates_numera() {
        let dir = std::env::temp_dir().join(format!("iureocr-editar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("doc.pdf");
        std::fs::write(&input, write_full(&pdf(0), &SaveOptions::default()).unwrap()).unwrap();

        // "SECRETO" está en y≈50 de 100, x 20–60 de 200.
        let out = dir.join("testado.pdf");
        let r = redact(&input, &[RedactArea { page: 0, rect: [0.05, 0.4, 0.5, 0.6] }], &out).unwrap();
        assert_eq!((r.areas, r.glyphs), (1, 7));
        let bytes = std::fs::read(&out).unwrap();
        let doc = Document::open(Arc::new(bytes.clone())).unwrap();
        assert!(pdfcraft_redact::marks(&doc).is_empty());
        assert!(!bytes.windows(7).any(|w| w == b"SECRETO"));

        let o = BatesOptions { prefix: "EXP-".into(), suffix: String::new(), digits: 4, start: 7, position: 5, font_size: 9.0 };
        let b = bates(&out, &o, &dir.join("bates.pdf")).unwrap();
        assert_eq!((b.first.as_str(), b.last.as_str(), b.pages), ("EXP-0007", "EXP-0007", 1));
        std::fs::remove_dir_all(&dir).ok();
    }
}


