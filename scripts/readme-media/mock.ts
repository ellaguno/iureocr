// Backend simulado para capturar la interfaz real de IureOCR en un navegador.
// Idioma: app.html?lang=es | app.html?lang=en; tema oscuro: &theme=dark
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { DIR, HERO, HERO_NAME, OTHERS } from "./docs.mjs";

const lang: "es" | "en" = new URLSearchParams(location.search).get("lang") === "es" ? "es" : "en";
const SCANS = `/scripts/readme-media/scans/${lang}/`;
const SUFFIX = " - OCR";

mockWindows("main");

const settings: any = {
  languages: "spa+eng", dpi: 300, jpegQuality: 85, outputMode: "same", outputDir: null, suffix: SUFFIX,
  skipIfText: true, autoOcr: false, theme: new URLSearchParams(location.search).get("theme") === "dark" ? "dark" : "light", uiLanguage: lang, checkUpdates: true,
  iureDomain: "demo.iurefficient.com", iureEmail: "ana.torres@example.com", iureAppPassword: "", iureLastFolder: null,
};

// Documentos: ruta -> páginas (URL de la imagen de cada página) y tamaño en bytes.
const docs: Record<string, { pages: string[]; size: number; image?: boolean }> = {};
const heroPath = `${DIR[lang]}/${HERO_NAME[lang]}`;
const heroPages = HERO[lang].pages.map((_: unknown, i: number) => `${SCANS}hero-${i + 1}.jpg`);
docs[heroPath] = { pages: heroPages, size: 3_480_000 };
docs[heroPath.replace(/\.pdf$/, `${SUFFIX}.pdf`)] = { pages: heroPages, size: 3_610_000 };
const paths: Record<string, string> = { hero: heroPath };
for (const d of OTHERS[lang]) {
  const p = `${DIR[lang]}/${d.name}`;
  paths[d.id] = p;
  const pages = Array.from({ length: d.pages }, (_, i) => (i === 0 ? `${SCANS}${d.id}-1.jpg` : `${SCANS}filler-${((i - 1) % 3) + 1}.jpg`));
  docs[p] = { pages, size: d.size, image: d.image };
  docs[p.replace(/\.pdf$/, `${SUFFIX}.pdf`)] = { pages, size: d.size };
}
const heroText = await fetch(`${SCANS}hero.txt`).then((r) => r.text());

let pending: { jobId: string; path: string; resolve: (v: any) => void } | null = null;

const tesseract = { exe: "/usr/bin/tesseract", tessdata: "/usr/lib/IureOCR/resources/tessdata", version: "5.3.4", languages: ["eng", "osd", "spa"], bundled: false };

mockIPC(
  (cmd, args: any) => {
    switch (cmd) {
      case "ui_language": return lang;
      case "get_settings": return structuredClone(settings);
      case "set_settings": Object.assign(settings, args.settings); return structuredClone(settings);
      case "system_info":
        return { version: "0.5.1", platform: "linux", tesseract, tesseractError: null, pdfiumBundled: true, settingsPath: "/home/demo/.config/com.iurefficient.iureocr/settings.json", logPath: "/home/demo/.local/share/com.iurefficient.iureocr/logs/iureocr.log", updateTarget: "linux-deb", supportedExtensions: ["pdf", "jpg", "jpeg", "png", "tif", "tiff", "bmp", "webp"] };
      case "file_size": return docs[args.path]?.size ?? 1_000_000;
      case "pdf_page_count": return docs[args.path]?.pages.length ?? 1;
      case "pdf_page_sizes": { const d = docs[args.path]; return (d?.pages ?? [""]).map(() => (d?.image ? [2480, 3508] : [595, 842])); }
      case "pdf_thumbnails": return args.indices.map((i: number) => docs[args.path]?.pages[i] ?? "");
      case "read_text_file": return heroText;
      case "launch_args": return [];
      case "ocr_start": return new Promise((resolve) => { pending = { jobId: args.jobId, path: args.path, resolve }; });
      case "ocr_cancel": return null;
      case "pdf_edit_pages": return { path: args.path.replace(/\.pdf$/, " - rotated.pdf"), pages: docs[args.path]?.pages.length ?? 1 };
      case "iure_session_status": return { loggedIn: true, name: "Ana Torres", email: "ana.torres@example.com", terminology: null, error: null };
      case "apps_status":
        return [
          { id: "transcribe", name: "IureTranscribe", description: "", installed: true, path: "/usr/bin/iuretranscribe", downloadUrl: "", latestVersion: null },
          { id: "editor", name: "iureditor", description: "", installed: true, path: "/usr/bin/iureditor", downloadUrl: "", latestVersion: null },
          { id: "dav", name: "IureDav", description: "", installed: false, path: null, downloadUrl: "https://github.com/ellaguno/iuredav/releases/latest", latestVersion: null },
        ];
      case "onlyoffice_status": return { installed: true, path: "/usr/bin/onlyoffice-desktopeditors", downloadUrl: "" };
      case "agents_status":
        return {
          claudeDesktop: { installed: true, connected: false, stale: false, configPath: "/home/demo/.config/Claude/claude_desktop_config.json", downloadUrl: "https://claude.ai/download" },
          vscode: { installed: true, connected: true, stale: false, configPath: null, downloadUrl: "https://code.visualstudio.com/download", copilot: true },
        };
      case "copilot_status":
        return {
          signedIn: true, endpointUrl: "https://demo.iurefficient.com/mcp", manageUrl: "https://demo.iurefficient.com/settings/mcp", error: null,
          tokens: [{ id: "t1", name: "Microsoft 365 Copilot", tokenPrefix: "iurmcp_7Hq2", isValid: true, createdAt: "2026-09-29T16:20:00Z", expiresAt: "2027-09-29T16:20:00Z", lastUsedAt: "2026-10-01T10:05:00Z", callCount: 37 }],
        };
      case "check_update_notice": return null;
      case "plugin:deep-link|get_current": return null;
      case "plugin:notification|is_permission_granted": return true;
      default:
        if (cmd.startsWith("plugin:")) return null;
        console.log("mock: no response for", cmd);
        return null;
    }
  },
  { shouldMockEvents: true },
);

const w = window as any;
w.__lang = lang;
w.__paths = paths;
w.__progress = (stage: "render" | "ocr", current: number, total: number) => pending && emit("ocr-progress", { jobId: pending.jobId, stage, current, total });
w.__finish = (chars: number, elapsedSecs: number) => {
  if (!pending) return;
  const p = pending;
  pending = null;
  const pdf = p.path.replace(/\.pdf$/, `${SUFFIX}.pdf`);
  p.resolve({ skipped: false, pdfPath: pdf, txtPath: pdf.replace(/\.pdf$/, ".txt"), pages: docs[p.path].pages.length, chars, elapsedSecs, note: null });
};
w.__heroChars = heroText.replace(/\s+/g, " ").trim().length;

await import("../../src/main.ts");
const state = await import("../../src/lib/state.svelte.ts");
w.__state = state;
