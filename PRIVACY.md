# Política de privacidad de IureOCR / IureOCR privacy policy

*Última actualización / Last updated: 2026-11-25* · [Español](#español) · [English](#english)

## Español

IureOCR es una aplicación de escritorio que convierte escaneos e imágenes en PDF con texto
buscable y ofrece herramientas PDF (quitar, rotar y reordenar páginas, testado, numeración
Bates). La publica Eduardo Llaguno Velasco («nosotros»). Esta política explica qué datos
maneja, incluido su servidor MCP local (`IureOCR --mcp`).

**Resumen:** el reconocimiento de texto se hace por completo en tu equipo con Tesseract; tus
documentos no se envían a nosotros ni a ningún servicio de OCR o de inteligencia artificial
en la nube. IureOCR no tiene telemetría, analítica ni reportes automáticos de fallos. Lo que
sale de tu equipo va a la instancia de Iurefficient que tú configures o a GitHub.

### Tus documentos
- Los PDF e imágenes que abres se procesan en tu equipo (pdfium y Tesseract, incluidos en la
  app). El resultado (PDF buscable y `.txt`) se guarda junto al original o en la carpeta que
  elijas; el original no se modifica salvo que tú decidas reemplazarlo al editar páginas.
- Sólo salen de tu equipo si tú los guardas en Iurefficient (ver abajo).

### Qué sale de tu equipo
1. **Iurefficient (opcional).** Sólo si configuras una cuenta. IureOCR se conecta a *esa*
   instancia para iniciar sesión (correo, contraseña y, si lo usas, el código de verificación
   en dos pasos), comprobar la sesión al arrancar y en Ajustes, crear su contraseña de
   aplicación WebDAV, listar carpetas, buscar expedientes, subir los documentos que tú decidas
   guardar ahí y crear o revocar tokens de acceso para Microsoft 365 Copilot. El tratamiento
   de esos datos lo rige la política de privacidad de quien opera esa instancia (normalmente tu
   organización). Sin cuenta, IureOCR nunca la contacta.
2. **GitHub.** Al abrir Ajustes, IureOCR consulta `api.github.com` para mostrar la última
   versión de las otras apps de Iurefficient. Fuera de la Microsoft Store, también busca al
   arrancar una versión nueva de sí misma (se puede desactivar) y descarga las actualizaciones
   de `github.com`. En la versión de la Microsoft Store no busca ni descarga versiones nuevas
   de sí misma: la actualiza la Store. GitHub recibe, como en cualquier conexión, tu dirección
   IP y la versión de la app en el agente de usuario; no se le envía nada más.
3. **Asistentes de IA (opcional).** Si conectas IureOCR a Claude Desktop, VS Code u otro
   cliente MCP, ese asistente puede pedirle a IureOCR que lea o haga OCR de archivos de tu
   equipo, y recibe el texto resultante. Lo que el asistente haga después con ese texto
   (incluido enviarlo a su proveedor de IA) lo rige la política de ese asistente. IureOCR sólo
   responde a lo que le pide el asistente que tú conectaste; no contacta por sí mismo a ningún
   servicio de IA.
4. **Enlaces que abres.** El sitio de Iurefficient, la descarga de OnlyOffice, de Claude o de
   VS Code y otros enlaces sólo abren tu navegador.

### Qué se guarda en tu equipo
- Los ajustes (idiomas de OCR, resolución, carpeta de salida, tema, idioma de la interfaz, y
  el dominio y el correo de tu cuenta de Iurefficient) y un registro de actividad con nombres y
  rutas de los archivos procesados, en la carpeta de datos de tu usuario.
- Si inicias sesión en Iurefficient: la sesión y la contraseña de aplicación WebDAV (no tu
  contraseña) en el llavero del sistema (Credential Manager de Windows, Llavero de macOS o
  Secret Service en Linux), y el dominio y el correo de la cuenta activa, que las demás apps de
  Iurefficient usan al abrirse. Sólo si el llavero no está disponible, la contraseña de
  aplicación WebDAV queda en el archivo de ajustes. Cerrar la sesión borra la sesión guardada.
- Si conectas Claude Desktop, IureOCR agrega su propia entrada en el archivo de configuración
  de Claude Desktop (sólo esa entrada); desconectar la quita.
- Copias temporales de los documentos mientras editas sus páginas, que se borran al guardar o
  descartar.

### Niños
IureOCR es una herramienta de trabajo y no está dirigida a menores de 13 años.

### Tus opciones y contacto
Puedes usar IureOCR sin cuenta, desactivar la búsqueda de versiones nuevas (fuera de la
Store) y cerrar la sesión de Iurefficient en cualquier momento. Para preguntas sobre esta
política, abre un *issue* en <https://github.com/ellaguno/iureocr/issues>. Si la política
cambia, la nueva versión se publica en esta misma página con su fecha.

## English

IureOCR is a desktop application that turns scans and images into searchable PDFs and offers
PDF tools (remove, rotate and reorder pages, redaction, Bates numbering). It is published by
Eduardo Llaguno Velasco ("we"). This policy explains what data it handles, including its
local MCP server (`IureOCR --mcp`).

**Summary:** text recognition runs entirely on your computer with Tesseract; your documents
are not sent to us or to any cloud OCR or artificial intelligence service. IureOCR has no
telemetry, analytics or automatic crash reports. What leaves your computer goes to the
Iurefficient instance you configure or to GitHub.

### Your documents
- The PDFs and images you open are processed on your computer (pdfium and Tesseract, bundled
  with the app). The result (searchable PDF and `.txt`) is saved next to the original or in the
  folder you choose; the original is not modified unless you choose to replace it when editing
  pages.
- They only leave your computer if you save them to Iurefficient (see below).

### What leaves your computer
1. **Iurefficient (optional).** Only if you set up an account. IureOCR connects to *that*
   instance to sign in (email, password and, if you use one, the two-step verification code),
   check the session at startup and in Settings, create its WebDAV app password, list folders,
   search matters, upload the documents you choose to save there, and create or revoke access
   tokens for Microsoft 365 Copilot. Those data are governed by the privacy policy of whoever
   runs that instance (usually your organization). Without an account, IureOCR never contacts it.
2. **GitHub.** When you open Settings, IureOCR queries `api.github.com` to show the latest
   version of the other Iurefficient apps. Outside the Microsoft Store it also checks at startup
   for a new version of itself (this can be turned off) and downloads updates from `github.com`.
   The Microsoft Store version does not check for or download new versions of itself: the Store
   updates it. As with any connection, GitHub receives your IP address and the app's version in
   the user agent; nothing else is sent.
3. **AI assistants (optional).** If you connect IureOCR to Claude Desktop, VS Code or another MCP
   client, that assistant can ask IureOCR to read or OCR files on your computer, and it receives
   the resulting text. What the assistant does with that text afterwards (including sending it
   to its AI provider) is governed by that assistant's policy. IureOCR only answers the
   assistant you connected; it does not contact any AI service on its own.
4. **Links you open.** The Iurefficient website, the OnlyOffice, Claude or VS Code downloads and
   other links only open your browser.

### What is stored on your computer
- Settings (OCR languages, resolution, output folder, theme, interface language, and your
  Iurefficient account's domain and email) and an activity log with the names and paths of the
  processed files, in your user's data folder.
- If you sign in to Iurefficient: the session and the WebDAV app password (not your password) in
  the system keychain (Windows Credential Manager, macOS Keychain or Secret Service on Linux), and
  the active account's domain and email, which the other Iurefficient apps use when they open.
  Only if the keychain is unavailable is the WebDAV app password kept in the settings file.
  Signing out deletes the saved session.
- If you connect Claude Desktop, IureOCR adds its own entry to Claude Desktop's configuration
  file (only that entry); disconnecting removes it.
- Temporary copies of documents while you edit their pages, deleted when you save or discard.

### Children
IureOCR is a work tool and is not directed to children under 13.

### Your choices and contact
You can use IureOCR without an account, turn off the new-version check (outside the Store) and
sign out of Iurefficient at any time. For questions about this policy, open an issue at
<https://github.com/ellaguno/iureocr/issues>. If the policy changes, the new version is
published on this same page with its date.
