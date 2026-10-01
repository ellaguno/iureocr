# Capturas y GIF del README

Genera `docs/media/hero-{en,es}.gif` y `docs/media/{queue,result,pages,agents}-{en,es}.png`
con la interfaz real de IureOCR: el frontend corre en el servidor de Vite y el backend de
Tauri se simula en `mock.ts` (`@tauri-apps/api/mocks`). Los documentos son ficticios
(`docs.mjs`); `make-scans.mjs` los dibuja como páginas escaneadas y pasa el contrato por
Tesseract, así que el texto que aparece en las capturas es OCR de verdad.

Requisitos: Chromium de Playwright (o `CHROME=/ruta/a/chrome`), `tesseract` con `spa` y
`eng`, ImageMagick (`convert`), `ffmpeg`, `pngquant` y `fuser`.

```bash
npm i --no-save puppeteer-core      # una vez, sin tocar package.json
scripts/readme-media/build.sh       # PORT=5304 por defecto
```

Por partes: `node make-scans.mjs` (páginas y OCR en `scans/`), `node capture.mjs <en|es> [puerto]`
con Vite ya levantado (`npx vite --port 5304`) para los PNG sin procesar en `out/`.
