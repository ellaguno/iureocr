import { ask, message } from "@tauri-apps/plugin-dialog";
import { api, type UpdateNotice } from "./api";
import { app, toast } from "./state.svelte";

/**
 * Actualizaciones en dos capas: el actualizador de Tauri (instala dentro de la
 * app; en Linux sólo el AppImage) y, si no puede, un aviso con enlace a la release.
 */
export async function checkForUpdates(silent: boolean): Promise<void> {
  const target = app.sys?.updateTarget;
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check(target ? { target } : undefined);
    if (update) {
      app.updateNotice = { version: update.version, url: "https://github.com/ellaguno/iureocr/releases/latest" };
      const install = await ask(`Hay una nueva versión de IureOCR (${update.version}).\n¿Descargar e instalar ahora?`, {
        title: "IureOCR — Actualización disponible",
        kind: "info",
        okLabel: "Actualizar",
        cancelLabel: "Ahora no",
      });
      if (!install) return;
      toast("Descargando la actualización…", "info", 6000);
      await update.downloadAndInstall();
      const restart = await ask("Actualización instalada. ¿Reiniciar IureOCR ahora?", { title: "IureOCR", kind: "info", okLabel: "Reiniciar", cancelLabel: "Después" });
      if (restart) {
        const { relaunch } = await import("@tauri-apps/plugin-process");
        await relaunch();
      }
      return;
    }
  } catch (err) {
    console.warn("Actualizador de Tauri no disponible:", err);
  }
  let notice: UpdateNotice | null = null;
  try {
    notice = await api.checkUpdateNotice();
  } catch (err) {
    if (!silent) await message(`No se pudo buscar actualizaciones.\n${err}`, { title: "IureOCR", kind: "warning" });
    return;
  }
  app.updateNotice = notice;
  if (!notice && !silent) await message("Ya tienes la última versión.", { title: "IureOCR", kind: "info" });
}
