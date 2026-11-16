import { useEffect, useRef, useState } from "react";
import { api, errText, type ControlProfile, type GameOptions, type Instance } from "../api";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

type Tab = "options" | "packs" | "controls";

/** Changes things inside a running game through AniLauncher Link. */
export function GameRemote({ inst, onClose }: { inst: Instance; onClose: () => void }) {
  const { settings, toast, linked } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [tab, setTab] = useState<Tab>("options");
  const world = linked[inst.id];
  const connected = inst.id in linked;

  useEffect(() => {
    if (!connected) onClose();
  }, [connected, onClose]);

  const where = !world
    ? L("В меню игры", "In the game menu")
    : world.kind === "singleplayer"
      ? L(`Мир «${world.name}»`, `World “${world.name}”`)
      : world.kind === "multiplayer"
        ? settings.streamer
          ? L("На сервере", "On a server")
          : L(`Сервер «${world.name}»`, `Server “${world.name}”`)
        : L("В игре", "In game");

  return (
    <Modal title={L(`Пульт игры · ${inst.name}`, `Game remote · ${inst.name}`)} onClose={onClose} wide>
      <div className="remote-head">
        <span className="remote-dot" aria-hidden="true" />
        <span>{where}</span>
        <button
          className="btn btn--small"
          onClick={() =>
            void api
              .linkSend(inst.id, { cmd: "toast", title: "AniLauncher", text: L("Связь с лаунчером работает", "Launcher link works") })
              .then(() => toast(L("Уведомление отправлено в игру", "Sent a notification to the game"), "ok", undefined, false))
              .catch((e) => toast(errText(e), "error", undefined, false))
          }
        >
          {L("Проверить связь", "Test link")}
        </button>
      </div>
      <div className="seg" role="tablist">
        {(
          [
            ["options", L("Настройки", "Settings")],
            ["packs", L("Ресурспаки", "Resource packs")],
            ["controls", L("Управление", "Controls")],
          ] as const
        ).map(([id, label]) => (
          <button key={id} role="tab" aria-selected={tab === id} className={`seg__btn ${tab === id ? "seg__btn--on" : ""}`} onClick={() => setTab(id)}>
            {label}
          </button>
        ))}
      </div>
      {tab === "options" && <OptionsTab inst={inst} />}
      {tab === "packs" && <PacksTab inst={inst} />}
      {tab === "controls" && <ControlsTab inst={inst} />}
    </Modal>
  );
}

function OptionsTab({ inst }: { inst: Instance }) {
  const { settings, toast } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [opts, setOpts] = useState<GameOptions | null>(null);
  const timers = useRef<Record<string, number>>({});

  useEffect(() => {
    api
      .linkSend(inst.id, { cmd: "getOptions" })
      .then((r) => setOpts(r.options as GameOptions))
      .catch((e) => toast(errText(e), "error", undefined, false));
  }, [inst.id, toast]);

  const change = <K extends keyof GameOptions>(key: K, value: GameOptions[K]) => {
    setOpts((o) => (o ? { ...o, [key]: value } : o));
    window.clearTimeout(timers.current[key]);
    timers.current[key] = window.setTimeout(() => {
      void api.linkSend(inst.id, { cmd: "setOptions", values: { [key]: value } }).catch((e) => toast(errText(e), "error", undefined, false));
    }, 250);
  };

  if (!opts) return <p className="muted">{L("Спрашиваю игру…", "Asking the game…")}</p>;
  const slider = (key: keyof GameOptions, label: string, min: number, max: number, step: number, show: (v: number) => string) => (
    <label className="setting" key={key}>
      <span>
        {label}: <strong>{show(opts[key] as number)}</strong>
      </span>
      <input type="range" min={min} max={max} step={step} value={opts[key] as number} onChange={(e) => change(key, Number(e.target.value) as never)} />
    </label>
  );
  const pct = (v: number) => `${Math.round(v * 100)}%`;
  return (
    <div className="settings-list">
      {slider("fov", L("Поле зрения", "Field of view"), 30, 110, 1, (v) => String(v))}
      {slider("renderDistance", L("Дальность прорисовки", "Render distance"), 2, 32, 1, (v) => L(`${v} чанков`, `${v} chunks`))}
      {slider("simulationDistance", L("Дальность симуляции", "Simulation distance"), 5, 32, 1, (v) => L(`${v} чанков`, `${v} chunks`))}
      {slider("gamma", L("Яркость", "Brightness"), 0, 1, 0.05, pct)}
      {slider("guiScale", L("Размер интерфейса", "GUI scale"), 0, 6, 1, (v) => (v === 0 ? L("авто", "auto") : String(v)))}
      {slider("maxFps", L("Ограничение FPS", "Max FPS"), 10, 260, 10, (v) => (v >= 260 ? L("без ограничения", "unlimited") : String(v)))}
      <div className="setting">
        <span>{L("Вертикальная синхронизация", "VSync")}</span>
        <Toggle on={opts.vsync} onChange={(v) => change("vsync", v)} label="vsync" />
      </div>
      {slider("master", L("Общая громкость", "Master volume"), 0, 1, 0.05, pct)}
      {slider("music", L("Музыка", "Music"), 0, 1, 0.05, pct)}
      <p className="muted small">{L("Меняется сразу в игре и сохраняется в её настройках.", "Changes apply in the game right away and are saved in its options.")}</p>
    </div>
  );
}

