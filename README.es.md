[Read in English](README.md)

# IureOCR

Reconoce el texto de escaneos y fotos de documentos **en tu propio equipo** y deja un
PDF con texto buscable, listo para subir a Iurefficient. Es la cuarta app de
escritorio de [Iurefficient](https://iurefficient.com), junto a IureTranscribe,
IureEditor e IureDav, y comparte con ellas la cuenta, el llavero y la sesión.

## Qué hace

- **OCR local con Tesseract** en español e inglés (más idiomas si tu Tesseract los
  tiene). Nada sale del equipo: ni el documento ni el texto.
- Acepta **PDF escaneados** e **imágenes** (JPG, PNG, TIFF, BMP, WEBP). El resultado
  es un PDF con la imagen original y, encima, una capa de texto invisible: se puede
  buscar, copiar y el servidor de Iurefficient lo indexa sin volver a procesarlo.
  Además deja un `.txt` con el texto plano.
- **Omite los PDF que ya tienen texto** (mismo criterio que el servidor: menos de
  100 caracteres en las tres primeras páginas = hay que reconocer). Se puede forzar.
- **Guardar en Iurefficient**: en un proyecto (API REST) o en cualquier carpeta del
  árbol de documentos (WebDAV). Con la unidad de IureDav montada, basta con elegirla
  como carpeta de salida.
- **Cola** con progreso por página, cancelación y reintento. Arrastra archivos a la
  ventana, ábrelos con IureOCR desde el sistema o con enlaces `iureocr://ocr?file=…`.
- Para editar el resultado sugiere **OnlyOffice Desktop Editors**: lo detecta si está
  instalado y, si no, enlaza su descarga. No se incluye.
- **Interfaz en inglés y español**: inglés por defecto y español si el sistema
  operativo está en español; se puede elegir en Ajustes → Apariencia. Es independiente
  de los idiomas del texto que reconoce Tesseract.

Pendiente para versiones siguientes (ver [CHANGELOG](CHANGELOG.md)): herramientas PDF
(unir, dividir, extraer y borrar páginas, rotar, proteger con contraseña, marca de
agua, numerar), compresión real de imágenes, rescate de páginas ilegibles con un
modelo de visión y la lista de documentos pendientes de OCR de la instancia.

## Cómo funciona por dentro

1. **pdfium** rasteriza cada página del PDF a JPEG a la resolución elegida (300 ppp
   por defecto). Las imágenes se pasan tal cual.
2. **Tesseract** recibe la lista de páginas y produce, él mismo, el PDF con la capa
   de texto (su renderizador `pdf`, el mismo que usa ocrmypdf) y el `.txt`.
3. El resultado se guarda junto al original con el sufijo ` - OCR` (configurable) o
   en una carpeta fija.

Un PDF con texto ya legible no se rasteriza: se avisa y se ofrece forzarlo.

## Tesseract en cada sistema

| Sistema | De dónde sale Tesseract | Modelos de idioma |
| --- | --- | --- |
| Linux | el paquete del sistema: el `.deb`/`.rpm` dependen de `tesseract-ocr` con `spa` y `eng` | los incluidos en la app (`tessdata_fast`) |
| Windows | **incluido en el instalador** (build de UB Mannheim, Apache-2.0) | incluidos |
| macOS | `brew install tesseract` (por ahora no va incluido) | incluidos |

La app busca Tesseract en este orden: la variable `IUREOCR_TESSERACT`, el que viene
dentro del instalador, las rutas habituales del sistema y el `PATH`. En Ajustes se ve
cuál encontró, su versión y los idiomas disponibles.

## Desarrollo

Requisitos: Node 22, Rust estable, las dependencias de Tauri 2 de tu sistema y
Tesseract instalado (en Linux `sudo apt install tesseract-ocr tesseract-ocr-spa
tesseract-ocr-eng`).

```bash
npm ci
python3 scripts/descargar-pdfium.py     # biblioteca pdfium de esta plataforma
python3 scripts/descargar-tessdata.py   # modelos spa, eng y osd (tessdata_fast)
npm run tauri dev
```

En Windows, además, `python3 scripts/descargar-tesseract-windows.py` deja el
Tesseract que se empaqueta (requiere `7z`).

```bash
npm run check                 # tipos del frontend
cd src-tauri && cargo check   # backend
npm run tauri build           # instaladores de esta plataforma
```

## Publicar una versión

Sube la versión en `package.json`, `src-tauri/Cargo.toml` y
`src-tauri/tauri.conf.json`, añade la sección al CHANGELOG y etiqueta:

```bash
git commit -am "v0.2.0: …"
git tag -a v0.2.0 -m "IureOCR 0.2.0" && git push --follow-tags
```

`.github/workflows/build.yml` compila en cada push y, con una etiqueta `vX.Y.Z`,
publica la release con instaladores para Windows (`1-windows-x64`), macOS
(`2-macos-universal`) y Linux (`3-linux-x64`, `.deb`, `.rpm` y AppImage), más el
`latest.json` del actualizador. Las actualizaciones se firman con la clave minisign
de la app (`~/.tauri/iureocr-updater.key`); la firma Authenticode de Windows llega
con SignPath cuando exista el token en los secretos del repositorio.

## Dónde guarda las cosas

| Qué | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Ajustes | `~/.config/com.iurefficient.iureocr/settings.json` | `%APPDATA%\com.iurefficient.iureocr\settings.json` | `~/Library/Application Support/com.iurefficient.iureocr/settings.json` |
| Registro | `~/.local/share/com.iurefficient.iureocr/logs/iureocr.log` | `%APPDATA%\com.iurefficient.iureocr\logs\iureocr.log` | `~/Library/Application Support/com.iurefficient.iureocr/logs/iureocr.log` |

Las credenciales van al llavero del sistema, compartidas con las otras apps de
Iurefficient; la cuenta activa (instancia y correo) en `<config>/iurefficient/`.

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

*Firma de código gratuita proporcionada por SignPath.io, con certificado de SignPath
Foundation.*

- **Committers and reviewers:** Eduardo Llaguno ([@ellaguno](https://github.com/ellaguno)).
- **Approvers:** Eduardo Llaguno ([@ellaguno](https://github.com/ellaguno)).
- Every Windows release is built from this repository by GitHub Actions
  (`.github/workflows/build.yml`), submitted to SignPath from that workflow and
  approved manually before it is signed. Only the installer published on the
  [releases page](https://github.com/ellaguno/iureocr/releases) is signed.

### Privacy policy

This program will not transfer any information to other networked systems unless
specifically requested by the user or the person installing or operating it.

Text recognition runs entirely on the user's computer with
[Tesseract](https://github.com/tesseract-ocr/tesseract); documents never leave the
machine. Specifically, IureOCR connects only to:

- the Iurefficient instance that the user configures, and only when the user signs
  in or asks to save a document there;
- `api.github.com`, once at start-up, to check whether a newer release exists. It can
  be turned off in Settings.

It collects no telemetry and no usage statistics. Credentials are stored in the
operating system keychain, never in configuration files.

## Licencia

MIT. Tesseract (Apache-2.0), pdfium (BSD-3) y los modelos `tessdata_fast`
(Apache-2.0) conservan sus licencias.
