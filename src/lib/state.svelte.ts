import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import {
  api,
  type AppId,
  type AppStatus,
  type IureSessionStatus,
  type IureUploadProgress,
  type OcrOutcome,
  type OcrProgress,
  type OnlyOfficeStatus,
  type Settings,
  type SystemInfo,
} from "./api";
import { baseName } from "./format";
import { i18n, syncLanguage, t, tn } from "./i18n.svelte";

export type View = "ocr" | "settings";
/** `ready`: archivo listo sin pasar por OCR (p. ej. resultado de una edición de páginas). */
export type JobStatus = "queued" | "render" | "ocr" | "done" | "skipped" | "error" | "cancelled" | "ready";

export interface Job {
  id: string;
  path: string;
  name: string;
  sizeBytes: number | null;
  status: JobStatus;
  current: number;
  total: number;
  force: boolean;
  /** Páginas del archivo (null hasta contarlas; 1 para imágenes). */
  pages: number | null;
  /** Tamaño (ancho, alto) de cada página, para el visor. */
  pageSizes: [number, number][];
  /** Miniaturas por índice de página (data URL), cargadas bajo demanda. */
  thumbs: Record<number, string>;
  error?: string;
  result?: OcrOutcome;
  /** Subida en curso a Iurefficient. */
  upload?: IureUploadProgress | null;
  /** Dónde quedó guardado en Iurefficient. */
  saved?: { target: string; webUrl: string; fileNames: string[]; at: number };
  startedAt?: number;
  finishedAt?: number;
}

export interface Toast {
  id: number;
  kind: "info" | "success" | "error";
  text: string;
}

export const app = $state({
  ready: false,
  view: "ocr" as View,
  settings: null as Settings | null,
  sys: null as SystemInfo | null,
  jobs: [] as Job[],
  selectedJobId: null as string | null,
  toasts: [] as Toast[],
  dragging: false,
  running: false,
  iureSession: null as IureSessionStatus | null,
  updateNotice: null as { version: string; url: string } | null,
  apps: null as AppStatus[] | null,
  onlyoffice: null as OnlyOfficeStatus | null,
});

let toastSeq = 0;
let stopRequested = false;
let notifyOk: boolean | null = null;

export function toast(text: string, kind: Toast["kind"] = "info", ms = 4500) {
  const id = ++toastSeq;
  app.toasts.push({ id, kind, text });
  setTimeout(() => {
    const i = app.toasts.findIndex((t) => t.id === id);
    if (i >= 0) app.toasts.splice(i, 1);
  }, ms);
}

export async function notify(title: string, body: string) {
  try {
    if (notifyOk === null) notifyOk = (await isPermissionGranted()) || (await requestPermission()) === "granted";
    if (notifyOk && !document.hasFocus()) sendNotification({ title, body });
  } catch {
    /* sin notificaciones del sistema */
  }
}