const BUILTIN: Record<string, [string, string]> = {
  programmer_art: ["Программистская графика", "Programmer Art"],
  high_contrast: ["Высокий контраст", "High Contrast"],
};

function PacksTab({ inst }: { inst: Instance }) {
  const { settings, toast } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [packs, setPacks] = useState<{ available: string[]; selected: string[] } | null>(null);
  const [busy, setBusy] = useState<string | null>(null);

  const load = () =>
    api
      .linkSend(inst.id, { cmd: "listPacks" })
      .then((r) => setPacks(r.packs as { available: string[]; selected: string[] }))
      .catch((e) => toast(errText(e), "error", undefined, false));
  useEffect(() => void load(), [inst.id]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!packs) return <p className="muted">{L("Спрашиваю игру…", "Asking the game…")}</p>;
  const shown = packs.available.filter((id) => id.startsWith("file/") || id in BUILTIN);
  const name = (id: string) => (id in BUILTIN ? BUILTIN[id][settings.lang === "ru" ? 0 : 1] : id.replace(/^file\//, "").replace(/\.zip$/i, ""));
  return (
    <div className="settings-list">
      {shown.length === 0 && <p className="muted">{L("В сборке нет ресурспаков. Найди их во вкладке «Моды» → «Ресурспаки» — они включатся прямо в игре.", "No resource packs in this build. Find some in Mods → Resource packs and they turn on right in the game.")}</p>}
      {shown.map((id) => {
        const on = packs.selected.includes(id);
        return (
          <div className="setting" key={id}>
            <span className="ellipsis">{name(id)}</span>
            {busy === id ? (
              <span className="muted small">{L("Перезагрузка ресурсов…", "Reloading resources…")}</span>
            ) : (
              <Toggle
                on={on}
                label={name(id)}
                onChange={async (v) => {
                  setBusy(id);
                  try {
                    await api.linkSend(inst.id, { cmd: v ? "enablePack" : "disablePack", pack: id });
                    sfx.success();
                    await load();
                  } catch (e) {
                    toast(errText(e), "error", undefined, false);
                  } finally {
                    setBusy(null);
                  }
                }}
              />
            )}
          </div>
        );
      })}
    </div>
  );
}

function ControlsTab({ inst }: { inst: Instance }) {
  const { settings, toast } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [list, setList] = useState<ControlProfile[] | null>(null);
  useEffect(() => {
    api.listControls().then(setList).catch(() => setList([]));
  }, []);
  if (!list) return null;
  return (
    <div className="content-list">
      {list.length === 0 && <p className="muted">{L("Профилей управления пока нет. Сохрани раскладку из любой сборки (меню сборки → «Профили управления»).", "No control profiles yet. Save one from any build (build menu → Control profiles).")}</p>}
      {list.map((p) => (
        <div key={p.name} className="content-row">
          <span className="mod-card__icon mod-card__icon--small">
            <Icon name="grid" size={20} />
          </span>
          <span className="content-row__text">
            <strong className="ellipsis">{p.name}</strong>
            <span className="muted small">{L(`${Object.keys(p.keys).length} клавиш`, `${Object.keys(p.keys).length} keys`)}</span>
          </span>
          <button
            className="btn btn--small btn--primary"
            onClick={async () => {
              try {
                await api.linkSend(inst.id, { cmd: "setKeys", keys: p.keys });
                sfx.success();
                toast(L(`Раскладка «${p.name}» применена в игре`, `Applied “${p.name}” in the game`), "ok");
              } catch (e) {
                toast(errText(e), "error", undefined, false);
              }
            }}
          >
            {L("Применить сейчас", "Apply now")}
          </button>
        </div>
      ))}
    </div>
  );
}
