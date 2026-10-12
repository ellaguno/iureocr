//! Edición de contenido con el motor de PdfCraft (prueba): testado y numeración Bates.
//!
//! Igual que en `pdf.rs`, nunca se sobrescribe el original: el resultado se escribe como
//! archivo nuevo junto a él. Tras un testado se guarda siempre completo (no incremental),
//! porque un guardado incremental conservaría los bytes originales con lo testado dentro.

use crate::pdf::sibling_output;
use anyhow::{anyhow, Context, Result};
use chrono::Datelike;
use iurefficient_connect::tr;
use pdfcraft_cos::{write_full, Document, SaveOptions};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

/// Testa las zonas indicadas: quita texto, imágenes y trazos que caen debajo y pinta
/// recuadros negros. Si la verificación encuentra restos, falla sin escribir nada.
pub fn redact(input: &Path, areas: &[RedactArea]) -> Result<(PathBuf, RedactResult)> {
    if areas.is_empty() {
        return Err(anyhow!(tr!(
            "Mark at least one area to redact",
            "Marca al menos una zona para testar"
        )));
    }
    let mut doc = open(input)?;
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
    let output = sibling_output(input, &tr!(" - redacted", " - testado"));
    save(&doc, &output)?;
    let result = RedactResult {
        path: output.to_string_lossy().into_owned(),
        areas: report.marks,
        pages: report.pages,
        glyphs: report.glyphs,
        images: report.images_removed + report.images_cleared,
    };
    Ok((output, result))
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
pub fn bates(input: &Path, o: &BatesOptions) -> Result<(PathBuf, BatesResult)> {
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
    let output = sibling_output(input, " - Bates");
    save(&doc, &output)?;
    let result = BatesResult {
        path: output.to_string_lossy().into_owned(),
        pages: count,
        first: bates_label(o, o.start),
        last: bates_label(o, o.start + count as u64 - 1),
    };
    Ok((output, result))
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

    #[test]
    fn testado_quita_el_texto_y_bates_numera() {
        let dir = std::env::temp_dir().join(format!("iureocr-editar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let input = dir.join("doc.pdf");
        std::fs::write(&input, write_full(&pdf(0), &SaveOptions::default()).unwrap()).unwrap();

        // "SECRETO" está en y≈50 de 100, x 20–60 de 200.
        let (out, r) = redact(&input, &[RedactArea { page: 0, rect: [0.05, 0.4, 0.5, 0.6] }]).unwrap();
        assert_eq!((r.areas, r.glyphs), (1, 7));
        let bytes = std::fs::read(&out).unwrap();
        let doc = Document::open(Arc::new(bytes.clone())).unwrap();
        assert!(pdfcraft_redact::marks(&doc).is_empty());
        assert!(!bytes.windows(7).any(|w| w == b"SECRETO"));

        let o = BatesOptions { prefix: "EXP-".into(), suffix: String::new(), digits: 4, start: 7, position: 5, font_size: 9.0 };
        let (_, b) = bates(&out, &o).unwrap();
        assert_eq!((b.first.as_str(), b.last.as_str(), b.pages), ("EXP-0007", "EXP-0007", 1));
        std::fs::remove_dir_all(&dir).ok();
    }
}
