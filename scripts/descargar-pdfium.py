#!/usr/bin/env python3
"""Descarga la biblioteca pdfium que la app usa para rasterizar páginas de PDF.

Viene de los binarios precompilados de bblanchon/pdfium-binaries (BSD-3, como
pdfium). Se fija la versión a propósito. Queda en `src-tauri/resources/pdfium/`
con el nombre que pdfium-render espera en cada sistema.

    python3 scripts/descargar-pdfium.py                    # la de esta máquina
    python3 scripts/descargar-pdfium.py --objetivo win-x64 # una concreta
"""
import argparse
import io
import platform
import sys
import tarfile
import urllib.request
from pathlib import Path

VERSION = "chromium/8066"
DESTINO = Path(__file__).resolve().parent.parent / "src-tauri" / "resources" / "pdfium"

# objetivo -> (nombre del .tgz, ruta dentro del .tgz, nombre final)
OBJETIVOS = {
    "linux-x64": ("pdfium-linux-x64.tgz", "lib/libpdfium.so", "libpdfium.so"),
    "linux-arm64": ("pdfium-linux-arm64.tgz", "lib/libpdfium.so", "libpdfium.so"),
    "win-x64": ("pdfium-win-x64.tgz", "bin/pdfium.dll", "pdfium.dll"),
    "mac-univ": ("pdfium-mac-univ.tgz", "lib/libpdfium.dylib", "libpdfium.dylib"),
}


def objetivo_local() -> str:
    m = platform.machine().lower()
    if sys.platform.startswith("linux"):
        return "linux-arm64" if m in ("arm64", "aarch64") else "linux-x64"
    if sys.platform == "darwin":
        return "mac-univ"
    if sys.platform.startswith("win"):
        return "win-x64"
    raise SystemExit(f"plataforma no contemplada: {sys.platform}")


def descargar(objetivo: str) -> Path:
    tgz, dentro, nombre = OBJETIVOS[objetivo]
    salida = DESTINO / nombre
    if salida.exists():
        print(f"  ya estaba: {salida}")
        return salida
    url = f"https://github.com/bblanchon/pdfium-binaries/releases/download/{VERSION.replace('/', '%2F')}/{tgz}"
    print(f"  descargando {url}")
    with urllib.request.urlopen(url, timeout=180) as r:
        datos = r.read()
    DESTINO.mkdir(parents=True, exist_ok=True)
    with tarfile.open(fileobj=io.BytesIO(datos), mode="r:gz") as t:
        miembro = t.extractfile(dentro)
        if miembro is None:
            raise SystemExit(f"{tgz} no contiene {dentro}")
        salida.write_bytes(miembro.read())
    print(f"  escrito: {salida} ({salida.stat().st_size // 1024 // 1024} MB)")
    return salida


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--objetivo", choices=sorted(OBJETIVOS), help="plataforma concreta")
    args = ap.parse_args()
    print(f"pdfium {VERSION} -> {DESTINO}")
    descargar(args.objetivo or objetivo_local())


if __name__ == "__main__":
    main()
