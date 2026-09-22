mod ocr;
mod settings;
mod tesseract;

use iurefficient_connect::{
    api,
    rest::{Login, Session, SessionExport},
    secrets,
    webdav::{self, WebDav},
    Account,
};
use serde::{Deserialize, Serialize};
use settings::Settings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

const APP_NAME: &str = "IureOCR";

pub struct AppState {
    settings_path: PathBuf,
    settings: Arc<Mutex<Settings>>,
    resource_dir: Option<PathBuf>,
    /// Trabajos en curso: id → señal de cancelación.
    jobs: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Hilo que ejecuta los OCR de uno en uno (pdfium sólo se inicializa una vez por proceso).
    ocr: ocr::Worker,
    iure_session: tokio::sync::Mutex<Option<Arc<Session>>>,
}

// ---------------------------------------------------------------------------
// Registro en archivo (en Windows y en los lanzadores nadie ve stderr)
// ---------------------------------------------------------------------------
static LOG_PATH: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

struct LogTee(std::fs::File);

impl std::io::Write for LogTee {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::io::stderr().write_all(buf);
        self.0.write_all(buf)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = std::io::stderr().flush();
        self.0.flush()
    }
}

fn init_logging() {
    let env = env_logger::Env::default().default_filter_or("info");
    let mut builder = env_logger::Builder::from_env(env);
    let file = dirs::data_dir()
        .map(|d| d.join("com.iurefficient.iureocr").join("logs"))
        .and_then(|dir| {
            std::fs::create_dir_all(&dir).ok()?;
            let log = dir.join("iureocr.log");
            let _ = std::fs::rename(&log, dir.join("iureocr.prev.log"));
            std::fs::File::create(&log).ok().map(|f| (log, f))
        });
    match file {
        Some((log, f)) => {
            builder.target(env_logger::Target::Pipe(Box::new(LogTee(f))));
            let _ = LOG_PATH.set(Some(log));
        }
        None => {
            let _ = LOG_PATH.set(None);
        }
    }
    builder.init();
    std::panic::set_hook(Box::new(|info| {
        log::error!("la aplicación se cerró por un error interno: {info}");
    }));
    log::info!(
        "{APP_NAME} {} ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
}

// ---------------------------------------------------------------------------
// Ajustes y sistema
// ---------------------------------------------------------------------------
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn set_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings, String> {
    let mut s = settings;
    if s.languages.trim().is_empty() {
        s.languages = "spa+eng".into();
    }
    s.dpi = s.dpi.clamp(72, 600);
    s.jpeg_quality = s.jpeg_quality.clamp(30, 100);
    let acc_ok = Account::new(&s.iure_domain, &s.iure_email).ok();
    if let Some(acc) = acc_ok {
        // La contraseña WebDAV escrita a mano va al llavero compartido; vacía = se borra.
        if s.iure_app_password.trim().is_empty() {
            let _ = secrets::borrar(&acc, secrets::Kind::WebDav);
        } else if secrets::guardar(&acc, secrets::Kind::WebDav, s.iure_app_password.trim()).is_ok()
        {
            s.iure_app_password.clear();
        }
    }
    s.save(&state.settings_path).map_err(|e| e.to_string())?;
    *state.settings.lock().unwrap() = s.clone();
    Ok(s)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    version: &'static str,
    platform: &'static str,
    tesseract: Option<tesseract::Tesseract>,
    tesseract_error: Option<String>,
    pdfium_bundled: bool,
    settings_path: String,
    log_path: Option<String>,
    update_target: String,
    supported_extensions: &'static [&'static str],
}

fn pdfium_dir(state: &AppState) -> Option<PathBuf> {
    let d = state
        .resource_dir
        .as_ref()?
        .join("resources")
        .join("pdfium");
    if d.is_dir() {
        Some(d)
    } else {
        None
    }
}

