//! Páginas de un PDF: recuento, miniaturas (pdfium) y edición básica (lopdf):
//! quitar páginas, quedarse con algunas y rotar. Nunca se sobrescribe el
//! original: el resultado se escribe como archivo nuevo.

use anyhow::{anyhow, Context, Result};
use base64::Engine;
use iurefficient_connect::{lang, tr};
use std::path::{Path, PathBuf};

/// Número de páginas de un PDF.
pub fn page_count(pdfium: &pdfium_render::prelude::Pdfium, path: &Path) -> Result<usize> {
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|e| {
        anyhow!(tr!(
            "Could not open the PDF: {e}",
            "No se pudo abrir el PDF: {e}"
        ))
    })?;
    Ok(doc.pages().len() as usize)
}

/// Tamaño (ancho, alto) en puntos de cada página, ya con su rotación aplicada.
pub fn page_sizes(pdfium: &pdfium_render::prelude::Pdfium, path: &Path) -> Result<Vec<(f32, f32)>> {
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|e| {
        anyhow!(tr!(
            "Could not open the PDF: {e}",
            "No se pudo abrir el PDF: {e}"
        ))
    })?;
    Ok(doc
        .pages()
        .iter()
        .map(|p| (p.width().value, p.height().value))
        .collect())
}

/// Tamaño en píxeles de un archivo de imagen, como una sola página.
pub fn image_size(path: &Path) -> Result<Vec<(f32, f32)>> {
    let (w, h) = image::image_dimensions(path).with_context(|| {
        tr!(
            "Could not read the image {}",
            "No se pudo leer la imagen {}",
            path.display()
        )
    })?;
    Ok(vec![(w as f32, h as f32)])
}

fn jpeg_data_url(rgb: &image::RgbImage, quality: u8) -> Result<String> {
    let mut buf = Vec::new();
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    enc.encode_image(rgb)?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&buf)
    ))
}

/// Miniaturas (JPEG en data URL) de las páginas `indices` (0-based) a `width` píxeles.
pub fn thumbnails(
    pdfium: &pdfium_render::prelude::Pdfium,
    path: &Path,
    indices: &[usize],
    width: u32,
) -> Result<Vec<String>> {
    use pdfium_render::prelude::*;
    let doc = pdfium.load_pdf_from_file(path, None).map_err(|e| {
        anyhow!(tr!(
            "Could not open the PDF: {e}",
            "No se pudo abrir el PDF: {e}"
        ))
    })?;
    let pages = doc.pages();
    let mut out = Vec::with_capacity(indices.len());
    for &i in indices {
        let page = pages
            .get(i as pdfium_render::prelude::PdfPageIndex)
            .map_err(|e| {
                anyhow!(tr!(
                    "Page {} out of range: {e}",
                    "Página {} fuera de rango: {e}",
                    i + 1
                ))
            })?;
        let cfg = PdfRenderConfig::new().set_target_width(width as i32);
        let bitmap = page.render_with_config(&cfg).map_err(|e| {
            anyhow!(tr!(
                "Could not render page {}: {e}",
                "No se pudo dibujar la página {}: {e}",
                i + 1
            ))
        })?;
        let rgb = bitmap
            .as_image()
            .map_err(|e| anyhow!(tr!("Page {}: {e}", "Página {}: {e}", i + 1)))?
            .to_rgb8();
        out.push(jpeg_data_url(&rgb, quality_for(width))?);
    }
    Ok(out)
}

/// Las miniaturas pequeñas aguantan más compresión; las del visor, no.
fn quality_for(width: u32) -> u8 {
    if width > 400 {
        85
    } else {
        70
    }
}

/// Miniatura de un archivo de imagen (JPG, PNG, TIFF…).
pub fn image_thumbnail(path: &Path, width: u32) -> Result<String> {
    let img = image::open(path).with_context(|| {
        tr!(
            "Could not read the image {}",
            "No se pudo leer la imagen {}",
            path.display()
        )
    })?;
    let small = if img.width() > width {
        img.thumbnail(width, width * 3)
    } else {
        img
    };
    jpeg_data_url(&small.to_rgb8(), quality_for(width))
}

fn load(path: &Path) -> Result<lopdf::Document> {
    let mut doc = lopdf::Document::load(path)
        .with_context(|| tr!("Could not read {}", "No se pudo leer {}", path.display()))?;
    if doc.is_encrypted() {
        doc.decrypt("").map_err(|_| {
            anyhow!(tr!(
                "The PDF is password-protected; remove the password before editing it",
                "El PDF está protegido con contraseña; quítala antes de editarlo"
            ))
        })?;
    }
    Ok(doc)
}

/// Ruta libre `<stem><suffix>.pdf` junto al original (o « (2)», « (3)»… si existe).
pub fn sibling_output(input: &Path, suffix: &str) -> PathBuf {
    let dir = input
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| lang::pick("document", "documento").into());
    let first = dir.join(format!("{stem}{suffix}.pdf"));
    if !first.exists() {
        return first;
    }
    for n in 2..1000 {
        let p = dir.join(format!("{stem}{suffix} ({n}).pdf"));
        if !p.exists() {
            return p;
        }
    }
    first
}

