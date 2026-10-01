// Genera las páginas «escaneadas» de los documentos ficticios (docs.mjs) y pasa el
// contrato por Tesseract para tener el texto reconocido de verdad.
// Salida: scans/<lang>/*.jpg y scans/<lang>/hero.txt
import puppeteer from "puppeteer-core";
import fs from "node:fs";
import { execFileSync } from "node:child_process";
import { HERO, FILLER, OTHERS } from "./docs.mjs";

const HERE = new URL(".", import.meta.url).pathname;
const CHROME = process.env.CHROME ?? process.env.HOME + "/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome";

const CSS = `body{margin:0;width:1240px;height:1754px;background:#fff;font-family:"DejaVu Serif","Liberation Serif",serif;color:#111}
.p{padding:130px 140px;font-size:25px;line-height:1.62;text-align:justify}
h1{font-size:32px;text-align:center;letter-spacing:.06em;margin:0 0 8px}
.sub{text-align:center;font-size:22px;margin-bottom:46px}
h2{font-size:24px;letter-spacing:.04em;margin:34px 0 10px}
p{margin:0 0 18px}
.sig{display:flex;justify-content:space-between;margin-top:150px;font-size:22px;text-align:center}
.sig div{width:400px;border-top:2px solid #111;padding-top:10px}
.foot{position:absolute;bottom:80px;left:0;right:0;text-align:center;font-size:18px;color:#444}`;

const esc = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;");
function block(line) {
  if (line.startsWith("#")) return `<h2>${esc(line.slice(1))}</h2>`;
  if (line.startsWith("@")) {
    const [a, b, c, d] = line.slice(1).split("|").map(esc);
    return `<div class="sig"><div>${a}<br>${b}</div><div>${c}<br>${d}</div></div>`;
  }
  return `<p>${esc(line).replace(/\*\*(.+?)\*\*/g, "<b>$1</b>")}</p>`;
}
function pageHtml({ title, sub, lines, num }) {
  return `<!doctype html><html><head><meta charset="utf-8"><style>${CSS}</style></head><body><div class="p">
${title ? `<h1>${esc(title)}</h1>` : ""}${sub ? `<div class="sub">${esc(sub)}</div>` : ""}
${lines.map(block).join("\n")}</div>${num ? `<div class="foot">${num}</div>` : ""}</body></html>`;
}

const browser = await puppeteer.launch({ executablePath: CHROME, args: ["--no-sandbox"] });
const page = await browser.newPage();
await page.setViewport({ width: 1240, height: 1754 });

let seed = 1;
async function render(html, out) {
  await page.setContent(html);
  await page.evaluate(() => document.fonts.ready);
  const png = out.replace(/\.jpg$/, ".png");
  await page.screenshot({ path: png });
  // Aspecto de escaneo: papel ligeramente cálido, ruido, un poco de inclinación y desenfoque.
  const angle = (((seed++ * 37) % 9) - 4) / 10;
  execFileSync("convert", [png, "-background", "white", "-rotate", String(angle), "-gravity", "center", "-extent", "1240x1754",
    "-attenuate", "0.35", "+noise", "Gaussian", "-blur", "0x0.4",
    "-fill", "#f4efe4", "-colorize", "7%", "-quality", "78", out]);
  fs.rmSync(png);
}

for (const lang of ["es", "en"]) {
  const dir = `${HERE}scans/${lang}/`;
  fs.mkdirSync(dir, { recursive: true });
  const h = HERO[lang];
  for (let i = 0; i < h.pages.length; i++) {
    await render(pageHtml({ title: i === 0 ? h.title : "", sub: i === 0 ? h.sub : "", lines: h.pages[i], num: `${i + 1}/${h.pages.length}` }), `${dir}hero-${i + 1}.jpg`);
  }
  for (const d of OTHERS[lang]) {
    await render(pageHtml({ title: d.title, sub: "", lines: FILLER[lang].slice(0, 2) }), `${dir}${d.id}-1.jpg`);
  }
  for (let k = 0; k < 3; k++) {
    const lines = [...FILLER[lang].slice(k), ...FILLER[lang].slice(0, k), FILLER[lang][k]];
    await render(pageHtml({ title: "", sub: "", lines }), `${dir}filler-${k + 1}.jpg`);
  }
  // OCR real del contrato, como lo hace la app: una lista de páginas y el renderer txt.
  fs.writeFileSync(`${dir}list.txt`, h.pages.map((_, i) => `${dir}hero-${i + 1}.jpg`).join("\n") + "\n");
  execFileSync("tesseract", [`${dir}list.txt`, `${dir}hero`, "-l", lang === "es" ? "spa+eng" : "eng+spa", "txt"], { stdio: "ignore" });
  fs.rmSync(`${dir}list.txt`);
}
await browser.close();
console.log("scans listos");
