import { useEffect, useRef, useState } from "react";
import type { CustomTheme } from "../api";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/Bits";
import { applyTheme, colorVars, elementVars, themes } from "../themes";
import type { Key } from "../i18n";
import { previewPack, soundPacks, type SoundPack } from "../sounds";

/** Colors of a theme as #rrggbb (the color input needs 6-digit hex). */
function hex6(v: string | undefined): string {
  if (!v) return "#000000";
  if (/^#[0-9a-f]{6}$/i.test(v)) return v;
  if (/^#[0-9a-f]{3}$/i.test(v)) return "#" + v.slice(1).split("").map((c) => c + c).join("");
  return "#000000";
}

export function ThemeEditor({ theme, onClose }: { theme: CustomTheme | null; onClose: () => void }) {
  const { t, settings, updateSettings, toast } = useStore();
  const [draft, setDraft] = useState<CustomTheme>(
    theme ?? {
      id: `custom-${Date.now().toString(36)}`,
      name: settings.lang === "ru" ? "Моя тема" : "My theme",
      base: themes.find((x) => x.id === settings.theme)?.id ?? "official",
      vars: {},
      radius: 12,
      border: 2,
      glow: false,
    },
  );
  const saved = useRef(false);
  const base = themes.find((x) => x.id === draft.base) ?? themes[0];

  // Live preview: the whole launcher shows the draft while the editor is open
  useEffect(() => {
    applyTheme(draft.id, [draft]);
  }, [draft]);
  useEffect(
    () => () => {
      if (!saved.current) applyTheme(settings.theme, settings.customThemes);
    },
    // restore the real theme only when the editor closes
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [],
  );

  const setVar = (k: string, v: string | null) =>
    setDraft((d) => {
      const vars = { ...d.vars };
      if (v === null) delete vars[k];
      else vars[k] = v;
      return { ...d, vars };
    });

  const save = () => {
    saved.current = true;
    const others = settings.customThemes.filter((c) => c.id !== draft.id);
    updateSettings({ customThemes: [...others, draft], theme: draft.id });
    toast(t("themes.saved"), "ok");
    onClose();
  };

  return (
    <Modal title={t("themes.editor")} onClose={onClose} wide>
      <div className="field-row">
        <label className="field">
          <span>{t("themes.name")}</span>
          <input value={draft.name} maxLength={32} onChange={(e) => setDraft({ ...draft, name: e.target.value })} />
        </label>
        <label className="field">
          <span>{t("themes.base")}</span>
          <select value={draft.base} onChange={(e) => setDraft({ ...draft, base: e.target.value })}>
            {themes.map((x) => (
              <option key={x.id} value={x.id}>
                {x.name[settings.lang]}
              </option>
            ))}
          </select>
        </label>
      </div>

      <h3 className="section-title">{t("themes.colors")}</h3>
      <div className="color-grid">
        {colorVars.map((k) => (
          <label key={k} className="color-field">
            <input type="color" value={hex6(draft.vars[k] ?? base.vars[k])} onChange={(e) => setVar(k, e.target.value)} />
            <span className="small">{t(`themes.var.${k}` as Key)}</span>
            {draft.vars[k] && (
              <button className="link small" onClick={() => setVar(k, null)}>
                {t("themes.reset")}
              </button>
            )}
          </label>
        ))}
      </div>

      <h3 className="section-title">{t("themes.shape")}</h3>
      <div className="field-row">
        <label className="field">
          <span>
            {t("themes.radius")}: {draft.radius}px
          </span>
          <input type="range" min={0} max={24} value={draft.radius} onChange={(e) => setDraft({ ...draft, radius: Number(e.target.value) })} />
        </label>
        <label className="field">
          <span>
            {t("themes.border")}: {draft.border}px
          </span>
          <input type="range" min={0} max={4} value={draft.border} onChange={(e) => setDraft({ ...draft, border: Number(e.target.value) })} />
        </label>
        <label className="check-inline">
          {t("themes.glow")}
          <Toggle on={draft.glow} onChange={(v) => setDraft({ ...draft, glow: v })} label={t("themes.glow")} />
        </label>
      </div>

      <div className="field-row">
        <label className="field">
          <span>{t("snd.pack")}</span>
          <select
            value={draft.soundPack ?? ""}
            onChange={(e) => {
              setDraft({ ...draft, soundPack: e.target.value || undefined });
              if (e.target.value) previewPack(e.target.value as SoundPack);
            }}
          >
            <option value="">{t("snd.pack.theme")}</option>
            {soundPacks.map((p) => (
              <option key={p} value={p}>
                {t(`snd.pack.${p}` as Key)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <h3 className="section-title">{t("themes.elements")}</h3>
      <div className="color-grid">
        {elementVars.map((k) => (
          <label key={k} className="color-field">
            <input
              type="color"
              value={hex6(draft.vars[k] ?? (k.includes("text") ? base.vars["--on-accent"] : k.includes("border") ? base.vars["--line"] : k.includes("card") ? base.vars["--panel"] : k.includes("title") ? base.vars["--text"] : base.vars["--accent"]))}
              onChange={(e) => setVar(k, e.target.value)}
            />
            <span className="small">{t(`themes.el.${k}` as Key)}</span>
            {draft.vars[k] ? (
              <button className="link small" onClick={() => setVar(k, null)}>
                {t("themes.reset")}
              </button>
            ) : (
              <span className="muted small">{t("themes.reset")}</span>
            )}
          </label>
        ))}
      </div>

      <div className="theme-preview">
        <span className="play theme-preview__play">{t("home.play")}</span>
        <span className="card theme-preview__card">
          <strong>{t("home.myBuilds")}</strong>
          <span className="muted small">Fabric · 1.21.11</span>
        </span>
        <span className="nav nav--active">{t("nav.home")}</span>
      </div>

      <div className="modal__foot">
        <button className="btn" onClick={onClose}>
          {t("create.cancel")}
        </button>
        <button className="btn btn--primary" onClick={save}>
          {t("create.save")}
        </button>
      </div>
    </Modal>
  );
}
