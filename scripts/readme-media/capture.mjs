// Conduce la interfaz real de IureOCR (Vite + backend simulado en mock.ts) y guarda:
//   out/<lang>/hero/NNN.png + hero.txt (lista de cuadros con su duración, para ffmpeg)
//   out/<lang>/{queue,result,pages,agents}.png
// Uso: node capture.mjs <es|en> [puerto]
import puppeteer from "puppeteer-core";
import fs from "node:fs";

const lang = process.argv[2] === "es" ? "es" : "en";
const PORT = process.argv[3] ?? "5304";
const OUT = new URL(`./out/${lang}/`, import.meta.url).pathname;
const CHROME = process.env.CHROME ?? process.env.HOME + "/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome";
fs.rmSync(OUT, { recursive: true, force: true });
fs.mkdirSync(OUT + "hero", { recursive: true });

const browser = await puppeteer.launch({ executablePath: CHROME, args: ["--no-sandbox", "--font-render-hinting=none", `--lang=${lang}`] });
const wait = (ms) => new Promise((r) => setTimeout(r, ms));

async function open(query = "") {
  const p = await browser.newPage();
  p.on("pageerror", (e) => console.log("ERR:", e.message));
  p.on("console", (m) => m.text().startsWith("mock:") && console.log(m.text()));
  await p.setViewport({ width: 1280, height: 800, deviceScaleFactor: 1.5 });
  await p.goto(`http://localhost:${PORT}/scripts/readme-media/app.html?lang=${lang}${query}`, { waitUntil: "networkidle0" });
  await p.waitForFunction(() => window.__state?.app.ready);
  // ::selection: el color de la selección con la ventana activa (en headless sale el de ventana inactiva).
  await p.addStyleTag({ content: "*{caret-color:transparent!important;transition:none!important;animation:none!important} .toasts,.toast{display:none!important} ::selection{background:#b3d4fc}" });
  return p;
}
/** Añade archivos a la lista como si se hubieran soltado en la ventana. */
const add = (p, ids) => p.evaluate(async (ids) => { await window.__state.addFiles(ids.map((id) => window.__paths[id])); }, ids);
/** Pone un trabajo en un estado dado directamente en el estado de la app. */
const setJob = (p, id, patch) => p.evaluate((id, patch) => {
  const j = window.__state.app.jobs.find((j) => j.path === window.__paths[id]);
  Object.assign(j, typeof patch === "function" ? patch(j) : patch);
}, id, patch);
const clickText = (p, sel, text) => p.evaluate((sel, text) => {
  const el = [...document.querySelectorAll(sel)].find((e) => e.textContent.trim().includes(text));
  if (!el) throw new Error(`no encuentro ${sel} «${text}»`);
  el.click();
}, sel, text);

const T = lang === "es"
  ? { recognize: "Reconocer texto", viewText: "Ver texto", pages: "Páginas", settings: "Ajustes", agents: "Asistentes de IA", select: "SEGUNDA." }
  : { recognize: "Recognize text", viewText: "View text", pages: "Pages", settings: "Settings", agents: "AI assistants", select: "SECOND." };

const doneResult = (path, pages, chars, secs) => ({
  status: "done",
  result: { skipped: false, pdfPath: path.replace(/\.pdf$/, " - OCR.pdf"), txtPath: path.replace(/\.pdf$/, " - OCR.txt"), pages, chars, elapsedSecs: secs, note: null },
});

const skipped = () => ({ status: "skipped", result: { skipped: true, pdfPath: null, txtPath: null, pages: 0, chars: 21480, elapsedSecs: 0, note: lang === "es" ? "El PDF ya tiene capa de texto; no hace falta OCR. Puedes forzarlo desde el menú del archivo." : "The PDF already has a text layer; no OCR needed. You can force it from the file's options." } });
const saved = (target) => ({ saved: { target, webUrl: "https://demo.iurefficient.com/documents", fileNames: ["a.pdf", "a.txt"], at: Date.now() } });

