# Registro de cambios

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las
versiones, [SemVer](https://semver.org/lang/es/).

## [Sin publicar]

Previsto: unir PDF, dividir por rangos, reordenar páginas, proteger con contraseña,
marca de agua y numerar; compresión real de imágenes (v2); rescate de páginas
ilegibles con un modelo de visión, primero por el motor de la instancia u OpenRouter
y después en local; lista de documentos pendientes de OCR de la instancia; Tesseract
incluido también en macOS.

## [0.2.0] — 2026-09-22

### Añadido

- **Miniaturas de las páginas** en cada archivo, con la página que se está
  reconociendo resaltada, y una vista de páginas en cuadrícula («Páginas»).
- **Edición gráfica de páginas**: en la cuadrícula se marcan páginas y se pueden
  **quitar**, **conservar sólo esas** o **rotar** 90°. Cada edición escribe un PDF
  nuevo junto al original (` - sin páginas`, ` - páginas`, ` - rotado`), que aparece
  en la lista listo para abrir, guardar en Iurefficient o pasar por OCR.
- Miniatura también para imágenes sueltas.

### Corregido

- «Abrir PDF» fallaba con `plugin opener open_path not allowed`: faltaba el permiso
  para abrir rutas de archivo.

### Cambiado

- pdfium se comparte entre el OCR y las miniaturas (una sola inicialización por proceso).

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
