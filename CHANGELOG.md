# Registro de cambios

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y las
versiones, [SemVer](https://semver.org/lang/es/).

## [Sin publicar]

Previsto: unir PDF, dividir por rangos, proteger con contraseña,
marca de agua y numerar; compresión real de imágenes (v2); rescate de páginas
ilegibles con un modelo de visión, primero por el motor de la instancia u OpenRouter
y después en local; lista de documentos pendientes de OCR de la instancia; Tesseract
incluido también en macOS.

## [0.3.3] — 2026-09-25

### Añadido

- La barra lateral se puede colapsar (sólo iconos) y se colapsa sola con la
  ventana de 1000 px o menos; en ventanas anchas se recuerda la preferencia.

## [0.3.0] — 2026-09-22

### Añadido

- **Visor interno** («Ver», o clic en cualquier miniatura): páginas en scroll
  continuo, ir a página, zoom (botones, `+`/`-`, Ctrl+rueda) y teclado (RePág, AvPág,
  Inicio, Fin, Esc). Sólo se dibujan las páginas visibles, a la resolución que ocupan
  en pantalla, así que abre expedientes de cientos de páginas sin esperar. Resalta la
  página que se está reconociendo y, al terminar el OCR, muestra el PDF resultante.
- **Reordenar páginas** en la cuadrícula: arrastrando miniaturas (si la página
  arrastrada está seleccionada, se mueven todas las seleccionadas), con «Al principio»,
  «Al final» y «Orden inverso». «Guardar orden» escribe ` - reordenado.pdf` junto al
  original; las páginas conservan los atributos que heredaban del árbol de páginas.
- Doble clic en una página de la cuadrícula la abre en el visor.
- Ajuste **«Reconocer el texto en cuanto se abre un archivo»** (apagado por
  defecto) y botón «Reconocer texto de N» para lanzar el OCR de todos los abiertos.

### Cambiado

- Abrir un archivo **ya no lanza el OCR**: queda en la lista «Sin OCR» para verlo,
  editar sus páginas o subirlo, y el OCR se pide con «Reconocer texto». Quien prefiera
  el comportamiento anterior lo activa en Ajustes.
- «Abrir PDF» pasa a «Abrir fuera» (la aplicación del sistema); «Ver» usa el visor.
- La sección se llama «Documentos».

### Corregido

- «Páginas» se quedaba en «Contando páginas…»: el recuento se escribía en una copia
  no reactiva del archivo y la interfaz nunca se enteraba. Si el archivo no se puede
  leer, ahora se avisa en lugar de esperar para siempre.

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
