import { useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { confirm } from "../components/Confirm";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { api, errText } from "../api";
import { useStore } from "../store";
import { Modal, Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

function stamp() {
  const d = new Date();
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}_${p(d.getHours())}-${p(d.getMinutes())}`;
}

function safeName(s: string) {
  return s.replace(/[\\/:*?"<>|]+/g, "").trim().slice(0, 40) || "build";
}

function size(n: number) {
  if (n >= 1024 ** 3) return `${(n / 1024 ** 3).toFixed(1)} ГБ`;
  if (n >= 1024 ** 2) return `${(n / 1024 ** 2).toFixed(0)} МБ`;
  return `${Math.max(1, Math.round(n / 1024))} КБ`;
}

/** One button that packs logs, crash reports and system info into a zip to send for help. */
export function ReportDialog({ instanceId, onClose }: { instanceId?: string | null; onClose: () => void }) {
  const { settings, instances, toast } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [target, setTarget] = useState<string>(instanceId ?? "");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const inst = instances.find((i) => i.id === target);

  const make = async () => {
    try {
      const dest = await save({
        defaultPath: `AniLauncher-отчёт-${inst ? safeName(inst.name) + "-" : ""}${stamp()}.zip`,
        filters: [{ name: "ZIP", extensions: ["zip"] }],
      });
      if (!dest) return;
      setBusy(true);
      await api.bugReport(dest, target || null, note);
      sfx.success();
      toast(L("Отчёт сохранён — отправь этот файл", "Report saved — send this file"), "ok", {
        label: L("Показать в папке", "Show in folder"),
        run: () => void api.revealFile(dest).catch(() => {}),
      });
      onClose();
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  return (
    <Modal title={L("Отчёт об ошибке", "Bug report")} onClose={onClose}>
      <p className="muted small">
        {L(
          "Соберёт в один архив логи игры, отчёты о вылетах, список модов и сведения о компьютере. Токены входа, путь с именем пользователя Windows и Client ID в отчёт не попадают.",
          "Packs game logs, crash reports, the mod list and computer info into one zip. Login tokens, your Windows user name and the Client ID are left out.",
        )}
      </p>
      <label className="field">
        <span>{L("Про какую сборку", "Which build")}</span>
        <select value={target} onChange={(e) => setTarget(e.target.value)}>
          <option value="">{L("Все сборки (общий отчёт)", "All builds (overview)")}</option>
          {instances.map((i) => (
            <option key={i.id} value={i.id}>
              {i.name} · {i.loader} {i.mcVersion}
            </option>
          ))}
        </select>
      </label>
      <label className="field">
        <span>{L("Что случилось", "What happened")}</span>
        <textarea
          rows={4}
          value={note}
          maxLength={2000}
          placeholder={L("Например: игра вылетает, когда захожу в мир с шейдерами", "For example: the game crashes when I join a world with shaders")}
          onChange={(e) => setNote(e.target.value)}
        />
      </label>
      <div className="modal__foot">
        <button className="btn" onClick={onClose}>
          {L("Отмена", "Cancel")}
        </button>
        <button className="btn btn--primary" disabled={busy} onClick={make}>
          <Icon name="download" size={16} /> {busy ? L("Собираю…", "Packing…") : L("Сохранить отчёт", "Save report")}
        </button>
      </div>
    </Modal>
  );
}

/** Settings section: back up the whole launcher to a zip and restore it (e.g. after reinstalling Windows). */
export function BackupSettings() {
  const { settings, toast, running } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [opts, setOpts] = useState({ worlds: true, history: false, accounts: false });
  const [est, setEst] = useState<{ files: number; bytes: number; builds: number } | null>(null);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [report, setReport] = useState(false);

  useEffect(() => {
    let alive = true;
    api.estimateLauncherBackup(opts).then((e) => alive && setEst(e)).catch(() => {});
    return () => {
      alive = false;
    };
  }, [opts]);
  useEffect(() => {
    const off = listen<{ done: number; total: number }>("backup-progress", (e) => setProgress(e.payload));
    return () => void off.then((f) => f());
  }, []);

  const create = async () => {
    try {
      const dest = await save({ defaultPath: `AniLauncher-копия-${stamp()}.zip`, filters: [{ name: "ZIP", extensions: ["zip"] }] });
      if (!dest) return;
      setProgress({ done: 0, total: est?.files ?? 1 });
      await api.createLauncherBackup(dest, opts);
      sfx.success();
      toast(L("Копия готова. Сохрани её на флешку или в облако — не на диск, который будешь форматировать.", "Backup ready. Keep it on a USB stick or in the cloud, not on the drive you will wipe."), "ok", {
        label: L("Показать в папке", "Show in folder"),
        run: () => void api.revealFile(dest).catch(() => {}),
      });
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setProgress(null);
    }
  };

  const restore = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "ZIP", extensions: ["zip"] }] });
      if (typeof file !== "string") return;
      const info = await api.inspectLauncherBackup(file);
      const when = new Date(info.created * 1000).toLocaleString(settings.lang === "ru" ? "ru-RU" : "en-GB", { dateStyle: "long", timeStyle: "short" });
      const ok = await confirm(
        L(
          `Копия от ${when}: сборок — ${info.builds}${info.worlds ? ", с мирами" : ", без миров"}${info.accounts ? ", с аккаунтами" : ""}.\n\nНастройки заменятся на те, что в копии; сборки из копии вернутся (с тем же номером — заменятся), остальные останутся. Продолжить?`,
          `Backup from ${when}: ${info.builds} builds${info.worlds ? ", with worlds" : ", without worlds"}${info.accounts ? ", with accounts" : ""}.\n\nSettings will be replaced with the backup's; builds from it come back (same ones are replaced), others stay. Continue?`,
        ),
        { title: "AniLauncher", kind: "warning" },
      );
      if (!ok) return;
      setProgress({ done: 0, total: 1 });
      const n = await api.restoreLauncherBackup(file);
      toast(L(`Восстановлено файлов: ${n}. Перезапускаю интерфейс…`, `Restored ${n} files. Reloading…`), "ok");
      setTimeout(() => window.location.reload(), 1200);
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setProgress(null);
    }
  };

  const pct = progress && progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;
  return (
    <>
      <section className="panel settings-list">
        <h2>{L("Резервная копия лаунчера", "Launcher backup")}</h2>
        <p className="muted small">
          {L(
            "Всё твоё в одном файле: настройки и темы, профиль и аватарка, скины, профили управления, статистика и все сборки с модами и настройками. Сам Minecraft, Java и библиотеки не копируются — лаунчер скачает их заново. Перед переустановкой Windows сделай копию, а потом в новом лаунчере нажми «Восстановить».",
            "Everything of yours in one file: settings and themes, profile and avatar, skins, control profiles, stats and every build with its mods and settings. Minecraft itself, Java and libraries are not copied — the launcher downloads them again.",
          )}
        </p>
        <div className="setting">
          <span>{L("Миры и скриншоты", "Worlds and screenshots")}</span>
          <Toggle on={opts.worlds} onChange={(v) => setOpts({ ...opts, worlds: v })} label="worlds" />
        </div>
        <div className="setting">
          <span>{L("Точки восстановления и бэкапы миров", "Restore points and world backups")}</span>
          <Toggle on={opts.history} onChange={(v) => setOpts({ ...opts, history: v })} label="history" />
        </div>
        <div className="setting">
          <span>
            {L("Аккаунты (не придётся входить заново)", "Accounts (no need to sign in again)")}
            {opts.accounts && (
              <span className="small copy-warn">
                {" "}
                {L("— в файле будет вход в твой аккаунт, никому его не давай", "— the file will contain your login, never share it")}
              </span>
            )}
          </span>
          <Toggle on={opts.accounts} onChange={(v) => setOpts({ ...opts, accounts: v })} label="accounts" />
        </div>
        {est && (
          <p className="muted small">
            {L(`Сборок: ${est.builds} · файлов: ${est.files} · примерно ${size(est.bytes)} до сжатия`, `Builds: ${est.builds} · files: ${est.files} · about ${size(est.bytes)} before compression`)}
          </p>
        )}
        {progress ? (
          <div className="backup-progress" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
            <span style={{ width: `${pct}%` }} />
            <em>{pct}%</em>
          </div>
        ) : (
          <div className="inline-form setting-block">
            <button className="btn btn--primary" onClick={create}>
              <Icon name="upload" size={16} /> {L("Создать копию", "Create backup")}
            </button>
            <button className="btn" disabled={running.length > 0} title={running.length ? L("Сначала закрой игру", "Close the game first") : undefined} onClick={restore}>
              <Icon name="download" size={16} /> {L("Восстановить из копии", "Restore from backup")}
            </button>
          </div>
        )}
      </section>
      <section className="panel settings-list">
        <h2>{L("Отчёт об ошибке", "Bug report")}</h2>
        <p className="muted small">{L("Если что-то сломалось — собери отчёт и отправь файл. Его же можно сделать из консоли сборки.", "If something breaks, make a report and send the file. Also available from a build's console.")}</p>
        <div className="inline-form setting-block">
          <button className="btn" onClick={() => setReport(true)}>
            <Icon name="terminal" size={16} /> {L("Собрать отчёт", "Make a report")}
          </button>
        </div>
      </section>
      {report && <ReportDialog onClose={() => setReport(false)} />}
    </>
  );
}