fn finish(mut doc: lopdf::Document, output: &Path) -> Result<()> {
    doc.prune_objects();
    doc.renumber_objects();
    doc.compress();
    doc.save(output).with_context(|| {
        tr!(
            "Could not write {}",
            "No se pudo escribir {}",
            output.display()
        )
    })?;
    Ok(())
}

/// Quita las páginas indicadas (1-based). Devuelve cuántas quedan.
pub fn delete_pages(input: &Path, pages: &[u32], output: &Path) -> Result<usize> {
    let mut doc = load(input)?;
    let total = doc.get_pages().len();
    let valid: Vec<u32> = pages
        .iter()
        .copied()
        .filter(|p| *p >= 1 && (*p as usize) <= total)
        .collect();
    if valid.len() >= total {
        return Err(anyhow!(tr!(
            "You cannot remove every page",
            "No se pueden quitar todas las páginas"
        )));
    }
    if valid.is_empty() {
        return Err(anyhow!(tr!(
            "There are no pages to remove",
            "No hay páginas que quitar"
        )));
    }
    doc.delete_pages(&valid);
    finish(doc, output)?;
    Ok(total - valid.len())
}

/// Conserva sólo las páginas indicadas (1-based), en su orden original.
pub fn keep_pages(input: &Path, pages: &[u32], output: &Path) -> Result<usize> {
    let doc = load(input)?;
    let total = doc.get_pages().len();
    let keep: std::collections::BTreeSet<u32> = pages
        .iter()
        .copied()
        .filter(|p| *p >= 1 && (*p as usize) <= total)
        .collect();
    if keep.is_empty() {
        return Err(anyhow!(tr!(
            "Choose at least one page",
            "Elige al menos una página"
        )));
    }
    let remove: Vec<u32> = (1..=total as u32).filter(|p| !keep.contains(p)).collect();
    if remove.is_empty() {
        return Err(anyhow!(tr!(
            "All pages are already included",
            "Ya están todas las páginas"
        )));
    }
    let mut doc = doc;
    doc.delete_pages(&remove);
    finish(doc, output)?;
    Ok(keep.len())
}

/// Gira las páginas indicadas (1-based) `degrees` grados en sentido horario (90, 180, 270).
pub fn rotate_pages(input: &Path, pages: &[u32], degrees: i64, output: &Path) -> Result<usize> {
    let mut doc = load(input)?;
    let ids = doc.get_pages();
    let delta = degrees.rem_euclid(360);
    let mut done = 0;
    for p in pages {
        let Some(&id) = ids.get(p) else { continue };
        let dict = doc
            .get_object_mut(id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| anyhow!(tr!("Page {p}: {e}", "Página {p}: {e}")))?;
        let current = dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        dict.set("Rotate", lopdf::Object::Integer((current + delta) % 360));
        done += 1;
    }
    if done == 0 {
        return Err(anyhow!(tr!(
            "There are no pages to rotate",
            "No hay páginas que rotar"
        )));
    }
    finish(doc, output)?;
    Ok(done)
}

