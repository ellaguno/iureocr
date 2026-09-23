import { invoke } from "@tauri-apps/api/core";

export interface Settings {
  languages: string;
  dpi: number;
  jpegQuality: number;
  outputMode: "same" | "custom";
  outputDir: string | null;
  suffix: string;
  skipIfText: boolean;
  autoOcr: boolean;
  theme: "system" | "light" | "dark";
  checkUpdates: boolean;
  iureDomain: string;
  iureEmail: string;
  iureAppPassword: string;
  iureLastFolder: string | null;
}

export interface TesseractInfo {
  exe: string;
  tessdata: string | null;
  version: string;
  languages: string[];
  bundled: boolean;
}

export interface SystemInfo {
  version: string;
  platform: string;
  tesseract: TesseractInfo | null;
  tesseractError: string | null;
  pdfiumBundled: boolean;
  settingsPath: string;
  logPath: string | null;
  updateTarget: string;
  supportedExtensions: string[];
}

export interface OcrOutcome {
  skipped: boolean;
  pdfPath: string | null;
  txtPath: string | null;
  pages: number;
  chars: number;
  elapsedSecs: number;
  note: string | null;
}

export interface OcrProgress {
  jobId: string;
  stage: "render" | "ocr";
  current: number;
  total: number;
}

export interface PdfEditResult {
  path: string;
  pages: number;
}

export interface Terminology {
  case: string;
  cases: string;
  client: string;
  clients: string;
}

export interface IureSessionStatus {
  loggedIn: boolean;
  name: string | null;
  email: string | null;
  terminology: Terminology | null;
  error: string | null;
}

export interface IureLoginResult {
  loggedIn: boolean;
  requiresTotp: boolean;
  totpToken: string | null;
  name: string | null;
}

export interface IureEntry {
  name: string;
  path: string;
  isFolder: boolean;
  canUpload: boolean;
  size: number | null;
}

export interface IureListing {
  path: string;
  canUpload: boolean;
  entries: IureEntry[];
}

export interface IureConnectionInfo {
  webUrl: string;
  rootFolders: string[];
}

export interface IureCase {
  id: string;
  caseNumber: string;
  title: string;
}

export interface IureUploadProgress {
  jobId: string;
  fileName: string;
  index: number;
  totalFiles: number;
  sent: number;
  total: number;
}

export interface IureUploadResult {
  target: string;
  webUrl: string;
  fileNames: string[];
}

export type AppId = "transcribe" | "editor" | "dav" | "ocr";

export interface AppStatus {
  id: AppId;
  name: string;
  description: string;
  installed: boolean;
  path: string | null;
  downloadUrl: string;
  latestVersion: string | null;
}

export interface OnlyOfficeStatus {
  installed: boolean;
  path: string | null;
  downloadUrl: string;
}

export interface UpdateNotice {
  version: string;
  url: string;
}

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  setSettings: (settings: Settings) => invoke<Settings>("set_settings", { settings }),
  systemInfo: () => invoke<SystemInfo>("system_info"),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  readTextFile: (path: string, maxChars?: number) => invoke<string>("read_text_file", { path, maxChars: maxChars ?? null }),
  fileSize: (path: string) => invoke<number>("file_size", { path }),
  launchArgs: () => invoke<string[]>("launch_args"),
  ocrStart: (jobId: string, path: string, force: boolean) => invoke<OcrOutcome>("ocr_start", { jobId, path, force }),
  ocrCancel: (jobId: string) => invoke<void>("ocr_cancel", { jobId }),
  pdfPageCount: (path: string) => invoke<number>("pdf_page_count", { path }),
  /** Tamaño (ancho, alto) de cada página: puntos en PDF, píxeles en imágenes. */
  pdfPageSizes: (path: string) => invoke<[number, number][]>("pdf_page_sizes", { path }),
  /** Miniaturas JPEG (data URL) de las páginas `indices` (desde 0); una imagen devuelve una sola. */
  pdfThumbnails: (path: string, indices: number[], width: number) => invoke<string[]>("pdf_thumbnails", { path, indices, width }),
  /** Quitar, conservar o rotar páginas (1-based); escribe un PDF nuevo junto al original. */
  pdfEditPages: (path: string, op: "delete" | "keep" | "rotate" | "reorder", pages: number[], degrees?: number) =>
    invoke<PdfEditResult>("pdf_edit_pages", { path, op, pages, degrees: degrees ?? null }),
  iureLogin: (password: string, totpCode?: string, totpToken?: string) =>
    invoke<IureLoginResult>("iure_login", { password, totpCode: totpCode ?? null, totpToken: totpToken ?? null }),
  iureSessionStatus: () => invoke<IureSessionStatus>("iure_session_status"),
  iureLogout: () => invoke<void>("iure_logout"),
  iureEnsureWebdavPassword: () => invoke<boolean>("iure_ensure_webdav_password"),
  iureTestConnection: () => invoke<IureConnectionInfo>("iure_test_connection"),
  iureList: (folder: string) => invoke<IureListing>("iure_list", { folder }),
  iureUpload: (request: { jobId: string; mode: "folder" | "case"; folder?: string; caseId?: string | null; caseTitle?: string; files: string[] }) =>
    invoke<IureUploadResult>("iure_upload", { request }),
  iureSearchCases: (query: string) => invoke<IureCase[]>("iure_search_cases", { query }),
  appsStatus: (withNetwork: boolean) => invoke<AppStatus[]>("apps_status", { withNetwork }),
  launchApp: (app: AppId) => invoke<void>("launch_app", { app }),
  onlyofficeStatus: () => invoke<OnlyOfficeStatus>("onlyoffice_status"),
  openWithOnlyoffice: (path: string) => invoke<void>("open_with_onlyoffice", { path }),
  checkUpdateNotice: () => invoke<UpdateNotice | null>("check_update_notice"),
};
