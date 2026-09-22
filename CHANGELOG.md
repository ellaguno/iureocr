# Registro de cambios

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las
versiones, [SemVer](https://semver.org/lang/es/).

## [Sin publicar]

Previsto: herramientas PDF (unir, dividir, extraer y borrar páginas, rotar, proteger,
marca de agua, numerar); compresión real de imágenes (v2); rescate de páginas
ilegibles con un modelo de visión, primero por el motor de la instancia u OpenRouter
y después en local; lista de documentos pendientes de OCR de la instancia; Tesseract
incluido también en macOS.

## [0.1.0] — 2026-09-22

Primera versión: el prototipo de OCR para validar calidad y empaquetado.

### Añadido

- OCR local de PDF escaneados e imágenes con Tesseract (español e inglés), con PDF
  de salida buscable y `.txt`. pdfium rasteriza las páginas; Tesseract genera el PDF.
- Cola con progreso por página, cancelación, reintento y «forzar OCR» para PDF que
  ya tienen texto (se omiten por defecto, con el mismo umbral que el servidor).
- Guardar el resultado en Iurefficient: en un proyecto o en una carpeta del árbol
  de documentos. Cuenta, llavero y sesión compartidos con IureDav, IureTranscribe e
  IureEditor (conector 0.5.0).
- Detección de OnlyOffice para editar el resultado, con enlace de descarga si falta.
- Windows: Tesseract incluido en el instalador. Linux: dependencia del paquete del
  sistema. macOS: `brew install tesseract` por ahora.
- Actualizador firmado, aviso de versiones nuevas, registro en archivo, enlaces
  `iureocr://` y apertura de archivos desde el sistema.