#[tauri::command]
fn system_info(state: State<'_, AppState>) -> SystemInfo {
    let (tesseract, tesseract_error) = match tesseract::locate(state.resource_dir.as_deref()) {
        Ok(t) => (Some(t), None),
        Err(e) => (None, Some(e.to_string())),
    };
    SystemInfo {
        version: env!("CARGO_PKG_VERSION"),
        platform: std::env::consts::OS,
        tesseract,
        tesseract_error,
        pdfium_bundled: pdfium_dir(&state).is_some(),
        settings_path: state.settings_path.to_string_lossy().into_owned(),
        log_path: LOG_PATH
            .get()
            .and_then(|p| p.as_ref())
            .map(|p| p.to_string_lossy().into_owned()),
        update_target: format!(
            "{}-{}",
            if std::env::consts::OS == "macos" {
                "darwin"
            } else {
                std::env::consts::OS
            },
            std::env::consts::ARCH
        ),
        supported_extensions: ocr::SUPPORTED_EXTENSIONS,
    }
}

#[tauri::command]
fn reveal_path(path: String) -> Result<(), String> {
    tauri_plugin_opener::reveal_item_in_dir(path).map_err(|e| e.to_string())
}

#[tauri::command]
fn read_text_file(path: String, max_chars: Option<usize>) -> Result<String, String> {
    let t = std::fs::read_to_string(&path).map_err(|e| format!("No se pudo leer {path}: {e}"))?;
    Ok(match max_chars {
        Some(n) if t.chars().count() > n => t.chars().take(n).collect::<String>() + "\n…",
        _ => t,
    })
}

#[tauri::command]
fn file_size(path: String) -> Result<u64, String> {
    std::fs::metadata(&path)
        .map(|m| m.len())
        .map_err(|e| e.to_string())
}

/// Argumentos con los que se abrió la app (rutas de archivo o enlaces `iureocr://`).
#[tauri::command]
fn launch_args() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-'))
        .collect()
}

// ---------------------------------------------------------------------------
// OCR
// ---------------------------------------------------------------------------
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OcrProgress {
    job_id: String,
    stage: String,
    current: usize,
    total: usize,
}

