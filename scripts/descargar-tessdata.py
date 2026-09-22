#!/usr/bin/env python3
"""Descarga los modelos de idioma de Tesseract que se empaquetan con la app.

Se usan los `tessdata_fast` (Apache-2.0): español, inglés y `osd` (detección de
orientación). Son más pequeños y rápidos que los `best`, con una precisión muy
parecida en documentos escaneados. Quedan en `src-tauri/resources/tessdata/`.

    python3 scripts/descargar-tessdata.py
"""
import urllib.request
from pathlib import Path

# Commit fijado a propósito para que el instalador no cambie de modelos solo.
COMMIT = "main"
IDIOMAS = ["spa", "eng", "osd"]
DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "resources" / "tessdata"


def main() -> None:
    DESTINO.mkdir(parents=True, exist_ok=True)
    for idioma in IDIOMAS:
        salida = DESTINO / f"{idioma}.traineddata"
        if salida.exists():
            print(f"  ya estaba: {salida.name}")
            continue
        url = f"https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/{COMMIT}/{idioma}.traineddata"
        print(f"  descargando {url}")
        with urllib.request.urlopen(url, timeout=180) as r:
            salida.write_bytes(r.read())
        print(f"  escrito: {salida.name} ({salida.stat().st_size // 1024} KB)")
    # Tesseract necesita pdf.ttf (fuente sin glifos para la capa de texto) junto a los modelos.
    pdf_ttf = DESTINO / "pdf.ttf"
    if not pdf_ttf.exists():
        url = "https://raw.githubusercontent.com/tesseract-ocr/tesseract/main/tessdata/pdf.ttf"
        print(f"  descargando {url}")
        with urllib.request.urlopen(url, timeout=180) as r:
            pdf_ttf.write_bytes(r.read())
        print(f"  escrito: pdf.ttf")


if __name__ == "__main__":
    main()
