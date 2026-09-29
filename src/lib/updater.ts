import { ask, message } from "@tauri-apps/plugin-dialog";
import { api, type UpdateNotice } from "./api";
import { app, toast } from "./state.svelte";
import { t } from "./i18n.svelte";

const RELEASES_URL = "https://github.com/ellaguno/iureocr/releases/latest";

/**
 * Actualizaciones en dos capas: el actualizador de Tauri (instala dentro de la
 * app) y, si no puede, un aviso con enlace a la release.
 *
 * Sin `target` explícito: el plugin busca primero `{os}-{arch}-{instalador}`
 * (p. ej. `linux-x86_64-deb`) y luego `{os}-{arch}`, así una instalación .deb
 * recibe el .deb y no el AppImage.
 */
export async function checkForUpdates(silent: boolean): Promise<void> {
  try {
    const { check } = await import("@tauri-apps/plugin-updater");
    const update = await check();
    if (update) {
      app.updateNotice = { version: update.version, url: RELEASES_URL };
      const install = await ask(t("update.available", { version: update.version }), {
        title: t("update.availableTitle"),
        kind: "info",
        okLabel: t("update.install"),
        cancelLabel: t("update.notNow"),
      });
      if (!install) return;
      toast(t("update.downloading"), "info", 6000);
      try {
        await update.downloadAndInstall();
      } catch (err) {
        // Antes este error caía en el catch externo y se perdía en silencio.
        console.error("No se pudo instalar la actualización:", err);
        const open = await ask(t("update.installFailed", { error: String(err) }), {
          title: "IureOCR",
          kind: "warning",
          okLabel: t("update.openDownload"),
          cancelLabel: t("update.later"),
        });
        if (open) {
          const { openUrl } = await import("@tauri-apps/plugin-opener");
          await openUrl(RELEASES_URL);
        }
        return;
      }
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