#[tauri::command]
async fn ocr_start(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    path: String,
    force: bool,
) -> Result<ocr::Outcome, String> {
    let settings = state.settings.lock().unwrap().clone();
    let input = PathBuf::from(&path);
    let output_dir = match settings.output_mode.as_str() {
        "custom" => settings
            .output_dir
            .as_deref()
            .map(str::trim)
            .filter(|d| !d.is_empty())
            .map(PathBuf::from),
        _ => None,
    }
    .or_else(|| input.parent().map(Path::to_path_buf))
    .unwrap_or_else(|| PathBuf::from("."));
    let tess = tesseract::locate(state.resource_dir.as_deref()).map_err(|e| e.to_string())?;
    let pdfium = pdfium_dir(&state);
    let cancel = Arc::new(AtomicBool::new(false));
    state
        .jobs
        .lock()
        .unwrap()
        .insert(job_id.clone(), cancel.clone());
    let req = ocr::Request {
        job_id: job_id.clone(),
        input,
        languages: settings.languages.clone(),
        dpi: settings.dpi,
        jpeg_quality: settings.jpeg_quality,
        output_dir,
        suffix: settings.suffix.clone(),
        force,
    };
    let app2 = app.clone();
    let jid = job_id.clone();
    let progress = Box::new(move |stage: &str, current: usize, total: usize| {
        let _ = app2.emit(
            "ocr-progress",
            OcrProgress {
                job_id: jid.clone(),
                stage: stage.into(),
                current,
                total,
            },
        );
    });
    let result = state.ocr.submit(req, tess, pdfium, progress, cancel).await;
    state.jobs.lock().unwrap().remove(&job_id);
    match &result {
        Ok(o) => log::info!(
            "OCR {}: {} páginas, {} caracteres, {:.1}s{}",
            path,
            o.pages,
            o.chars,
            o.elapsed_secs,
            if o.skipped {
                " (omitido: ya tenía texto)"
            } else {
                ""
            }
        ),
        Err(e) => log::warn!("OCR {}: {e:#}", path),
    }
    result.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
fn ocr_cancel(state: State<'_, AppState>, job_id: String) {
    if let Some(c) = state.jobs.lock().unwrap().get(&job_id) {
        c.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

// ---------------------------------------------------------------------------
// Iurefficient: cuenta compartida, sesión REST y WebDAV (vía iurefficient-connect)
// ---------------------------------------------------------------------------
fn iure_user_agent() -> String {
    iurefficient_connect::user_agent(APP_NAME, env!("CARGO_PKG_VERSION"))
}

fn iure_account(state: &AppState) -> Result<Account, String> {
    let s = state.settings.lock().unwrap().clone();
    Account::new(&s.iure_domain, &s.iure_email).map_err(|e| e.to_string())
}

fn iure_webdav(state: &AppState) -> Result<WebDav, String> {
    let s = state.settings.lock().unwrap().clone();
    let acc = Account::new(&s.iure_domain, &s.iure_email).map_err(|e| e.to_string())?;
    let password = match secrets::leer(&acc, secrets::Kind::WebDav) {
        Ok(Some(p)) => p,
        _ => s.iure_app_password.clone(),
    };
    if password.trim().is_empty() {
        return Err(
            "No hay contraseña de aplicación WebDAV: inicia sesión o escríbela en Ajustes".into(),
        );
    }
    WebDav::new(acc, &password, &iure_user_agent()).map_err(|e| e.to_string())
}

fn persist_session(acc: &Account, sess: &Session) {
    if let Ok(json) = serde_json::to_string(&sess.export()) {
        let _ = secrets::guardar(acc, secrets::Kind::Session, &json);
    }
}

async fn iure_session(state: &AppState) -> Result<Arc<Session>, String> {
    if let Some(s) = state.iure_session.lock().await.as_ref() {
        return Ok(s.clone());
    }
    let acc = iure_account(state)?;
    let saved = secrets::leer(&acc, secrets::Kind::Session)
        .ok()
        .flatten()
        .and_then(|j| serde_json::from_str::<SessionExport>(&j).ok())
        .ok_or_else(|| "Inicia sesión en Iurefficient desde Ajustes".to_string())?;
    let sess = Session::new(acc.clone(), &iure_user_agent()).map_err(|e| e.to_string())?;
    sess.import(&saved).await.map_err(|e| format!("{e:#}"))?;
    persist_session(&acc, &sess);
    let sess = Arc::new(sess);
    *state.iure_session.lock().await = Some(sess.clone());
    Ok(sess)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IureLoginResult {
    logged_in: bool,
    requires_totp: bool,
    totp_token: Option<String>,
    name: Option<String>,
}

fn display_name(u: &iurefficient_connect::rest::User) -> Option<String> {
    u.name
        .clone()
        .or_else(|| {
            u.extra
                .get("full_name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
        })
        .filter(|n| !n.trim().is_empty())
}

#[tauri::command]
async fn iure_login(
    state: State<'_, AppState>,
    password: String,
    totp_code: Option<String>,
    totp_token: Option<String>,
) -> Result<IureLoginResult, String> {
    let acc = iure_account(&state)?;
    let sess = Session::new(acc.clone(), &iure_user_agent()).map_err(|e| e.to_string())?;
    let user = match (totp_token, totp_code) {
        (Some(t), Some(code)) if !t.is_empty() => sess
            .verify_totp(&t, &code)
            .await
            .map_err(|e| format!("{e:#}"))?,
        _ => match sess.login(&password).await.map_err(|e| format!("{e:#}"))? {
            Login::Ok(u) => u,
            Login::TotpRequired { totp_token } => {
                return Ok(IureLoginResult {
                    logged_in: false,
                    requires_totp: true,
                    totp_token: Some(totp_token),
                    name: None,
                });
            }
        },
    };
    persist_session(&acc, &sess);
    let _ = iurefficient_connect::account::set_active(&acc, APP_NAME);
    *state.iure_session.lock().await = Some(Arc::new(sess));
    let name = display_name(&user).or(Some(user.email.clone()));
    Ok(IureLoginResult {
        logged_in: true,
        requires_totp: false,
        totp_token: None,
        name,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IureSessionStatus {
    logged_in: bool,
    name: Option<String>,
    email: Option<String>,
    terminology: Option<api::Terminology>,
    error: Option<String>,
}

#[tauri::command]
async fn iure_session_status(state: State<'_, AppState>) -> Result<IureSessionStatus, String> {
    let off = |error: Option<String>| IureSessionStatus {
        logged_in: false,
        name: None,
        email: None,
        terminology: None,
        error,
    };
    let sess = match iure_session(&state).await {
        Ok(s) => s,
        Err(e) => return Ok(off(Some(e))),
    };
    match sess.me().await {
        Ok(u) => {
            let terminology = api::terminology(&sess).await.ok();
            Ok(IureSessionStatus {
                logged_in: true,
                name: display_name(&u),
                email: Some(u.email),
                terminology,
                error: None,
            })
        }
        Err(e) => {
            *state.iure_session.lock().await = None;
            Ok(off(Some(format!("{e:#}"))))
        }
    }
}

#[tauri::command]
async fn iure_logout(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(s) = state.iure_session.lock().await.take() {
        let _ = s.logout().await;
    }
    if let Ok(acc) = iure_account(&state) {
        let _ = secrets::borrar(&acc, secrets::Kind::Session);
        let _ = iurefficient_connect::account::clear_active(&acc);
    }
    Ok(())
}

fn hostname_label() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(|| std::env::var("COMPUTERNAME").ok())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|h| h.trim().to_string())
        })
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "este equipo".into())
}

/// Garantiza una contraseña de aplicación WebDAV en el llavero (la crea con la sesión).
#[tauri::command]
async fn iure_ensure_webdav_password(state: State<'_, AppState>) -> Result<bool, String> {
    let acc = iure_account(&state)?;
    if secrets::leer(&acc, secrets::Kind::WebDav)
        .ok()
        .flatten()
        .filter(|p| !p.trim().is_empty())
        .is_some()
    {
        return Ok(false);
    }
    let sess = iure_session(&state).await?;
    let created =
        api::create_webdav_token(&sess, &format!("{APP_NAME} en {}", hostname_label()), None)
            .await
            .map_err(|e| format!("{e:#}"))?;
    if secrets::guardar(&acc, secrets::Kind::WebDav, &created.secret).is_err() {
        let mut s = state.settings.lock().unwrap();
        s.iure_app_password = created.secret;
        let _ = s.save(&state.settings_path);
    }
    Ok(true)
}

#[tauri::command]
async fn iure_test_connection(
    state: State<'_, AppState>,
) -> Result<webdav::ConnectionInfo, String> {
    let dav = iure_webdav(&state)?;
    dav.test_connection().await.map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn iure_list(state: State<'_, AppState>, folder: String) -> Result<webdav::Listing, String> {
    let dav = iure_webdav(&state)?;
    dav.list(&folder).await.map_err(|e| format!("{e:#}"))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct IureUploadProgress {
    job_id: String,
    file_name: String,
    index: usize,
    total_files: usize,
    sent: u64,
    total: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IureUploadResult {
    /// Carpeta WebDAV o título del proyecto.
    target: String,
    web_url: String,
    file_names: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IureUploadRequest {
    job_id: String,
    /// "folder" (WebDAV) o "case" (API REST).
    mode: String,
    /// Carpeta WebDAV, o id del proyecto (None = sin proyecto).
    folder: Option<String>,
    case_id: Option<String>,
    case_title: Option<String>,
    files: Vec<String>,
}

#[tauri::command]
async fn iure_upload(
    app: AppHandle,
    state: State<'_, AppState>,
    request: IureUploadRequest,
) -> Result<IureUploadResult, String> {
    let total_files = request.files.len();
    let mut names = Vec::new();
    if request.mode == "case" {
        let sess = iure_session(&state).await?;
        for (index, f) in request.files.iter().enumerate() {
            let local = PathBuf::from(f);
            let name = local
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let file_name = webdav::fallback_name(&name).unwrap_or(name);
            let _ = app.emit(
                "iure-upload-progress",
                IureUploadProgress {
                    job_id: request.job_id.clone(),
                    file_name: file_name.clone(),
                    index,
                    total_files,
                    sent: 0,
                    total: 0,
                },
            );
            let opts = api::UploadOptions {
                case_id: request.case_id.clone(),
                file_name: Some(file_name.clone()),
                document_type: Some("other".into()),
                tags: vec!["iureocr".into()],
                ..Default::default()
            };
            api::upload_document(&sess, &local, &opts)
                .await
                .map_err(|e| format!("{file_name}: {e:#}"))?;
            names.push(file_name);
        }
        return Ok(IureUploadResult {
            target: request.case_title.unwrap_or_else(|| "sin proyecto".into()),
            web_url: sess.account().web_url(),
            file_names: names,
        });
    }
    let dav = iure_webdav(&state)?;
    let folder = request.folder.clone().unwrap_or_default();
    for (index, f) in request.files.iter().enumerate() {
        let local = PathBuf::from(f);
        let file_name = local
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let app2 = app.clone();
        let job_id = request.job_id.clone();
        let fname = file_name.clone();
        let last = Arc::new(Mutex::new(std::time::Instant::now()));
        let progress = move |sent: u64, total: u64| {
            let mut last = last.lock().unwrap();
            if last.elapsed().as_millis() > 120 || sent == total {
                *last = std::time::Instant::now();
                let _ = app2.emit(
                    "iure-upload-progress",
                    IureUploadProgress {
                        job_id: job_id.clone(),
                        file_name: fname.clone(),
                        index,
                        total_files,
                        sent,
                        total,
                    },
                );
            }
        };
        let up = dav
            .upload_with_fallback(&folder, &local, progress)
            .await
            .map_err(|e| format!("{e:#}"))?;
        names.push(up.file_name);
    }
    {
        let mut s = state.settings.lock().unwrap();
        s.iure_last_folder = Some(folder.clone());
        let _ = s.save(&state.settings_path);
    }
    Ok(IureUploadResult {
        target: if folder.is_empty() {
            "la raíz".into()
        } else {
            folder
        },
        web_url: dav.acc.web_url(),
        file_names: names,
    })
}

#[tauri::command]
async fn iure_search_cases(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<api::CaseSummary>, String> {
    let sess = iure_session(&state).await?;
    api::cases(&sess, Some(&query), 50)
        .await
        .map_err(|e| format!("{e:#}"))
}

// ---------------------------------------------------------------------------
// Apps hermanas y OnlyOffice
// ---------------------------------------------------------------------------
#[tauri::command]
async fn apps_status(with_network: bool) -> Vec<iurefficient_connect::apps::AppStatus> {
    if with_network {
        iurefficient_connect::apps::status(&iure_user_agent()).await
    } else {
        iurefficient_connect::apps::installed()
    }
}

#[tauri::command]
fn launch_app(app: String) -> Result<(), String> {
    let id = iurefficient_connect::apps::AppId::parse(&app)
        .ok_or_else(|| format!("app desconocida: {app}"))?;
    iurefficient_connect::apps::launch(id, &[]).map_err(|e| format!("{e:#}"))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OnlyOfficeStatus {
    installed: bool,
    path: Option<String>,
    download_url: &'static str,
}

/// OnlyOffice Desktop Editors, para editar el PDF o el documento: no lo incluimos,
/// sólo lo detectamos y, si falta, enlazamos su descarga.
fn onlyoffice_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "linux")]
    {
        candidates.push("/usr/bin/onlyoffice-desktopeditors".into());
        candidates.push("/opt/onlyoffice/desktopeditors/DesktopEditors".into());
        candidates.push("/snap/bin/onlyoffice-desktopeditors".into());
        candidates.push("/var/lib/flatpak/exports/bin/org.onlyoffice.desktopeditors".into());
        if let Some(h) = dirs::home_dir() {
            candidates
                .push(h.join(".local/share/flatpak/exports/bin/org.onlyoffice.desktopeditors"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        candidates.push("/Applications/ONLYOFFICE.app".into());
    }
    #[cfg(windows)]
    {
        candidates.push("C:\\Program Files\\ONLYOFFICE\\DesktopEditors\\DesktopEditors.exe".into());
        if let Some(l) = dirs::data_local_dir() {
            candidates.push(
                l.join("Programs")
                    .join("ONLYOFFICE")
                    .join("DesktopEditors")
                    .join("DesktopEditors.exe"),
            );
        }
    }
    candidates.into_iter().find(|p| p.exists())
}

#[tauri::command]
fn onlyoffice_status() -> OnlyOfficeStatus {
    let path = onlyoffice_path();
    OnlyOfficeStatus {
        installed: path.is_some(),
        path: path.map(|p| p.to_string_lossy().into_owned()),
        download_url: "https://www.onlyoffice.com/es/download-desktop.aspx",
    }
}

#[tauri::command]
fn open_with_onlyoffice(path: String) -> Result<(), String> {
    let exe = onlyoffice_path().ok_or("OnlyOffice no está instalado")?;
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg(&exe)
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        tesseract::hidden_command(&exe)
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Actualizaciones (aviso con enlace cuando el actualizador no puede instalar)
// ---------------------------------------------------------------------------
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateNotice {
    version: String,
    url: String,
}

#[tauri::command]
async fn check_update_notice() -> Result<Option<UpdateNotice>, String> {
    let r = iurefficient_connect::releases::consultar(
        "ellaguno/iureocr",
        env!("CARGO_PKG_VERSION"),
        &iure_user_agent(),
    )
    .await
    .map_err(|e| format!("{e:#}"))?;
    Ok(r.map(|r| UpdateNotice {
        version: r.version,
        url: r.url,
    }))
}

// ---------------------------------------------------------------------------
// Arranque
// ---------------------------------------------------------------------------
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logging();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
            let args: Vec<String> = argv
                .into_iter()
                .skip(1)
                .filter(|a| !a.starts_with('-') && !a.contains("://"))
                .collect();
            if !args.is_empty() {
                let _ = app.emit("launch-args", args);
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            #[cfg(any(windows, target_os = "linux"))]
            {
                use tauri_plugin_deep_link::DeepLinkExt;
                if let Err(e) = app.deep_link().register_all() {
                    log::warn!("no se pudo registrar el esquema iureocr://: {e}");
                }
            }
            let config_dir = app.path().app_config_dir()?;
            let settings_path = config_dir.join("settings.json");
            let mut settings = Settings::load(&settings_path);
            // Sin cuenta configurada: la que dejó otra app de Iurefficient en este equipo.
            if settings.iure_domain.trim().is_empty() || settings.iure_email.trim().is_empty() {
                if let Some(a) = iurefficient_connect::account::active() {
                    log::info!(
                        "cuenta de Iurefficient tomada de la cuenta activa compartida (la dejó {})",
                        a.app
                    );
                    settings.iure_domain = a.domain;
                    settings.iure_email = a.email;
                    let _ = settings.save(&settings_path);
                }
            }
            let resource_dir = app.path().resource_dir().ok();
            match tesseract::locate(resource_dir.as_deref()) {
                Ok(t) => log::info!(
                    "Tesseract {} en {} (idiomas: {}){}",
                    t.version,
                    t.exe,
                    t.languages.join(", "),
                    if t.bundled { ", empaquetado" } else { "" }
                ),
                Err(e) => log::warn!("{e}"),
            }
            app.manage(AppState {
                settings_path,
                settings: Arc::new(Mutex::new(settings)),
                resource_dir,
                jobs: Mutex::new(HashMap::new()),
                ocr: ocr::Worker::new(),
                iure_session: tokio::sync::Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            system_info,
            reveal_path,
            read_text_file,
            file_size,
            launch_args,
            ocr_start,
            ocr_cancel,
            iure_login,
            iure_session_status,
            iure_logout,
            iure_ensure_webdav_password,
            iure_test_connection,
            iure_list,
            iure_upload,
            iure_search_cases,
            apps_status,
            launch_app,
            onlyoffice_status,
            open_with_onlyoffice,
            check_update_notice,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar IureOCR");
}
