//! IureOCR instalada desde Microsoft Store (paquete MSIX).
//!
//! La Store no permite que una app se actualice por su cuenta: empaquetada, no se carga
//! el actualizador de Tauri ni se avisa de versiones nuevas en GitHub (la actualiza la
//! Store). Tampoco se registra el esquema `iureocr://` al arrancar (lo declara el
//! manifiesto) y los clientes MCP lanzan la app por su alias `iureocr.exe`.

/// Si corre como paquete MSIX (con identidad de paquete), como la versión de la Store.
pub fn from_store() -> bool {
    #[cfg(windows)]
    {
        static STORE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *STORE.get_or_init(|| {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetCurrentPackageFullName(length: *mut u32, name: *mut u16) -> i32;
            }
            const APPMODEL_ERROR_NO_PACKAGE: i32 = 15700;
            let mut len = 0u32;
            // Con paquete devuelve ERROR_INSUFFICIENT_BUFFER (el búfer vacío no alcanza).
            let r = unsafe { GetCurrentPackageFullName(&mut len, std::ptr::null_mut()) };
            r != APPMODEL_ERROR_NO_PACKAGE
        })
    }
    #[cfg(not(windows))]
    false
}

/// Empaquetada, el ejecutable vive en `C:\Program Files\WindowsApps\<paquete>_<versión>…`:
/// otro programa no puede lanzarlo por esa ruta (y cambia con cada versión). Lo que se
/// registra en Claude Desktop o VS Code es el alias del manifiesto,
/// `%LOCALAPPDATA%\Microsoft\WindowsApps\iureocr.exe`, que además arranca con la
/// identidad del paquete y ve los mismos ajustes que la ventana.
///
/// `iurefficient_connect::agents::command()` toma la ruta de `APPIMAGE` cuando existe
/// (pensado para el AppImage de Linux; en Windows nadie más la lee): se apunta al alias.
pub fn use_alias_for_mcp_clients() {
    #[cfg(windows)]
    if from_store() {
        if let Some(alias) = dirs::data_local_dir()
            .map(|d| d.join("Microsoft").join("WindowsApps").join("iureocr.exe"))
        {
            // El alias es un punto de reparación que `metadata()` no sigue: basta con que exista.
            if std::fs::symlink_metadata(&alias).is_ok() {
                std::env::set_var("APPIMAGE", alias);
            }
        }
    }
}
