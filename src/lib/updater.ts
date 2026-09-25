import { ask, message } from "@tauri-apps/plugin-dialog";
import { api, type UpdateNotice } from "./api";
import { app, toast } from "./state.svelte";
import { t } from "./i18n.svelte";

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
      const install = await ask(t("update.available", { version: update.version }), {
        title: t("update.availableTitle"),
        kind: "info",
        okLabel: t("update.install"),
        cancelLabel: t("update.notNow"),
      });
      if (!install) return;
      toast(t("update.downloading"), "info", 6000);
      await update.downloadAndInstall();
      const restart = await ask(t("update.installed"), { title: "IureOCR", kind: "info", okLabel: t("update.restart"), cancelLabel: t("update.later") });
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
    if (!silent) await message(t("update.checkFailed", { error: String(err) }), { title: "IureOCR", kind: "warning" });
    return;
  }
  app.updateNotice = notice;
  if (!notice && !silent) await message(t("update.upToDate"), { title: "IureOCR", kind: "info" });
}
