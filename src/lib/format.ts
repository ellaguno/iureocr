export function fmtBytes(bytes: number | null | undefined): string {
  if (bytes == null || !isFinite(bytes)) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v < 10 ? v.toFixed(1) : Math.round(v)} ${units[i]}`;
}

export function fmtSecs(secs: number | null | undefined): string {
  if (secs == null || !isFinite(secs)) return "—";
  if (secs < 60) return `${secs.toFixed(secs < 10 ? 1 : 0)} s`;
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${m} min ${String(s).padStart(2, "0")} s`;
}

export function baseName(p: string): string {
  return p.split(/[\\/]/).pop() ?? p;
}
