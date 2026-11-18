import { useMemo, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errText, type ContentKind, type Instance, type InstalledContent } from "../api";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

type Format = "text" | "markdown" | "html" | "csv" | "json";
const EXT: Record<Format, string> = { text: "txt", markdown: "md", html: "html", csv: "csv", json: "json" };

const link = (c: InstalledContent) => (c.projectId && !c.projectId.startsWith("own:") ? `https://modrinth.com/project/${c.projectId}` : "");
const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const csvCell = (s: string) => (/[",\n;]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);

/** The build's mods (or packs) as a list to send to a friend or move to another launcher, like Prism's export. */
export function ModListExport({ inst, kind, items, onClose }: { inst: Instance; kind: ContentKind; items: InstalledContent[]; onClose: () => void }) {
  const { settings, toast } = useStore();
  const ru = settings.lang !== "en";
  const L = (r: string, e: string) => (ru ? r : e);
  const [format, setFormat] = useState<Format>("text");
  const [onlyEnabled, setOnlyEnabled] = useState(true);
  const [versions, setVersions] = useState(true);
  const [links, setLinks] = useState(true);

  const list = useMemo(
    () => items.filter((c) => !onlyEnabled || c.enabled).sort((a, b) => a.title.localeCompare(b.title)),
    [items, onlyEnabled],
  );
  const ruForms: Record<string, [string, string, string]> = {
    mod: ["мод", "мода", "модов"],
    resourcepack: ["ресурспак", "ресурспака", "ресурспаков"],
    shader: ["шейдер", "шейдера", "шейдеров"],
    datapack: ["датапак", "датапака", "датапаков"],
  };
  const enWords: Record<string, string> = { mod: "mods", resourcepack: "resource packs", shader: "shaders", datapack: "data packs" };
  const what = ru ? (ruForms[kind]?.[2] ?? "") : (enWords[kind] ?? "");

  const text = useMemo(() => {
    const ver = (c: InstalledContent) => (versions && c.versionNumber ? ` ${c.versionNumber}` : "");
    switch (format) {
      case "text":
        return list.map((c) => `${c.title}${ver(c)}${links && link(c) ? ` — ${link(c)}` : ""}`).join("\n");
      case "markdown":
        return list.map((c) => `- ${links && link(c) ? `[${c.title}](${link(c)})` : c.title}${ver(c)}`).join("\n");
      case "html":
        return [
          "<!DOCTYPE html>",
          `<html><head><meta charset="utf-8"><title>${esc(inst.name)}</title></head><body>`,
          "<ul>",
          ...list.map((c) => `<li>${links && link(c) ? `<a href="${link(c)}">${esc(c.title)}</a>` : esc(c.title)}${esc(ver(c))}</li>`),
          "</ul>",
          "</body></html>",
        ].join("\n");
      case "csv":
        return [
          ["name", ...(versions ? ["version"] : []), ...(links ? ["link"] : []), "file"].join(","),
          ...list.map((c) => [c.title, ...(versions ? [c.versionNumber ?? ""] : []), ...(links ? [link(c)] : []), c.filename].map(csvCell).join(",")),
        ].join("\n");
      case "json":
        return JSON.stringify(
          list.map((c) => ({
            name: c.title,
            ...(versions ? { version: c.versionNumber ?? null } : {}),
            ...(links ? { link: link(c) || null } : {}),
            file: c.filename,
          })),
          null,
          2,
        );
    }
  }, [format, list, versions, links, inst]);

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      sfx.success();
      toast(L("Список скопирован", "List copied"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  const saveFile = async () => {
    try {
      const safe = inst.name.replace(/[\\/:*?"<>|]/g, "_");
      const dest = await save({ defaultPath: `${safe} — ${what}.${EXT[format]}`, filters: [{ name: format.toUpperCase(), extensions: [EXT[format]] }] });
      if (!dest) return;
      await api.writeTextFile(dest, text);
      sfx.success();
      toast(L("Список сохранён", "List saved"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const formats: [Format, string][] = [
    ["text", L("Текст", "Text")],
    ["markdown", "Markdown"],
    ["html", "HTML"],
    ["csv", "CSV"],
    ["json", "JSON"],
  ];

  return (
    <Modal title={L("Список для друга или другого лаунчера", "List for a friend or another launcher")} onClose={onClose} wide>
      <div className="modlist">
        <div className="seg" role="tablist">
          {formats.map(([f, label]) => (
            <button key={f} className={`seg__btn ${format === f ? "seg__btn--on" : ""}`} onClick={() => setFormat(f)}>
              {label}
            </button>
          ))}
        </div>
        <div className="modlist__opts">
          <label className="check-inline">
            {L("Только включённые", "Enabled only")}
            <Toggle on={onlyEnabled} onChange={setOnlyEnabled} label="enabled" />
          </label>
          <label className="check-inline">
            {L("Версии", "Versions")}
            <Toggle on={versions} onChange={setVersions} label="versions" />
          </label>
          <label className="check-inline">
            {L("Ссылки на Modrinth", "Modrinth links")}
            <Toggle on={links} onChange={setLinks} label="links" />
          </label>
        </div>
        <textarea className="modlist__text" readOnly value={text} spellCheck={false} />
        <p className="muted small">
          {L(
            "Друг может вставить этот список в «Как у друга» — лаунчер сам найдёт и поставит всё, что есть на Modrinth.",
            "A friend can paste this list into “Like a friend” — the launcher finds and installs everything that is on Modrinth.",
          )}
        </p>
        <div className="modlist__actions">
          <button className="btn btn--primary" onClick={copy}>
            <Icon name="copy" size={16} /> {L("Скопировать", "Copy")}
          </button>
          <button className="btn" onClick={saveFile}>
            <Icon name="download" size={16} /> {L("Сохранить файлом", "Save as file")}
          </button>
        </div>
      </div>
    </Modal>
  );
}
