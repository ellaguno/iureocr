[Leer en español](README.es.md)

# IureOCR

Recognizes the text in scans and photos of documents **on your own computer** and
produces a PDF with searchable text, ready to upload to Iurefficient. It is the fourth
desktop app from [Iurefficient](https://iurefficient.com), alongside IureTranscribe,
IureEditor and IureDav, and shares the account, keychain and session with them.

## What it does

- **Local OCR with Tesseract** in Spanish and English (more languages if your
  Tesseract has them). Nothing leaves the computer: neither the document nor the text.
- Accepts **scanned PDFs** and **images** (JPG, PNG, TIFF, BMP, WEBP). The result is a
  PDF with the original image and an invisible text layer on top: it can be searched
  and copied, and the Iurefficient server indexes it without processing it again.
  It also leaves a `.txt` with the plain text.
- **Skips PDFs that already have text** (same rule as the server: fewer than 100
  characters in the first three pages = needs recognition). It can be forced.
- **Save to Iurefficient**: to a project (REST API) or to any folder in the document
  tree (WebDAV). With the IureDav drive mounted, just choose it as the output folder.
- **Queue** with per-page progress, cancel and retry. Drag files onto the window,
  open them with IureOCR from the system or with `iureocr://ocr?file=…` links.
- For editing the result it suggests **OnlyOffice Desktop Editors**: it detects it if
  installed and otherwise links to its download. It is not bundled.
- **English and Spanish interface**: English by default, and Spanish if the operating
  system is set to Spanish; it can be changed in Settings → Appearance. This is
  independent of the text languages Tesseract recognizes.

Planned for upcoming versions (see the [CHANGELOG](CHANGELOG.md)): PDF tools (merge,
split, extract and delete pages, rotate, password protection, watermark, page
numbers), real image compression, rescuing unreadable pages with a vision model, and
the instance's list of documents pending OCR.

## How it works inside

1. **pdfium** rasterizes each PDF page to JPEG at the chosen resolution (300 dpi by
   default). Images are passed as they are.
2. **Tesseract** receives the list of pages and produces the PDF with the text layer
   itself (its `pdf` renderer, the same one ocrmypdf uses) and the `.txt`.
3. The result is saved next to the original with the suffix ` - OCR` (configurable)
   or in a fixed folder.

A PDF that already has readable text is not rasterized: the app says so and offers
to force it.

## Tesseract on each system

| System | Where Tesseract comes from | Language models |
| --- | --- | --- |
| Linux | the system package: the `.deb`/`.rpm` depend on `tesseract-ocr` with `spa` and `eng` | the ones bundled with the app (`tessdata_fast`) |
| Windows | **bundled in the installer** (UB Mannheim build, Apache-2.0) | bundled |
| macOS | `brew install tesseract` (not bundled for now) | bundled |

The app looks for Tesseract in this order: the `IUREOCR_TESSERACT` variable, the one
inside the installer, the usual system paths and the `PATH`. Settings shows which one
it found, its version and the available languages.

## Development

Requirements: Node 22, stable Rust, the Tauri 2 dependencies for your system and
Tesseract installed (on Linux `sudo apt install tesseract-ocr tesseract-ocr-spa
tesseract-ocr-eng`).

```bash
npm ci
python3 scripts/descargar-pdfium.py     # pdfium library for this platform
python3 scripts/descargar-tessdata.py   # spa, eng and osd models (tessdata_fast)
npm run tauri dev
```

On Windows, `python3 scripts/descargar-tesseract-windows.py` also fetches the
Tesseract that gets bundled (requires `7z`).

```bash
npm run check                 # frontend types
cd src-tauri && cargo check   # backend
npm run tauri build           # installers for this platform
```

## Releasing a version

Bump the version in `package.json`, `src-tauri/Cargo.toml` and
`src-tauri/tauri.conf.json`, add the section to the CHANGELOG and tag:

```bash
git commit -am "v0.2.0: …"
git tag -a v0.2.0 -m "IureOCR 0.2.0" && git push --follow-tags
```

`.github/workflows/build.yml` builds on every push and, with a `vX.Y.Z` tag,
publishes the release with installers for Windows (`1-windows-x64`), macOS
(`2-macos-universal`) and Linux (`3-linux-x64`, `.deb`, `.rpm` and AppImage), plus the
updater's `latest.json`. Updates are signed with the app's minisign key
(`~/.tauri/iureocr-updater.key`); Windows Authenticode signing arrives with SignPath
once the token is in the repository secrets.

## Where it stores things

| What | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Settings | `~/.config/com.iurefficient.iureocr/settings.json` | `%APPDATA%\com.iurefficient.iureocr\settings.json` | `~/Library/Application Support/com.iurefficient.iureocr/settings.json` |
| Log | `~/.local/share/com.iurefficient.iureocr/logs/iureocr.log` | `%APPDATA%\com.iurefficient.iureocr\logs\iureocr.log` | `~/Library/Application Support/com.iurefficient.iureocr/logs/iureocr.log` |

Credentials go to the system keychain, shared with the other Iurefficient apps; the
active account (instance and email) lives in `<config>/iurefficient/`.

## Code signing policy

Free code signing provided by [SignPath.io](https://signpath.io), certificate by
[SignPath Foundation](https://signpath.org).

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

## License

MIT. Tesseract (Apache-2.0), pdfium (BSD-3) and the `tessdata_fast` models
(Apache-2.0) keep their own licenses.