/** Discord status settings with a preview of how friends see it. */
export function DiscordSettings() {
  const { settings, updateSettings, instances } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const d = settings.discord;
  const set = (patch: Partial<typeof d>) => updateSettings({ discord: { ...d, ...patch } });
  const sample = instances[0];
  const details = settings.streamer || !d.showBuild ? L("Играет в Minecraft", "Playing Minecraft") : `${L("Сборка", "Build")}: ${sample?.name ?? "Выживание с друзьями"}`;
  const state = `${sample ? ({ fabric: "Fabric", quilt: "Quilt", forge: "Forge", neoforge: "NeoForge" } as Record<string, string>)[sample.loader] ?? "Vanilla" : "Fabric"} ${sample?.mcVersion ?? "1.21.11"}${d.showWorld ? L(" · в одиночном мире", " · in singleplayer") : ""}`;
  return (
    <section className="panel settings-list">
      <h2>Discord</h2>
      <p className="muted small">
        {L(
          "Статус в профиле Discord: во что ты играешь. Работает, когда Discord открыт на этом компьютере; если он закрыт — ничего не происходит. В режиме «Ютубер» название сборки скрывается. Адреса серверов и названия миров не показываются никогда.",
          "Status in your Discord profile: what you are playing. Works while Discord is open on this computer. YouTuber mode hides the build name. Server addresses and world names are never shown.",
        )}
      </p>
      <div className="setting">
        <span>{L("Показывать статус в Discord", "Show status in Discord")}</span>
        <Toggle on={d.enabled} onChange={(v) => set({ enabled: v })} label="discord" />
      </div>
      {d.enabled && (
        <>
          <div className="discord-preview" aria-label={L("Как это видят друзья", "How friends see it")}>
            <span className="discord-preview__label">{L("ИГРАЕТ В", "PLAYING")}</span>
            <div className="discord-preview__row">
              <img src="/logo.png" alt="" />
              <div>
                <strong>AniLauncher</strong>
                <span>{details}</span>
                <span>{state}</span>
                {d.showTime && <span>{L("01:23:45 прошло", "01:23:45 elapsed")}</span>}
              </div>
            </div>
          </div>
          <div className="setting">
            <span>{L("Название сборки", "Build name")}</span>
            <Toggle on={d.showBuild} onChange={(v) => set({ showBuild: v })} label="build" />
          </div>
          <div className="setting">
            <span>{L("Где ты: в меню, в одиночном мире или на сервере", "Where you are: menu, singleplayer or a server")}</span>
            <Toggle on={d.showWorld} onChange={(v) => set({ showWorld: v })} label="world" />
          </div>
          <div className="setting">
            <span>{L("Таймер, сколько играешь", "Time played")}</span>
            <Toggle on={d.showTime} onChange={(v) => set({ showTime: v })} label="time" />
          </div>
          <div className="setting">
            <span>{L("Статус, когда просто открыт лаунчер", "Status while only the launcher is open")}</span>
            <Toggle on={d.showIdle} onChange={(v) => set({ showIdle: v })} label="idle" />
          </div>
          <label className="field">
            <span>{L("ID приложения Discord (пусто — встроенный AniLauncher)", "Discord application ID (empty — built-in AniLauncher)")}</span>
            <input value={d.appId} inputMode="numeric" placeholder="1571852560866873454" onChange={(e) => set({ appId: e.target.value.replace(/\D/g, "") })} />
          </label>
        </>
      )}
    </section>
  );
}