/// Atributos que una página puede heredar de sus nodos `Pages` antecesores.
const INHERITABLE: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Reordena las páginas: `order` es la lista completa de páginas (1-based) en el orden
/// nuevo. El árbol de páginas se aplana en un solo nodo `Pages`; antes se copian a cada
/// página los atributos que heredaba, para que no cambie su aspecto.
pub fn reorder_pages(input: &Path, order: &[u32], output: &Path) -> Result<usize> {
    use lopdf::{Dictionary, Object};
    let mut doc = load(input)?;
    let ids = doc.get_pages();
    let total = ids.len();
    let mut seen = std::collections::BTreeSet::new();
    if order.len() != total || !order.iter().all(|p| ids.contains_key(p) && seen.insert(*p)) {
        return Err(anyhow!(tr!(
            "The order must include each page exactly once",
            "El orden debe incluir cada página una sola vez"
        )));
    }
    if order.iter().enumerate().all(|(i, p)| *p as usize == i + 1) {
        return Err(anyhow!(tr!(
            "The pages are already in that order",
            "Las páginas ya están en ese orden"
        )));
    }
    let root_pages = doc
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(Object::as_reference)
        .map_err(|e| {
            anyhow!(tr!(
                "PDF without a page tree: {e}",
                "PDF sin árbol de páginas: {e}"
            ))
        })?;

    for &id in ids.values() {
        // Sube por los padres y copia lo que la página no define por sí misma.
        let mut inherited = Dictionary::new();
        let mut parent = doc
            .get_dictionary(id)
            .ok()
            .and_then(|d| d.get(b"Parent").ok())
            .and_then(|o| o.as_reference().ok());
        let mut guard = 0;
        while let Some(pid) = parent {
            guard += 1;
            if guard > 64 {
                break;
            }
            let Ok(pd) = doc.get_dictionary(pid) else {
                break;
            };
            for key in INHERITABLE {
                if !inherited.has(key) {
                    if let Ok(v) = pd.get(key) {
                        inherited.set(key.to_vec(), v.clone());
                    }
                }
            }
            parent = pd.get(b"Parent").ok().and_then(|o| o.as_reference().ok());
        }
        let page = doc
            .get_object_mut(id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| anyhow!(tr!("Page: {e}", "Página: {e}")))?;
        for (k, v) in inherited.iter() {
            if !page.has(k) {
                page.set(k.clone(), v.clone());
            }
        }
        page.set("Parent", Object::Reference(root_pages));
    }

    let kids: Vec<Object> = order.iter().map(|p| Object::Reference(ids[p])).collect();
    let pages = doc
        .get_object_mut(root_pages)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| anyhow!(tr!("Page tree: {e}", "Árbol de páginas: {e}")))?;
    pages.set("Kids", Object::Array(kids));
    pages.set("Count", Object::Integer(total as i64));
    for key in INHERITABLE {
        pages.remove(key);
    }
    finish(doc, output)?;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `IUREOCR_TEST_PDF=/ruta/escaneo.pdf cargo test -- --ignored`: quita, conserva y rota.
    #[test]
    #[ignore]
    fn edita_paginas() {
        let Ok(pdf) = std::env::var("IUREOCR_TEST_PDF") else {
            return;
        };
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pdfium =
            crate::ocr::pdfium(Some(&manifest.join("resources").join("pdfium"))).expect("pdfium");
        let input = Path::new(&pdf);
        let n = page_count(pdfium, input).expect("páginas");
        assert!(n >= 2);
        let sizes = page_sizes(pdfium, input).expect("tamaños");
        assert_eq!(sizes.len(), n);
        assert!(sizes.iter().all(|(w, h)| *w > 0.0 && *h > 0.0));
        let thumbs = thumbnails(pdfium, input, &[0, 1], 120).expect("miniaturas");
        assert_eq!(thumbs.len(), 2);
        assert!(thumbs[0].starts_with("data:image/jpeg;base64,"));

        let out = std::env::temp_dir().join("iureocr-pdf-test");
        let _ = std::fs::remove_dir_all(&out);
        std::fs::create_dir_all(&out).unwrap();
        let sin2 = out.join("sin-pagina-2.pdf");
        assert_eq!(delete_pages(input, &[2], &sin2).expect("quitar"), n - 1);
        assert_eq!(page_count(pdfium, &sin2).unwrap(), n - 1);
        let solo2 = out.join("solo-pagina-2.pdf");
        assert_eq!(keep_pages(input, &[2], &solo2).expect("conservar"), 1);
        assert_eq!(page_count(pdfium, &solo2).unwrap(), 1);
        let mut order: Vec<u32> = (1..=n as u32).rev().collect();
        let reord = out.join("reordenado.pdf");
        assert_eq!(reorder_pages(input, &order, &reord).expect("reordenar"), n);
        let a = thumbnails(pdfium, input, &[0], 60).unwrap();
        let b = thumbnails(pdfium, &reord, &[n - 1], 60).unwrap();
        assert_eq!(a, b, "la primera página debe quedar al final");
        order.push(1);
        assert!(reorder_pages(input, &order, &out.join("y.pdf")).is_err());
        let rot = out.join("rotado.pdf");
        assert_eq!(rotate_pages(input, &[1], 90, &rot).expect("rotar"), 1);
        assert_eq!(page_count(pdfium, &rot).unwrap(), n);
        assert!(delete_pages(
            input,
            &(1..=n as u32).collect::<Vec<_>>(),
            &out.join("x.pdf")
        )
        .is_err());
    }
}

#[cfg(test)]
mod hilos {
    use super::*;

    /// `IUREOCR_TEST_PDF=… cargo test -- --ignored`: miniaturas y tamaños pedidos desde
    /// varios hilos a la vez, como hacen la lista, la cuadrícula y el visor.
    #[test]
    #[ignore]
    fn miniaturas_en_hilos() {
        let pdf = std::env::var("IUREOCR_TEST_PDF").unwrap();
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pdfium = crate::ocr::pdfium(Some(&manifest.join("resources").join("pdfium"))).unwrap();
        let n = page_count(pdfium, Path::new(&pdf)).unwrap();
        let hs: Vec<_> = (0..4)
            .map(|t| {
                let pdf = pdf.clone();
                std::thread::spawn(move || {
                    for r in 0..3 {
                        let idx: Vec<usize> = (0..n).filter(|i| (i + t + r) % 2 == 0).collect();
                        thumbnails(pdfium, Path::new(&pdf), &idx, 160).unwrap();
                        page_sizes(pdfium, Path::new(&pdf)).unwrap();
                    }
                })
            })
            .collect();
        for h in hs {
            h.join().unwrap();
        }
    }
}