// ---------------------------------------------------------------- GIF principal
{
  const p = await open();
  await p.bringToFront();
  await add(p, ["hero", "poder", "dictamen"]);
  await wait(600);
  const P = await p.evaluate(() => window.__paths);
  await setJob(p, "poder", doneResult(P.poder, 5, 9412, 14.2));
  await setJob(p, "dictamen", skipped());
  await wait(300);

  const frames = [];
  const shot = async (hold) => {
    await wait(80);
    const file = `${String(frames.length).padStart(3, "0")}.png`;
    await p.screenshot({ path: OUT + "hero/" + file });
    frames.push([file, hold]);
  };
  await shot(1.2);
  // «Reconocer texto» del contrato (el primer trabajo de la lista).
  await p.evaluate((txt) => [...document.querySelector(".job").querySelectorAll("button")].find((b) => b.textContent.includes(txt)).click(), T.recognize);
  await wait(200);
  for (let i = 1; i <= 3; i++) {
    await p.evaluate((i) => window.__progress("render", i, 3), i);
    await shot(0.35);
  }
  for (let i = 1; i <= 3; i++) {
    await p.evaluate((i) => window.__progress("ocr", i, 3), i);
    await shot(0.4);
    await p.evaluate((i) => window.__progress("ocr", i, 3), i);
    await shot(0.4);
  }
  await p.evaluate(() => window.__finish(window.__heroChars, 7.8));
  await wait(300);
  await shot(1.3);
  await clickText(p, ".job button", T.viewText);
  await wait(400);
  await shot(1.0);
  // Selecciona el texto de la cláusula SEGUNDA poco a poco, como al arrastrar el ratón.
  const steps = 7;
  for (let k = 1; k <= steps; k++) {
    await p.evaluate((mark, k, steps) => {
      const pre = document.querySelector("pre.text");
      const node = pre.firstChild;
      const start = node.textContent.indexOf(mark);
      const end = node.textContent.indexOf("\n\n", start);
      const r = document.createRange();
      r.setStart(node, start);
      r.setEnd(node, start + Math.round(((end - start) * k) / steps));
      const s = getSelection();
      s.removeAllRanges();
      s.addRange(r);
    }, T.select, k, steps);
    await shot(k === steps ? 2.0 : 0.13);
  }
  fs.writeFileSync(OUT + "hero/frames.json", JSON.stringify(frames));
  // Captura «resultado»: el mismo modal con el texto reconocido y una selección.
  await p.screenshot({ path: OUT + "result.png" });
  await p.close();
}

// ---------------------------------------------------------------- Cola con varios estados
{
  const p = await open();
  await add(p, ["demanda", "recibo", "hero", "dictamen", "poder"]);
  await wait(800);
  const P = await p.evaluate(() => window.__paths);
  await setJob(p, "poder", doneResult(P.poder, 5, 9412, 14.2));
  await setJob(p, "hero", { ...doneResult(P.hero, 3, await p.evaluate(() => window.__heroChars), 7.8), ...saved(lang === "es" ? "Torres vs. Ejemplo S.A. / Contratos" : "Torres v. Example Inc. / Contracts") });
  await setJob(p, "dictamen", skipped());
  await setJob(p, "recibo", { status: "queued" });
  await setJob(p, "demanda", { status: "ocr", current: 6, total: 14 });
  await p.evaluate(() => (window.__state.app.running = true));
  await wait(400);
  await p.screenshot({ path: OUT + "queue.png" });

  // Herramientas de páginas sobre el escrito de demanda.
  await setJob(p, "demanda", { status: "ready", current: 0, total: 0 });
  await p.evaluate(() => (window.__state.app.running = false));
  await wait(200);
  await p.evaluate((txt) => [...document.querySelector(".job").querySelectorAll("button")].find((b) => b.textContent.trim() === txt).click(), T.pages);
  await wait(1200);
  for (const idx of [2, 3, 7]) {
    const el = await p.$(`.grid .page[data-idx="${idx}"]`);
    await el.click();
    await wait(80);
  }
  await p.mouse.move(5, 5);
  await wait(300);
  await p.screenshot({ path: OUT + "pages.png" });
  await p.close();
}

// ---------------------------------------------------------------- Ajustes → Asistentes de IA (tema oscuro)
{
  const p = await open("&theme=dark");
  await p.evaluate(() => (window.__state.app.view = "settings"));
  await wait(800);
  await p.evaluate((txt) => {
    const h = [...document.querySelectorAll("section h2")].find((e) => e.textContent.includes(txt));
    const sec = h.closest("section");
    const sc = sec.closest(".scroll") ?? sec.parentElement;
    sc.scrollTop += sec.getBoundingClientRect().top - sc.getBoundingClientRect().top - 12;
  }, T.agents);
  await wait(300);
  await p.screenshot({ path: OUT + "agents.png" });
  await p.close();
}

await browser.close();
console.log("capturas listas en", OUT);
