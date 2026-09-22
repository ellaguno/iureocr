#!/usr/bin/env python3
"""Extrae el Tesseract de UB Mannheim (Apache-2.0) para empaquetarlo en Windows.

El instalador oficial es un NSIS; 7-Zip lo desempaqueta sin ejecutarlo. Se queda
solo con el ejecutable y sus DLL: los modelos de idioma van aparte, en
`resources/tessdata/` (ver descargar-tessdata.py), para que sean los mismos en
las tres plataformas. Resultado: `src-tauri/resources/tesseract/tesseract.exe`.

Requiere `7z` en el PATH (los runners de GitHub lo traen).

    python3 scripts/descargar-tesseract-windows.py
"""
import shutil
import subprocess
import tempfile
import urllib.request
from pathlib import Path

VERSION = "5.4.0.20240606"
URL = f"https://github.com/UB-Mannheim/tesseract/releases/download/v{VERSION}/tesseract-ocr-w64-setup-{VERSION}.exe"
DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "resources" / "tesseract"


def main() -> None:
    if (DESTINO / "tesseract.exe").exists():
        print(f"  ya estaba: {DESTINO / 'tesseract.exe'}")
        return
    siete = shutil.which("7z") or shutil.which("7za")
    if not siete:
        raise SystemExit("hace falta 7z en el PATH para desempaquetar el instalador")
    with tempfile.TemporaryDirectory() as tmp:
        instalador = Path(tmp) / "tesseract-setup.exe"
        print(f"  descargando {URL}")
        with urllib.request.urlopen(URL, timeout=300) as r:
            instalador.write_bytes(r.read())
        extraido = Path(tmp) / "x"
        subprocess.run([siete, "x", "-y", f"-o{extraido}", str(instalador)], check=True, capture_output=True)
        # El NSIS deja los ficheros bajo la raiz de la extraccion (tesseract.exe y *.dll).
        exe = next(extraido.rglob("tesseract.exe"), None)
        if exe is None:
            raise SystemExit("no se encontro tesseract.exe dentro del instalador")
        DESTINO.mkdir(parents=True, exist_ok=True)
        copiados = 0
        for f in exe.parent.iterdir():
            if f.is_file() and f.suffix.lower() in (".exe", ".dll"):
                shutil.copy2(f, DESTINO / f.name)
                copiados += 1
        print(f"  {copiados} ficheros en {DESTINO}")


if __name__ == "__main__":
    main()