/** "What's new" and the update button, with download progress. */
export function UpdateDialog({ onClose }: { onClose: () => void }) {
  const { settings, update, toast, running } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [progress, setProgress] = useState<{ done: number; total: number | null } | null>(null);
  useEffect(() => {
    const off = listen<{ done: number; total: number | null }>("update-progress", (e) => setProgress(e.payload));
    return () => void off.then((f) => f());
  }, []);
  if (!update) return null;
  const pct = progress?.total ? Math.round((progress.done / progress.total) * 100) : null;
  return (
    <Modal title={L(`Новая версия ${update.version}`, `Version ${update.version}`)} onClose={progress ? () => {} : onClose}>
      <p className="muted small">{L(`Сейчас у тебя ${update.current}.`, `You have ${update.current}.`)}</p>
      <div className="update-notes">{update.notes?.trim() || L("Описание изменений не добавлено.", "No release notes.")}</div>
      <p className="muted small">
        {L(
          "Лаунчер скачает обновление, установит его и перезапустится. Сборки, миры и настройки останутся на месте.",
          "The launcher downloads the update, installs it and restarts. Builds, worlds and settings stay.",
        )}
      </p>
      {progress ? (
        <div className="backup-progress" role="progressbar" aria-valuenow={pct ?? 0} aria-valuemin={0} aria-valuemax={100}>
          <span style={{ width: `${pct ?? 30}%` }} />
          <em>{pct !== null ? `${pct}%` : L("Скачиваю…", "Downloading…")}</em>
        </div>
      ) : (
        <div className="modal__foot">
          <button className="btn" onClick={onClose}>
            {L("Позже", "Later")}
          </button>
          <button
            className="btn btn--primary"
            disabled={running.length > 0}
            title={running.length ? L("Сначала закрой игру", "Close the game first") : undefined}
            onClick={async () => {
              setProgress({ done: 0, total: null });
              try {
                await api.installUpdate();
              } catch (e) {
                setProgress(null);
                toast(errText(e), "error");
              }
            }}
          >
            <Icon name="download" size={16} /> {L("Обновить и перезапустить", "Update and restart")}
          </button>
        </div>
      )}
    </Modal>
  );
}

