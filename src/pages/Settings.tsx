import { useEffect, useRef, useState } from "react";
import { confirm } from "../components/Confirm";
import { open, save } from "@tauri-apps/plugin-dialog";
import { defaultEvents, defaultSettings, defaultUi, type CustomTheme, type EventPrefs, type HomeBlock, type NavPage, type Settings, type UiPrefs } from "../api";
import { ThemeEditor } from "./ThemeEditor";
import { Icon } from "../components/Icon";
import { previewPack, sfx, soundPacks } from "../sounds";
import type { Key } from "../i18n";
import { BaseModsSettings, GameDefaultsSettings } from "./Extras";
import { BackupSettings, DiscordSettings, UpdateSettings } from "./Care";
import { api, errText } from "../api";
import { useStore } from "../store";
import { Toggle } from "../components/Bits";
import { themes } from "../themes";

type Tab = "look" | "home" | "ui" | "events" | "sound" | "game" | "data";

export function SettingsPage() {
  const { t, settings, updateSettings, toast, setAccounts, previewSeason } = useStore();
  const [clientId, setClientId] = useState(settings.clientId ?? "");
  const [editing, setEditing] = useState<CustomTheme | null | "new">(null);
  const [tab, setTab] = useState<Tab>("look");
  const [query, setQuery] = useState("");
  const [totalMb, setTotalMb] = useState(0);
  useEffect(() => {
    api.systemMemory().then(setTotalMb).catch(() => setTotalMb(0));
  }, []);

  const importTheme = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "AniLauncher theme", extensions: ["anitheme", "json"] }] });
      if (typeof file !== "string") return;
      const raw = JSON.parse(await api.readTextFile(file)) as Partial<CustomTheme>;
      if (!raw || typeof raw !== "object" || typeof raw.name !== "string" || typeof raw.vars !== "object") throw new Error(t("themes.badFile"));
      const vars: Record<string, string> = {};
      for (const [k, v] of Object.entries(raw.vars ?? {})) if (k.startsWith("--") && typeof v === "string" && /^#[0-9a-f]{3,8}$/i.test(v)) vars[k] = v;
      const theme: CustomTheme = {
        id: `custom-${Date.now().toString(36)}`,
        name: raw.name.slice(0, 32),
        base: themes.some((x) => x.id === raw.base) ? (raw.base as string) : "official",
        vars,
        radius: Math.min(24, Math.max(0, Number(raw.radius) || 12)),
        border: Math.min(4, Math.max(0, Number(raw.border) || 2)),
        glow: !!raw.glow,
      };
      updateSettings({ customThemes: [...settings.customThemes, theme], theme: theme.id });
      toast(t("themes.imported"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const exportTheme = async (th: CustomTheme) => {
    try {
      const dest = await save({ defaultPath: `${th.name}.anitheme`, filters: [{ name: "AniLauncher theme", extensions: ["anitheme"] }] });
      if (!dest) return;
      const { id: _id, ...rest } = th;
      void _id;
      await api.writeTextFile(dest, JSON.stringify(rest, null, 2));
      toast(t("set.saved"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const pickFont = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "Fonts", extensions: ["ttf", "otf", "woff", "woff2"] }] });
      if (typeof file !== "string") return;
      updateSettings({ fontPath: await api.importMedia(file) });
      sfx.success();
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const ui = settings.ui;
  const ev = settings.events;
  const setUi = (patch: Partial<UiPrefs>) => updateSettings({ ui: { ...ui, ...patch } });
  const setEv = (patch: Partial<EventPrefs>) => updateSettings({ events: { ...ev, ...patch } });

  const tabs: Array<{ id: Tab; icon: string; label: string }> = [
    { id: "look", icon: "palette", label: L("Внешний вид", "Appearance") },
    { id: "home", icon: "home", label: L("Главная", "Home") },
    { id: "ui", icon: "grid", label: L("Интерфейс", "Interface") },
    { id: "events", icon: "sparkle", label: L("События", "Events") },
    { id: "sound", icon: "play", label: L("Звуки", "Sounds") },
    { id: "game", icon: "rocket", label: L("Игра", "Game") },
    { id: "data", icon: "user", label: L("Аккаунт и данные", "Account & data") },
  ];

  // Search hides rows that don't match, across every tab
  const paneRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const root = paneRef.current;
    if (!root) return;
    const q = query.trim().toLowerCase();
    root.querySelectorAll<HTMLElement>(".setting, .field, .setting-block").forEach((el) => {
      el.style.display = !q || (el.textContent ?? "").toLowerCase().includes(q) ? "" : "none";
    });
    root.querySelectorAll<HTMLElement>(".panel").forEach((p) => {
      if (!q) return void (p.style.display = "");
      const rows = p.querySelectorAll<HTMLElement>(".setting, .field, .setting-block");
      const title = (p.querySelector("h2")?.textContent ?? "").toLowerCase();
      if (title.includes(q)) {
        rows.forEach((r) => (r.style.display = ""));
        p.style.display = "";
      } else p.style.display = rows.length && Array.from(rows).some((r) => r.style.display !== "none") ? "" : "none";
    });
  }, [query, tab, settings]);

  const blockName: Record<HomeBlock, string> = {
    quick: L("Быстрый старт", "Quick start"),
    builds: L("Мои сборки", "My builds"),
    widgets: L("Виджеты", "Widgets"),
    news: L("Новости", "News"),
  };
  const navName: Record<NavPage, string> = {
    home: t("nav.home"), builds: t("nav.builds"), mods: t("nav.mods"), skins: t("nav.skins"), shots: t("nav.shots"), news: t("nav.news"),
  };
  const moveBlock = (b: HomeBlock, dir: -1 | 1) => {
    const order = [...ui.homeOrder];
    const i = order.indexOf(b);
    const j = i + dir;
    if (j < 0 || j >= order.length) return;
    [order[i], order[j]] = [order[j], order[i]];
    sfx.click();
    setUi({ homeOrder: order });
  };

  const exportSettings = async () => {
    try {
      const dest = await save({ defaultPath: "AniLauncher-settings.json", filters: [{ name: "JSON", extensions: ["json"] }] });
      if (!dest) return;
      const { dataDir: _d, builtInClientId: _b, selectedInstance: _s, clientId: _c, ...rest } = settings;
      void _d; void _b; void _s; void _c;
      await api.writeTextFile(dest, JSON.stringify(rest, null, 2));
      toast(L("Настройки сохранены в файл", "Settings exported"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  const importSettings = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
      if (typeof file !== "string") return;
      const raw = JSON.parse(await api.readTextFile(file)) as Partial<Settings>;
      if (!raw || typeof raw !== "object" || Array.isArray(raw)) throw new Error(L("Это не файл настроек AniLauncher", "Not an AniLauncher settings file"));
      const patch: Partial<Settings> = {};
      const keep: Array<keyof Settings> = ["lang", "theme", "sounds", "animations", "memoryMb", "onLaunch", "soundPack", "volume", "ambient", "checkConflicts", "autoUpdateMods", "streamerNote", "globalJvmArgs", "birthday"];
      for (const k of keep) if (k in raw && typeof raw[k] === typeof defaultSettings[k]) (patch as Record<string, unknown>)[k] = raw[k];
      if (raw.ui && typeof raw.ui === "object") patch.ui = { ...defaultUi, ...raw.ui };
      if (raw.events && typeof raw.events === "object") patch.events = { ...defaultEvents, ...raw.events };
      if (raw.widgets && typeof raw.widgets === "object") patch.widgets = { ...settings.widgets, ...raw.widgets };
      if (raw.background && typeof raw.background === "object") patch.background = { ...settings.background, ...raw.background };
      if (Array.isArray(raw.customThemes)) patch.customThemes = [...settings.customThemes, ...raw.customThemes.filter((c) => c && typeof c.id === "string" && !settings.customThemes.some((x) => x.id === c.id))];
      if (raw.baseMods && typeof raw.baseMods === "object") patch.baseMods = raw.baseMods;
      updateSettings(patch);
      toast(L("Настройки загружены", "Settings imported"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };
  const resetLook = async () => {
    if (!(await confirm(L("Вернуть оформление, главную, интерфейс, события и звуки к стандартным? Профиль, сборки, аккаунты и свои темы останутся.", "Reset appearance, home, interface, events and sounds to defaults? Profile, builds, accounts and your themes stay."), { title: "AniLauncher", kind: "warning" }))) return;
    updateSettings({
      theme: defaultSettings.theme, ui: defaultUi, events: defaultEvents, background: defaultSettings.background, widgets: defaultSettings.widgets,
      sounds: true, soundPack: "theme", volume: defaultSettings.volume, ambient: true, animations: true, fontPath: null,
    });
    toast(L("Готово — всё как с завода", "Done — back to defaults"), "ok");
  };

  const ACCENTS = ["#3c8527", "#16a34a", "#0ea5e9", "#6366f1", "#a855f7", "#ec4899", "#ef4444", "#f97316", "#eab308"];
  const show = (id: Tab) => !!query.trim() || tab === id;

  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("set.title")}</h1>
      </div>
      <div className="settings-layout">
        <div className="settings-tabs" role="tablist" aria-orientation="vertical">
          <input className="settings-search" value={query} placeholder={L("Поиск настройки…", "Search settings…")} onChange={(e) => setQuery(e.target.value)} aria-label={L("Поиск", "Search")} />
          {tabs.map((x) => (
            <button key={x.id} role="tab" aria-selected={!query.trim() && tab === x.id} onClick={() => { sfx.click(); setQuery(""); setTab(x.id); }}>
              <Icon name={x.icon} size={17} /> {x.label}
            </button>
          ))}
        </div>

        <div className="settings-pane" ref={paneRef} key={query.trim() ? "search" : tab}>
          {show("look") && (
            <>
              <section className="panel">
                <h2>{t("set.theme")}</h2>
                <div className="theme-grid">
                  {themes.map((th) => (
                    <button
                      key={th.id}
                      className={`theme-card ${settings.theme === th.id ? "theme-card--on" : ""}`}
                      onMouseEnter={sfx.hover}
                      onClick={() => {
                        sfx.click();
                        updateSettings({ theme: th.id });
                      }}
                      style={{ background: th.vars["--bg"], color: th.vars["--text"], borderColor: settings.theme === th.id ? th.vars["--accent-hi"] : th.vars["--line"] }}
                    >
                      <span className="theme-card__preview">
                        <span style={{ background: th.vars["--side"] }} />
                        <span style={{ background: th.vars["--panel"] }}>
                          <i style={{ background: th.vars["--accent"] }} />
                        </span>
                      </span>
                      <span className="theme-card__name">{th.name[settings.lang]}</span>
                    </button>
                  ))}
                </div>
              </section>
              <section className="panel">
                <div className="section-head section-head--tight">
                  <h2>{t("themes.custom")}</h2>
                  <div className="inline-form">
                    <button className="btn" onClick={importTheme}>
                      <Icon name="download" size={16} /> {t("themes.import")}
                    </button>
                    <button className="btn btn--primary" onClick={() => setEditing("new")}>
                      <Icon name="palette" size={16} /> {t("themes.create")}
                    </button>
                  </div>
                </div>
                <div className="theme-grid">
                  {settings.customThemes.map((th) => {
                    const b = themes.find((x) => x.id === th.base) ?? themes[0];
                    const v = (k: string) => th.vars[k] ?? b.vars[k];
                    return (
                      <div key={th.id} className={`theme-card theme-card--custom ${settings.theme === th.id ? "theme-card--on" : ""}`} style={{ background: v("--bg"), color: v("--text"), borderColor: settings.theme === th.id ? v("--accent-hi") : v("--line") }}>
                        <button className="theme-card__pick" onClick={() => updateSettings({ theme: th.id })} aria-label={th.name}>
                          <span className="theme-card__preview" style={{ borderRadius: th.radius * 0.6 }}>
                            <span style={{ background: v("--side") }} />
                            <span style={{ background: v("--panel") }}>
                              <i style={{ background: th.vars["--play-bg"] ?? v("--accent") }} />
                            </span>
                          </span>
                          <span className="theme-card__name">{th.name}</span>
                        </button>
                        <span className="theme-card__actions">
                          <button className="link small" onClick={() => setEditing(th)}>{t("themes.edit")}</button>
                          <button className="link small" onClick={() => void exportTheme(th)}>{t("themes.export")}</button>
                          <button
                            className="link small"
                            onClick={() =>
                              updateSettings({
                                customThemes: settings.customThemes.filter((x) => x.id !== th.id),
                                theme: settings.theme === th.id ? th.base : settings.theme,
                              })
                            }
                          >
                            {t("themes.delete")}
                          </button>
                        </span>
                      </div>
                    );
                  })}
                </div>
              </section>
              <section className="panel settings-list">
                <h2>{L("Тонкая настройка", "Fine-tuning")}</h2>
                <div className="setting">
                  <span>{L("Акцентный цвет (поверх любой темы)", "Accent colour (over any theme)")}</span>
                  <div className="swatches">
                    <button className={`swatch swatch--none ${!ui.accent ? "swatch--on" : ""}`} title={L("Как в теме", "From theme")} aria-label={L("Как в теме", "From theme")} onClick={() => setUi({ accent: null })} />
                    {ACCENTS.map((c) => (
                      <button key={c} className={`swatch ${ui.accent === c ? "swatch--on" : ""}`} style={{ background: c }} aria-label={c} onClick={() => setUi({ accent: c })} />
                    ))}
                    <input type="color" value={ui.accent ?? "#3c8527"} onChange={(e) => setUi({ accent: e.target.value })} aria-label={L("Свой цвет", "Custom colour")} />
                  </div>
                </div>
                <label className="setting">
                  <span>{L("Скругление углов", "Corner rounding")}: <strong>{ui.radius === null ? L("как в теме", "theme") : `${ui.radius}px`}</strong></span>
                  <div className="inline-form">
                    <input type="range" min={0} max={24} step={1} value={ui.radius ?? 12} onChange={(e) => setUi({ radius: Number(e.target.value) })} />
                    {ui.radius !== null && <button className="btn btn--small" onClick={() => setUi({ radius: null })}>{L("Сброс", "Reset")}</button>}
                  </div>
                </label>
                <label className="setting">
                  <span>{L("Масштаб интерфейса", "Interface scale")}: <strong>{Math.round(ui.scale * 100)}%</strong></span>
                  <input type="range" min={0.8} max={1.3} step={0.05} value={ui.scale} onChange={(e) => setUi({ scale: Number(e.target.value) })} />
                </label>
                <div className="setting">
                  <span>{L("Шрифт текста", "Text font")}</span>
                  <div className="seg">
                    {(["inter", "pixel", "system"] as const).map((f) => (
                      <button key={f} className={`seg__btn ${ui.bodyFont === f ? "seg__btn--on" : ""}`} onClick={() => setUi({ bodyFont: f })}>
                        {f === "inter" ? "Inter" : f === "pixel" ? L("Пиксельный", "Pixel") : L("Системный", "System")}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="setting">
                  <span>{L("Шрифт заголовков", "Heading font")}</span>
                  <div className="seg">
                    {(["rubik", "minecraft", "pixel"] as const).map((f) => (
                      <button
                        key={f}
                        className={`seg__btn ${(ui.headingFont ?? "rubik") === f ? "seg__btn--on" : ""}`}
                        style={{ fontFamily: f === "rubik" ? "Rubik" : f === "minecraft" ? '"Minecraft Ten"' : '"Pixelify Sans"' }}
                        onClick={() => setUi({ headingFont: f })}
                      >
                        {f === "rubik" ? "Rubik" : f === "minecraft" ? "Minecraft" : L("Пиксельный", "Pixel")}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="setting">
                  <span>{L("Стеклянные панели (размытие фона)", "Glass panels (background blur)")}</span>
                  <Toggle on={ui.glass} onChange={(v) => setUi({ glass: v })} label="glass" />
                </div>
                <div className="setting">
                  <span>{t("set.animations")}</span>
                  <Toggle on={settings.animations} onChange={(v) => updateSettings({ animations: v })} label={t("set.animations")} />
                </div>
                <div className="setting setting--stack">
                  <span>{t("set.font")}</span>
                  <p className="muted small">{t("set.fontHint")}</p>
                  <div className="inline-form">
                    <button className="btn" onClick={pickFont}>{t("set.fontPick")}</button>
                    {settings.fontPath && <button className="btn" onClick={() => updateSettings({ fontPath: null })}>{t("set.fontReset")}</button>}
                  </div>
                </div>
              </section>
            </>
          )}

          {show("home") && (
            <>
              <section className="panel">
                <h2>{t("bg.title")}</h2>
                <div className="seg-row">
                  {(["art", "custom", "live", "shots"] as const).map((m) => (
                    <button
                      key={m}
                      className={`chip ${settings.background.mode === m ? "chip--on" : ""}`}
                      onClick={() => {
                        sfx.click();
                        updateSettings({ background: { ...settings.background, mode: m } });
                      }}
                    >
                      {t(`bg.mode.${m}` as Key)}
                    </button>
                  ))}
                </div>
                {settings.background.mode === "live" && (
                  <>
                    <p className="muted small">{t("bg.liveHint")}</p>
                    <div className="setting">
                      <span>{t("bg.weather")}</span>
                      <select
                        value={settings.background.weather}
                        onChange={(e) => updateSettings({ background: { ...settings.background, weather: e.target.value as typeof settings.background.weather } })}
                        aria-label={t("bg.weather")}
                      >
                        {(["auto", "clear", "rain", "snow", "fireflies"] as const).map((w) => (
                          <option key={w} value={w}>
                            {t(`bg.weather.${w}` as Key)}
                          </option>
                        ))}
                      </select>
                    </div>
                    <div className="setting">
                      <span>{t("bg.ambient")}</span>
                      <Toggle on={settings.ambient} onChange={(v) => updateSettings({ ambient: v })} label={t("bg.ambient")} />
                    </div>
                  </>
                )}
                {settings.background.mode === "custom" && (
                  <div className="inline-form">
                    <button
                      className="btn"
                      onClick={async () => {
                        try {
                          const f = await open({ multiple: false, filters: [{ name: "Image / GIF / Video", extensions: ["png", "jpg", "jpeg", "webp", "gif", "mp4", "webm"] }] });
                          if (typeof f !== "string") return;
                          updateSettings({ background: { ...settings.background, mode: "custom", file: await api.importMedia(f) } });
                        } catch (e) {
                          toast(errText(e), "error");
                        }
                      }}
                    >
                      <Icon name="image" size={16} /> {t("bg.pickFile")}
                    </button>
                    {settings.background.file && <span className="muted small ellipsis">{settings.background.file.split(/[\\/]/).pop()}</span>}
                  </div>
                )}
                <span className="small muted">{t("w.widgets")}</span>
                <div className="seg-row">
                  {(["quick", "week", "shots", "updates", "calendar"] as const).map((w) => (
                    <button
                      key={w}
                      className={`chip ${settings.widgets[w] !== false ? "chip--on" : ""}`}
                      onClick={() => updateSettings({ widgets: { ...settings.widgets, [w]: settings.widgets[w] === false } })}
                    >
                      {t(({ quick: "quick.title", week: "w.week", shots: "w.shots", updates: "w.updates", calendar: "w.calendar" } as const)[w])}
                    </button>
                  ))}
                </div>
              </section>
              <section className="panel settings-list">
                <h2>{L("Шапка главной", "Home header")}</h2>
                <div className="setting">
                  <span>{L("Высота картинки", "Picture height")}</span>
                  <div className="seg">
                    {(["compact", "normal", "tall"] as const).map((h) => (
                      <button key={h} className={`seg__btn ${ui.heroHeight === h ? "seg__btn--on" : ""}`} onClick={() => setUi({ heroHeight: h })}>
                        {h === "compact" ? L("Низкая", "Low") : h === "normal" ? L("Обычная", "Normal") : L("Высокая", "Tall")}
                      </button>
                    ))}
                  </div>
                </div>
                <label className="setting">
                  <span>{L("Затемнение снизу", "Bottom shade")}: <strong>{Math.round(ui.heroShade * 100)}%</strong></span>
                  <input type="range" min={0} max={1} step={0.05} value={ui.heroShade} onChange={(e) => setUi({ heroShade: Number(e.target.value) })} />
                </label>
                <label className="setting">
                  <span>{L("Надпись на кнопке «Играть»", "Play button text")}</span>
                  <div className="inline-form">
                    <input value={ui.playLabel} maxLength={16} placeholder={t("home.play")} onChange={(e) => setUi({ playLabel: e.target.value })} />
                    <span className="preview-play">{ui.playLabel.trim() || t("home.play")}</span>
                  </div>
                </label>
              </section>
              <section className="panel settings-list">
                <h2>{L("Блоки на главной", "Home blocks")}</h2>
                <p className="muted small">{L("Порядок сверху вниз. Выключенные блоки скрыты.", "Top to bottom. Switched-off blocks are hidden.")}</p>
                <div className="order-list setting-block">
                  {ui.homeOrder.map((b, i) => {
                    const off = ui.homeHidden.includes(b);
                    return (
                      <div key={b} className={`order-row ${off ? "order-row--off" : ""}`}>
                        <span className="order-row__name">{blockName[b]}</span>
                        <button className="icon-btn" disabled={i === 0} aria-label={L("Выше", "Up")} onClick={() => moveBlock(b, -1)}>↑</button>
                        <button className="icon-btn" disabled={i === ui.homeOrder.length - 1} aria-label={L("Ниже", "Down")} onClick={() => moveBlock(b, 1)}>↓</button>
                        <Toggle on={!off} onChange={(v) => setUi({ homeHidden: v ? ui.homeHidden.filter((x) => x !== b) : [...ui.homeHidden, b] })} label={blockName[b]} />
                      </div>
                    );
                  })}
                </div>
                <div className="setting">
                  <span>{L("Сколько сборок показывать", "Builds shown")}</span>
                  <div className="seg">
                    {[3, 7, 11].map((n) => (
                      <button key={n} className={`seg__btn ${ui.homeBuilds === n ? "seg__btn--on" : ""}`} onClick={() => setUi({ homeBuilds: n })}>{n}</button>
                    ))}
                  </div>
                </div>
              </section>
            </>
          )}

          {show("ui") && (
            <>
              <section className="panel settings-list">
                <h2>{L("Меню", "Menu")}</h2>
                <div className="setting">
                  <span>{L("Вид бокового меню", "Sidebar style")}</span>
                  <div className="seg">
                    <button className={`seg__btn ${ui.sidebar === "full" ? "seg__btn--on" : ""}`} onClick={() => setUi({ sidebar: "full" })}>{L("С подписями", "With labels")}</button>
                    <button className={`seg__btn ${ui.sidebar === "compact" ? "seg__btn--on" : ""}`} onClick={() => setUi({ sidebar: "compact" })}>{L("Только значки", "Icons only")}</button>
                  </div>
                </div>
                <div className="setting">
                  <span>{L("Разделы в меню", "Menu sections")}</span>
                  <div className="seg-row">
                    {(["builds", "mods", "skins", "shots", "news"] as const).map((n) => (
                      <button key={n} className={`chip ${!ui.hiddenNav.includes(n) ? "chip--on" : ""}`} onClick={() => setUi({ hiddenNav: ui.hiddenNav.includes(n) ? ui.hiddenNav.filter((x) => x !== n) : [...ui.hiddenNav, n] })}>
                        {navName[n]}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="setting">
                  <span>{L("Открывать при запуске", "Open on start")}</span>
                  <select value={ui.startPage} onChange={(e) => setUi({ startPage: e.target.value as UiPrefs["startPage"] })} aria-label={L("Открывать при запуске", "Open on start")}>
                    {(["home", "builds", "mods", "skins", "shots", "news"] as const).filter((n) => n === "home" || !ui.hiddenNav.includes(n)).map((n) => (
                      <option key={n} value={n}>{navName[n]}</option>
                    ))}
                    <option value="last">{L("Где закончил в прошлый раз", "Where I left off")}</option>
                  </select>
                </div>
                <div className="setting">
                  <span>{t("set.lang")}</span>
                  <div className="seg">
                    {(["ru", "en"] as const).map((l) => (
                      <button key={l} className={`seg__btn ${settings.lang === l ? "seg__btn--on" : ""}`} onClick={() => updateSettings({ lang: l })}>
                        {l === "ru" ? "Русский" : "English"}
                      </button>
                    ))}
                  </div>
                </div>
              </section>
              <section className="panel settings-list">
                <h2>{L("Окно и трей", "Window and tray")}</h2>
                <p className="muted small">
                  {L(
                    "Трей — значки возле часов. Оттуда можно открыть лаунчер, запустить одну из последних сборок, остановить игру, открыть пульт и включить режим «Ютубер». Пока идёт игра, на значке горит зелёная точка.",
                    "The tray is the icons by the clock. From there you can open the launcher, start a recent build, stop the game, open the remote and toggle YouTuber mode. A green dot shows while the game runs.",
                  )}
                </p>
                <div className="setting">
                  <span>{L("Крестик окна сворачивает в трей, а не закрывает", "The ✕ button hides to the tray instead of quitting")}</span>
                  <Toggle on={ui.closeToTray} onChange={(v) => setUi({ closeToTray: v })} label="closeToTray" />
                </div>
                <div className="setting">
                  <span>{L("Запускать вместе с Windows (сразу в трей)", "Start with Windows (straight to the tray)")}</span>
                  <Toggle
                    on={ui.autostart}
                    label="autostart"
                    onChange={async (v) => {
                      try {
                        const now = await api.setAutostart(v);
                        setUi({ autostart: now });
                      } catch (e) {
                        toast(errText(e), "error");
                      }
                    }}
                  />
                </div>
                <div className="setting">
                  <span>{L("Уведомления Windows, когда лаунчер свёрнут (вылет игры и т. п.)", "Windows notifications while the launcher is hidden (game crash etc.)")}</span>
                  <Toggle on={ui.notifications} onChange={(v) => setUi({ notifications: v })} label="notifications" />
                </div>
              </section>
              <section className="panel settings-list">
                <h2>{L("Уведомления", "Notifications")}</h2>
                <div className="setting">
                  <span>{L("Где появляются", "Position")}</span>
                  <select value={ui.toastPos} onChange={(e) => setUi({ toastPos: e.target.value as UiPrefs["toastPos"] })} aria-label={L("Где появляются", "Position")}>
                    <option value="br">{L("Справа снизу", "Bottom right")}</option>
                    <option value="tr">{L("Справа сверху", "Top right")}</option>
                    <option value="bl">{L("Слева снизу", "Bottom left")}</option>
                    <option value="tl">{L("Слева сверху", "Top left")}</option>
                  </select>
                </div>
                <label className="setting">
                  <span>{L("Сколько висят", "How long they stay")}: <strong>{ui.toastSeconds} {L("с", "s")}</strong></span>
                  <div className="inline-form">
                    <input type="range" min={2} max={10} step={0.5} value={ui.toastSeconds} onChange={(e) => setUi({ toastSeconds: Number(e.target.value) })} />
                    <button className="btn btn--small" onClick={() => toast(L("Так выглядит уведомление", "This is a notification"), "ok")}>{L("Проверить", "Test")}</button>
                  </div>
                </label>
                <div className="setting">
                  <span>{L("Спрашивать перед удалением скриншотов", "Confirm before deleting screenshots")}</span>
                  <Toggle on={ui.confirmDelete} onChange={(v) => setUi({ confirmDelete: v })} label="confirm" />
                </div>
              </section>
              <section className="panel settings-list">
                <h2>{t("yt.mode")}</h2>
                <p className="muted small">{t("yt.hint")}</p>
                <div className="setting">
                  <span>{t("yt.mode")}</span>
                  <Toggle on={settings.streamer} onChange={(v) => updateSettings({ streamer: v })} label={t("yt.mode")} />
                </div>
                <label className="field">
                  <span>{t("yt.note")}</span>
                  <input value={settings.streamerNote} maxLength={120} placeholder={t("yt.notePh")} onChange={(e) => updateSettings({ streamerNote: e.target.value })} />
                </label>
              </section>
            </>
          )}

          {show("events") && (
            <section className="panel settings-list">
              <h2>{t("season.title")}</h2>
              <p className="muted small">{t("season.hint")}</p>
              {([
                ["newyear", "❄", L("Новый год", "New Year"), L("20 декабря – 10 января", "20 Dec – 10 Jan")],
                ["halloween", "🎃", L("Хэллоуин", "Halloween"), L("25 октября – 2 ноября", "25 Oct – 2 Nov")],
                ["birthday", "🎂", L("День рождения", "Birthday"), settings.birthday ? L("Дата указана ниже", "Date set below") : L("Укажи дату ниже", "Set the date below")],
              ] as const).map(([id, icon, name, when]) => (
                <div key={id} className="setting event-row">
                  <span className="event-row__icon" aria-hidden="true">{icon}</span>
                  <span className="event-row__text">
                    <strong>{name}</strong>
                    <span className="muted small">{when}</span>
                  </span>
                  <button className="btn btn--small" onClick={() => previewSeason(id)}>{L("Посмотреть", "Preview")}</button>
                  <Toggle on={ev[id]} onChange={(v) => setEv({ [id]: v })} label={name} />
                </div>
              ))}
              <label className="setting">
                <span>{t("season.birthday")}</span>
                <input type="date" value={settings.birthday ?? ""} max={new Date().toISOString().slice(0, 10)} onChange={(e) => updateSettings({ birthday: e.target.value || null })} />
              </label>
              <label className="setting">
                <span>{L("Частицы (снег, листья, конфетти)", "Particles (snow, leaves, confetti)")}: <strong>{ev.particles === 0 ? L("выкл.", "off") : `${Math.round(ev.particles * 100)}%`}</strong></span>
                <input type="range" min={0} max={2} step={0.25} value={ev.particles} onChange={(e) => setEv({ particles: Number(e.target.value) })} />
              </label>
              <div className="setting">
                <span>{L("Украшения на главной (тыквы, гирлянда, торт)", "Home decorations (pumpkins, garland, cake)")}</span>
                <Toggle on={ev.decor} onChange={(v) => setEv({ decor: v })} label="decor" />
              </div>
              <div className="setting">
                <span>{L("Частицы на всех страницах, а не только на главной", "Particles on every page, not just Home")}</span>
                <Toggle on={ev.everywhere} onChange={(v) => setEv({ everywhere: v })} label="everywhere" />
              </div>
            </section>
          )}

          {show("sound") && (
            <section className="panel settings-list">
              <h2>{t("snd.title")}</h2>
              <div className="setting">
                <span>{t("set.sounds")}</span>
                <Toggle on={settings.sounds} onChange={(v) => updateSettings({ sounds: v })} label={t("set.sounds")} />
              </div>
              <div className="setting">
                <span>{t("snd.pack")}</span>
                <div className="inline-form">
                  <select value={settings.soundPack} onChange={(e) => updateSettings({ soundPack: e.target.value })} aria-label={t("snd.pack")}>
                    <option value="theme">{t("snd.pack.theme")}</option>
                    {soundPacks.map((p) => (
                      <option key={p} value={p}>{t(`snd.pack.${p}` as Key)}</option>
                    ))}
                  </select>
                  <button className="btn btn--small" onClick={() => previewPack(settings.soundPack === "theme" ? "classic" : (settings.soundPack as (typeof soundPacks)[number]))}>{t("snd.preview")}</button>
                </div>
              </div>
              <label className="setting">
                <span>{t("snd.volume")}: <strong>{Math.round(settings.volume * 100)}%</strong></span>
                <input type="range" min={0} max={1} step={0.05} value={settings.volume} onChange={(e) => updateSettings({ volume: Number(e.target.value) })} />
              </label>
              <div className="setting">
                <span>{L("Звук при наведении", "Hover sound")}</span>
                <Toggle on={ui.hoverSounds} onChange={(v) => setUi({ hoverSounds: v })} label="hover" />
              </div>
              <div className="setting">
                <span>{L("Звук нажатий", "Click sound")}</span>
                <Toggle on={ui.clickSounds} onChange={(v) => setUi({ clickSounds: v })} label="click" />
              </div>
              <div className="setting">
                <span>{t("bg.ambient")}</span>
                <Toggle on={settings.ambient} onChange={(v) => updateSettings({ ambient: v })} label={t("bg.ambient")} />
              </div>
            </section>
          )}

          {show("game") && (
            <>
              <section className="panel settings-list">
                <h2>{L("Запуск", "Launching")}</h2>
                <label className="setting">
                  <span>{t("set.memory")}: <strong>{(settings.memoryMb / 1024).toFixed(1)} GB</strong></span>
                  <input type="range" min={1024} max={totalMb > 4096 ? Math.min(32768, Math.floor(totalMb / 512) * 512) : 16384} step={512} value={settings.memoryMb} onChange={(e) => updateSettings({ memoryMb: Number(e.target.value) })} />
                </label>
                {totalMb > 0 && <p className="muted small setting-note">{t("set.ramInfo", { total: Math.round(totalMb / 1024) })}</p>}
                <div className="setting">
                  <span>{t("set.onLaunch")}</span>
                  <select value={settings.onLaunch} onChange={(e) => updateSettings({ onLaunch: e.target.value as typeof settings.onLaunch })} aria-label={t("set.onLaunch")}>
                    <option value="keep">{t("set.onLaunch.keep")}</option>
                    <option value="minimize">{t("set.onLaunch.minimize")}</option>
                    <option value="hide">{t("set.onLaunch.hide")}</option>
                  </select>
                </div>
                <div className="setting">
                  <span>{L("Открывать консоль при запуске игры", "Open the console when the game starts")}</span>
                  <Toggle on={ui.consoleOnLaunch} onChange={(v) => setUi({ consoleOnLaunch: v })} label="console" />
                </div>
                <div className="setting">
                  <span>{t("mods.autoUpdate")}</span>
                  <Toggle on={settings.autoUpdateMods} onChange={(v) => updateSettings({ autoUpdateMods: v })} label={t("mods.autoUpdate")} />
                </div>
                <div className="setting">
                  <span>{t("cf.setting")}</span>
                  <Toggle on={settings.checkConflicts} onChange={(v) => updateSettings({ checkConflicts: v })} label={t("cf.setting")} />
                </div>
                <label className="field">
                  <span>{L("Аргументы Java для всех сборок", "Java arguments for every build")}</span>
                  <input value={settings.globalJvmArgs} placeholder="-XX:+UseG1GC -XX:MaxGCPauseMillis=50" spellCheck={false} onChange={(e) => updateSettings({ globalJvmArgs: e.target.value })} />
                  <span className="muted small">{L("Свои аргументы у сборки идут после этих и важнее их. Если не знаешь, что это, оставь пустым.", "A build's own arguments come after these and win. Leave empty if unsure.")}</span>
                </label>
              </section>
              <section className="panel settings-list">
                <h2>AniLauncher Link</h2>
                <p className="muted small">
                  {L(
                    "Маленький мод, который лаунчер сам подключает к сборкам на Fabric, Quilt, Forge и NeoForge, Minecraft от 1.20 до 26.3. Он не лежит в папке mods; на Fabric и Quilt скрыт в Mod Menu, на Forge и NeoForge виден в списке модов как «AniLauncher Link». Через него лаунчер показывает уведомления прямо в игре, включает ресурспаки без выхода, меняет настройки и управление на ходу, сохраняет скриншоты в галерею с названием мира и координатами и считает время в каждом мире. На серверах ничего не даёт и ничего не меняет в игровом процессе.",
                    "A small mod the launcher adds to builds on Fabric, Quilt, Forge and NeoForge, Minecraft 1.20 to 26.3. It is not in the mods folder; hidden in Mod Menu on Fabric and Quilt, listed as “AniLauncher Link” in the Forge and NeoForge mod list. Through it the launcher shows notifications in the game, turns resource packs on without leaving, changes settings and controls on the fly, saves screenshots to the gallery with the world name and coordinates, and counts time in each world. It gives no advantage on servers and changes nothing in gameplay.",
                  )}
                </p>
                <div className="setting">
                  <span>{L("Подключать AniLauncher Link к играм", "Add AniLauncher Link to games")}</span>
                  <Toggle on={settings.companion} onChange={(v) => updateSettings({ companion: v })} label="companion" />
                </div>
                {settings.companion && (
                  <>
                    <div className="setting">
                      <span>{L("Дублировать уведомления лаунчера в игру", "Mirror launcher notifications in the game")}</span>
                      <Toggle on={settings.link.mirrorToasts} onChange={(v) => updateSettings({ link: { ...settings.link, mirrorToasts: v } })} label="mirror" />
                    </div>
                    <div className="setting">
                      <span>{L("Сообщать в игре об обновлениях модов", "Tell about mod updates in the game")}</span>
                      <Toggle on={settings.link.notifyUpdates} onChange={(v) => updateSettings({ link: { ...settings.link, notifyUpdates: v } })} label="updates" />
                    </div>
                    <div className="setting">
                      <span>{L("Напоминать в игре о найденных несовместимостях", "Remind about found incompatibilities in the game")}</span>
                      <Toggle on={settings.link.notifyProblems} onChange={(v) => updateSettings({ link: { ...settings.link, notifyProblems: v } })} label="problems" />
                    </div>
                    <div className="setting">
                      <span>{L("Включать скачанные ресурспаки сразу в игре", "Switch downloaded resource packs on in the game")}</span>
                      <Toggle on={settings.link.packsLive} onChange={(v) => updateSettings({ link: { ...settings.link, packsLive: v } })} label="packs" />
                    </div>
                    <h3 className="settings-sub">{L("Игра в стиле лаунчера", "The game in the launcher's style")}</h3>
                    <p className="muted small">
                      {L(
                        "Для Minecraft 1.21 и новее на Fabric, Quilt, Forge и NeoForge. Арт и название сборки, твой ник и цвета текущей темы лаунчера. В новых меню всегда есть кнопка «Обычное меню», если нужно меню игры или другого мода. Меняется со следующего запуска игры.",
                        "For Minecraft 1.21 and newer on Fabric, Quilt, Forge and NeoForge. The build's art and name, your nick and the colours of the current launcher theme. The new menus always have a “Classic menu” button for the game's or another mod's menu. Applies from the next game start.",
                      )}
                    </p>
                    {(
                      [
                        ["menu", L("Главное меню: арт сборки и кнопка «Продолжить»", "Main menu: build art and a Continue button")],
                        ["pause", L("Меню паузы: панель с местом, временем и координатами", "Pause menu: a panel with place, time and coordinates")],
                        ["loading", L("Экран загрузки с артом сборки (у Forge и NeoForge остаётся их собственный)", "Loading screen with the build art (Forge and NeoForge keep their own)")],
                        ["buttons", L("Кнопки игры в цветах темы (ресурспак, его можно выключить в игре)", "Game buttons in the theme colours (a resource pack you can turn off in the game)")],
                      ] as const
                    ).map(([key, label]) => (
                      <div className="setting" key={key}>
                        <span>{label}</span>
                        <Toggle on={settings.gameUi[key]} onChange={(v) => updateSettings({ gameUi: { ...settings.gameUi, [key]: v } })} label={key} />
                      </div>
                    ))}
                  </>
                )}
                <p className="muted small">{L("Включение и выключение мода срабатывает со следующего запуска игры, остальное — сразу.", "Switching the mod on or off applies from the next game start; the rest applies at once.")}</p>
              </section>
              <BaseModsSettings />
              <GameDefaultsSettings />
            </>
          )}

          {show("data") && (
            <>
              <section className="panel settings-list">
                <h2>{t("set.login")}</h2>
                <label className="field">
                  <span>{t("set.clientId")}</span>
                  <div className="inline-form">
                    <input value={clientId} placeholder="00000000-0000-0000-0000-000000000000" onChange={(e) => setClientId(e.target.value)} />
                    <button
                      className="btn btn--primary"
                      onClick={async () => {
                        updateSettings({ clientId: clientId.trim() });
                        setTimeout(async () => setAccounts(await api.listAccounts()), 300);
                        toast(t("set.saved"), "ok");
                      }}
                    >
                      {t("create.save")}
                    </button>
                  </div>
                </label>
                <p className="muted small">{settings.builtInClientId ? t("set.clientIdBuiltIn") : t("set.clientIdHint")}</p>
                <div className="setting setting--stack">
                  <span>{t("set.data")}</span>
                  <code className="small">{settings.dataDir}</code>
                </div>
              </section>
              <UpdateSettings />
              <DiscordSettings />
              <BackupSettings />
              <section className="panel settings-list">
                <h2>{L("Настройки в файл", "Settings to a file")}</h2>
                <p className="muted small">{L("Сохрани настройки в файл, чтобы перенести на другой компьютер или отправить другу. Аккаунты и Client ID в файл не попадают.", "Save settings to a file to move them or share with a friend. Accounts and Client ID are not included.")}</p>
                <div className="inline-form setting-block">
                  <button className="btn" onClick={exportSettings}><Icon name="upload" size={16} /> {L("Сохранить в файл", "Export")}</button>
                  <button className="btn" onClick={importSettings}><Icon name="download" size={16} /> {L("Загрузить из файла", "Import")}</button>
                  <button className="btn" onClick={resetLook}><Icon name="undo" size={16} /> {L("Сбросить вид", "Reset look")}</button>
                </div>
              </section>
            </>
          )}
        </div>
      </div>
      {editing && <ThemeEditor theme={editing === "new" ? null : editing} onClose={() => setEditing(null)} />}
    </div>
  );
}