export function applyTheme() {
  const pref = app.settings?.theme ?? "system";
  const dark = pref === "dark" || (pref === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

export async function saveSettings(patch: Partial<Settings>) {
  if (!app.settings) return;
  const prevLang = app.settings.uiLanguage;
  app.settings = await api.setSettings({ ...app.settings, ...patch });
  applyTheme();
  if (app.settings.uiLanguage !== prevLang) await onLanguageChanged();
}

/** Cambió el idioma: se aplica al momento y se refrescan los textos que vienen del backend. */
async function onLanguageChanged() {
  await syncLanguage();
  await Promise.allSettled([refreshSystem(), refreshApps(false), refreshIureSession()]);
}

export async function refreshSystem() {
  app.sys = await api.systemInfo();
}

// ---------------------------------------------------------------------------
// Cola de OCR
// ---------------------------------------------------------------------------
function supported(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return (app.sys?.supportedExtensions ?? ["pdf"]).includes(ext);
}

export async function addFiles(paths: string[]): Promise<Job[]> {
  const added: Job[] = [];
  let ignored = 0;
  for (const p of paths) {
    if (!supported(p)) {
      ignored++;
      continue;
    }
    if (app.jobs.some((j) => j.path === p && j.status !== "done" && j.status !== "error" && j.status !== "cancelled")) continue;
    const job = newJob(p, app.settings?.autoOcr ? "queued" : "ready");
    added.push(job);
  }
  if (ignored) toast(tn("state.ignored", ignored), "info");
  if (added.length && !app.selectedJobId) app.selectedJobId = added[0].id;
  if (added.length && app.settings?.autoOcr) void startQueue();
  return added;
}

/**
 * Crea el trabajo y lo mete en la lista. Devuelve la copia **reactiva** (la que vive
 * en `app.jobs`): escribir en el objeto original no refresca la interfaz.
 */
function newJob(path: string, status: JobStatus, atStart = false): Job {
  const plain: Job = { id: crypto.randomUUID(), path, name: baseName(path), sizeBytes: null, status, current: 0, total: 0, force: false, pages: null, pageSizes: [], thumbs: {} };
  if (atStart) app.jobs.unshift(plain);
  else app.jobs.push(plain);
  const job = app.jobs.find((j) => j.id === plain.id)!;
  api.fileSize(path).then((n) => (job.sizeBytes = n)).catch(() => {});
  api.pdfPageSizes(path)
    .then((sizes) => {
      job.pageSizes = sizes;
      job.pages = sizes.length;
    })
    .then(() => loadThumbs(job, 0, 6))
    .catch((e) => {
      job.pages = 0;
      toast(`${job.name}: ${e}`, "error", 8000);
    });
  return job;
}

/** Pide el OCR de un archivo listo (o repite uno terminado). */
export function requestOcr(id: string, force = false) {
  const job = app.jobs.find((j) => j.id === id);
  if (!job || isActive(job)) return;
  job.status = "queued";
  job.force = force;
  job.error = undefined;
  void startQueue();
}

/** Archivos listos que aún no han pasado por OCR. */
export function pendingOcr(): Job[] {
  return app.jobs.filter((j) => j.status === "ready");
}

/** Ruta del PDF que representa al trabajo: el resultado del OCR si lo hay, si no el original. */
export function pdfOf(job: Job): string {
  return job.result?.pdfPath ?? job.path;
}

export const THUMB_WIDTH = 160;

/** Carga (si faltan) las miniaturas de las páginas [from, to) del archivo original. */
export async function loadThumbs(job: Job, from: number, to: number): Promise<void> {
  const total = job.pages ?? 0;
  const indices: number[] = [];
  for (let i = from; i < Math.min(to, total); i++) if (!job.thumbs[i]) indices.push(i);
  if (!indices.length) return;
  try {
    const imgs = await api.pdfThumbnails(job.path, indices, THUMB_WIDTH);
    indices.forEach((i, k) => (job.thumbs[i] = imgs[k]));
  } catch (e) {
    console.warn("miniaturas:", e);
  }
}

/** Añade a la lista un archivo ya listo (resultado de una edición de páginas). */
export function addReadyFile(path: string): Job {
  const job = newJob(path, "ready", true);
  app.selectedJobId = job.id;
  return job;
}

export function isActive(job: Job): boolean {
  return job.status === "render" || job.status === "ocr";
}

export function queuedCount(): number {
  return app.jobs.filter((j) => j.status === "queued").length;
}

export async function startQueue() {
  if (app.running) return;
  if (!app.sys?.tesseract) {
    toast(app.sys?.tesseractError ?? t("state.tesseractUnavailable"), "error", 9000);
    return;
  }
  app.running = true;
  stopRequested = false;
  try {
    while (!stopRequested) {
      const next = app.jobs.find((j) => j.status === "queued");
      if (!next) break;
      await runJob(next);
    }
  } finally {
    app.running = false;
  }
}

export function stopQueue() {
  stopRequested = true;
}

async function runJob(job: Job) {
  job.status = "render";
  job.current = 0;
  job.total = 0;
  job.error = undefined;
  job.startedAt = Date.now();
  try {
    const out = await api.ocrStart(job.id, job.path, job.force);
    job.result = out;
    job.status = out.skipped ? "skipped" : "done";
    job.finishedAt = Date.now();
    if (!out.skipped) {
      const done = app.jobs.filter((j) => j.status === "done").length;
      if (queuedCount() === 0) notify("IureOCR", tn("state.notifyDone", done));
    }
  } catch (e) {
    const msg = String(e);
    // El backend responde «Cancelled» o «Cancelado» según el idioma.
    job.status = msg.includes("Cancelado") || msg.includes("Cancelled") ? "cancelled" : "error";
    job.error = msg;
    job.finishedAt = Date.now();
    if (job.status === "error") toast(`${job.name}: ${msg}`, "error", 9000);
  }
}

export async function cancelJob(id: string) {
  const job = app.jobs.find((j) => j.id === id);
  if (!job) return;
  if (isActive(job)) await api.ocrCancel(id);
  else if (job.status === "queued") job.status = "cancelled";
}

export function retryJob(id: string, force = false) {
  const job = app.jobs.find((j) => j.id === id);
  if (!job) return;
  job.status = "queued";
  job.force = force || job.force;
  job.error = undefined;
  job.result = undefined;
  job.saved = undefined;
  void startQueue();
}

export function removeJob(id: string) {
  const i = app.jobs.findIndex((j) => j.id === id);
  if (i < 0) return;
  if (isActive(app.jobs[i])) return;
  app.jobs.splice(i, 1);
  if (app.selectedJobId === id) app.selectedJobId = app.jobs[0]?.id ?? null;
}

export function clearFinished() {
  app.jobs = app.jobs.filter((j) => j.status === "queued" || isActive(j) || j.status === "ready");
  if (!app.jobs.some((j) => j.id === app.selectedJobId)) app.selectedJobId = app.jobs[0]?.id ?? null;
}

// ---------------------------------------------------------------------------
// Iurefficient
// ---------------------------------------------------------------------------
export function iureConfigured(): boolean {
  const s = app.settings;
  return !!(s && s.iureDomain.trim() && s.iureEmail.trim());
}

export function iureLoggedIn(): boolean {
  return !!app.iureSession?.loggedIn;
}

export async function refreshIureSession() {
  if (!iureConfigured()) {
    app.iureSession = { loggedIn: false, name: null, email: null, terminology: null, error: null };
    return;
  }
  try {
    app.iureSession = await api.iureSessionStatus();
  } catch (e) {
    app.iureSession = { loggedIn: false, name: null, email: null, terminology: null, error: String(e) };
  }
}

export function term(key: "case" | "cases" | "client" | "clients"): string {
  // La terminología del servidor viene en español: en inglés se usan los términos genéricos.
  const custom = i18n.lang === "es" ? app.iureSession?.terminology?.[key] : undefined;
  const v = custom || t(`term.${key}`);
  return v.charAt(0).toUpperCase() + v.slice(1);
}

/** Archivos que se suben: el PDF con texto (y el .txt), o el archivo listo sin OCR. */
export function filesOf(job: Job, includeTxt: boolean): string[] {
  if (job.status === "ready") return [job.path];
  const r = job.result;
  if (!r || job.status !== "done") return [];
  const files = [r.pdfPath!];
  if (includeTxt && r.txtPath) files.push(r.txtPath);
  return files;
}

export async function uploadJob(job: Job, target: { mode: "folder"; folder: string } | { mode: "case"; caseId: string | null; caseTitle: string }, includeTxt: boolean): Promise<boolean> {
  const files = filesOf(job, includeTxt);
  if (!files.length) {
    toast(t("state.nothingToUpload"), "error");
    return false;
  }
  job.upload = { jobId: job.id, fileName: "", index: 0, totalFiles: files.length, sent: 0, total: 0 };
  try {
    const res = await api.iureUpload(target.mode === "folder" ? { jobId: job.id, mode: "folder", folder: target.folder, files } : { jobId: job.id, mode: "case", caseId: target.caseId, caseTitle: target.caseTitle, files });
    job.saved = { target: res.target, webUrl: res.webUrl, fileNames: res.fileNames, at: Date.now() };
    toast(tn("state.saved", res.fileNames.length, { target: res.target }), "success", 6000);
    return true;
  } catch (e) {
    toast(t("state.saveFailed", { error: String(e) }), "error", 9000);
    return false;
  } finally {
    job.upload = null;
  }
}

// ---------------------------------------------------------------------------
// Apps hermanas, argumentos de arranque y enlaces iureocr://
// ---------------------------------------------------------------------------
export function appStatus(id: AppId): AppStatus | undefined {
  return app.apps?.find((a) => a.id === id);
}

export async function refreshApps(withNetwork: boolean): Promise<void> {
  try {
    app.apps = await api.appsStatus(withNetwork);
  } catch (e) {
    console.warn("apps_status:", e);
  }
  try {
    app.onlyoffice = await api.onlyofficeStatus();
  } catch {
    /* sin detección */
  }
}

/** Rutas de archivo o enlaces `iureocr://ocr?file=…` con los que se abrió la app. */
export async function handleLaunchArgs(args: string[]) {
  const paths: string[] = [];
  for (const a of args) {
    if (a.startsWith("iureocr://")) {
      try {
        const u = new URL(a);
        for (const f of u.searchParams.getAll("file")) paths.push(f);
      } catch {
        /* enlace mal formado */
      }
    } else if (!a.startsWith("-")) {
      paths.push(a);
    }
  }
  if (paths.length) {
    app.view = "ocr";
    await addFiles(paths);
  }
}

// ---------------------------------------------------------------------------
// Arranque
// ---------------------------------------------------------------------------
export async function init() {
  await syncLanguage();
  const [settings, sys] = await Promise.all([api.getSettings(), api.systemInfo()]);
  app.settings = settings;
  app.sys = sys;
  applyTheme();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);
  refreshIureSession();
  if (settings.checkUpdates) {
    setTimeout(() => {
      import("./updater").then((m) => m.checkForUpdates(true)).catch(() => {});
    }, 6000);
  }

  await listen<OcrProgress>("ocr-progress", (e) => {
    const job = app.jobs.find((j) => j.id === e.payload.jobId);
    if (!job || !isActive(job)) return;
    job.status = e.payload.stage;
    job.current = e.payload.current;
    job.total = e.payload.total;
  });
  await listen<IureUploadProgress>("iure-upload-progress", (e) => {
    const job = app.jobs.find((j) => j.id === e.payload.jobId);
    if (job && job.upload) job.upload = e.payload;
  });
  await getCurrentWebview().onDragDropEvent((event) => {
    const t = event.payload.type;
    if (t === "enter" || t === "over") app.dragging = true;
    else if (t === "leave") app.dragging = false;
    else if (t === "drop") {
      app.dragging = false;
      app.view = "ocr";
      addFiles(event.payload.paths);
    }
  });
  app.ready = true;
  refreshApps(false);
  await listen<string[]>("launch-args", (e) => void handleLaunchArgs(e.payload));
  try {
    const { getCurrent, onOpenUrl } = await import("@tauri-apps/plugin-deep-link");
    await onOpenUrl((urls) => void handleLaunchArgs(urls));
    const current = await getCurrent();
    if (current?.length) await handleLaunchArgs(current);
  } catch {
    /* sin deep link en esta plataforma */
  }
  try {
    await handleLaunchArgs(await api.launchArgs());
  } catch {
    /* sin argumentos */
  }
}