/** Settings section: version, manual check, automatic check toggle. */
export function UpdateSettings() {
  const { settings, updateSettings, update, setUpdate, openUpdate, toast } = useStore();
  const L = (ru: string, en: string) => (settings.lang === "ru" ? ru : en);
  const [version, setVersion] = useState("");
  const [configured, setConfigured] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [latest, setLatest] = useState(false);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
    api.updatesConfigured().then(setConfigured).catch(() => setConfigured(false));
  }, []);
  const check = async () => {
    setBusy(true);
    setLatest(false);
    try {
      const u = await api.checkUpdate();
      setUpdate(u);
      if (u) openUpdate(true);
      else setLatest(true);
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };
  return (
    <section className="panel settings-list">
      <h2>{L("Обновления лаунчера", "Launcher updates")}</h2>
      <div className="setting">
        <span>
          AniLauncher <strong>{version}</strong>
          {latest && <span className="muted small"> · {L("это последняя версия", "this is the latest version")}</span>}
        </span>
        {update ? (
          <button className="btn btn--primary btn--small" onClick={() => openUpdate(true)}>
            {L(`Обновить до ${update.version}`, `Update to ${update.version}`)}
          </button>
        ) : (
          <button className="btn btn--small" disabled={busy || configured === false} onClick={check}>
            {busy ? L("Проверяю…", "Checking…") : L("Проверить обновления", "Check for updates")}
          </button>
        )}
      </div>
      {configured === false && (
        <p className="muted small">
          {L(
            "В этой сборке обновления не настроены: они включаются, когда лаунчер собран через «Выпустить версию» на GitHub (см. README).",
            "Updates are not set up in this build: they turn on when the launcher is built with “Release a version” on GitHub (see README).",
          )}
        </p>
      )}
      <div className="setting">
        <span>{L("Проверять обновления при запуске", "Check for updates on start")}</span>
        <Toggle on={settings.ui.autoUpdateCheck} onChange={(v) => updateSettings({ ui: { ...settings.ui, autoUpdateCheck: v } })} label="autoUpdateCheck" />
      </div>
    </section>
  );
}
