import { useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, artSrc, errText, type Art, type GameVersion, type Instance, type Loader } from "../api";
import { useStore } from "../store";
import { DefaultArt, Modal, Toggle, loaderLabel } from "../components/Bits";
import { Icon } from "../components/Icon";
import { ArtPicker } from "./ArtPicker";
import { installBaseMods } from "./Extras";
import { sfx } from "../sounds";
import type { Key } from "../i18n";

const loaders: Array<{ id: Loader | "neoforge" | "forge"; label: string; soon?: boolean }> = [
  { id: "vanilla", label: "Vanilla" },
  { id: "fabric", label: "Fabric" },
  { id: "quilt", label: "Quilt" },
  { id: "neoforge", label: "NeoForge" },
  { id: "forge", label: "Forge" },
];

/** Create a new build, or edit an existing one when `instance` is passed. */
export function BuildDialog({ instance, onClose }: { instance?: Instance; onClose: () => void }) {
  const { t, toast, reloadInstances, select, settings } = useStore();
  const editing = !!instance;
  const [nameInput, setName] = useState(instance?.name ?? "");
  const [loader, setLoader] = useState<Loader>(instance?.loader ?? "fabric");
  const [versions, setVersions] = useState<GameVersion[] | null>(null);
  const [snapshots, setSnapshots] = useState(false);
  const [mc, setMc] = useState(instance?.mcVersion ?? "");
  const [loaderVersions, setLoaderVersions] = useState<string[]>([]);
  const [loaderVersion, setLoaderVersion] = useState(instance?.loaderVersion ?? "");
  const [art, setArt] = useState<Art | null>(instance?.art ?? null);
  const [memory, setMemory] = useState<number | null>(instance?.memoryMb ?? null);
  const [jvm, setJvm] = useState(instance?.jvmArgs ?? "");
  const [pickingArt, setPickingArt] = useState(false);
  const [width, setWidth] = useState(instance?.width ? String(instance.width) : "");
  const [height, setHeight] = useState(instance?.height ? String(instance.height) : "");
  const [fullscreen, setFullscreen] = useState(!!instance?.fullscreen);
  const [server, setServer] = useState(instance?.server ?? "");
  const [javaPath, setJavaPath] = useState(instance?.javaPath ?? "");
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let alive = true;
    setVersions(null);
    api
      .listGameVersions(loader)
      .then((v) => {
        if (!alive) return;
        setVersions(v);
        setMc((cur) => (cur && v.some((x) => x.id === cur) ? cur : v.find((x) => x.kind === "release")?.id ?? ""));
      })
      .catch((e) => alive && toast(errText(e), "error"));
    return () => {
      alive = false;
    };
  }, [loader, toast]);

  useEffect(() => {
    if (loader === "vanilla" || !mc) {
      setLoaderVersions([]);
      return;
    }
    let alive = true;
    api
      .listLoaderVersions(loader, mc)
      .then((v) => alive && setLoaderVersions(v))
      .catch(() => alive && setLoaderVersions([]));
    return () => {
      alive = false;
    };
  }, [loader, mc]);

  const shown = useMemo(
    () => (versions ?? []).filter((v) => snapshots || v.kind === "release" || v.id === mc),
    [versions, snapshots, mc],
  );

  const autoName = `${loader === "vanilla" ? "Minecraft" : loaderLabel(loader)} ${mc}`.trim();
  const save = async () => {
    if (!mc) return;
    const name = nameInput.trim() || autoName;
    setBusy(true);
    try {
      if (editing && instance) {
        await api.updateInstance({
          ...instance,
          name,
          mcVersion: mc,
          loader,
          loaderVersion: loaderVersion || null,
          art,
          memoryMb: memory,
          jvmArgs: jvm.trim() || null,
          width: Number(width) || null,
          height: Number(height) || null,
          fullscreen,
          server: server.trim() || null,
          javaPath: javaPath || null,
        });
      } else {
        const created = await api.createInstance({ name, mcVersion: mc, loader, loaderVersion: loaderVersion || null, art });
        select(created.id);
        toast(t("toast.created"), "ok");
        if ((settings.baseMods[loader] ?? []).length > 0) {
          toast(t("base.installing"));
          void installBaseMods(created.id, loader, settings.baseMods).then((n) => n && toast(t("base.done", { n }), "ok"));
        }
      }
      sfx.success();
      await reloadInstances();
      onClose();
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const src = artSrc(art);
  return (
    <>
      <Modal title={editing ? t("edit.title") : t("create.title")} onClose={onClose} wide>
        <div className="build-form">
          <button className="build-form__art" onClick={() => setPickingArt(true)} aria-label={t("create.art")}>
            {src ? <img className="art-fill" src={src} alt="" /> : <DefaultArt />}
            <span className="build-form__art-label">
              <Icon name="image" size={16} /> {t("create.art")}
            </span>
          </button>

          <label className="field">
            <span>{t("create.name")}</span>
            <input value={nameInput} maxLength={48} placeholder={editing ? t("create.namePh") : autoName} onChange={(e) => setName(e.target.value)} autoFocus />
          </label>

          <div className="field">
            <span>{t("create.loader")}</span>
            <div className="seg-row">
              {loaders.map((l) => (
                <button
                  key={l.id}
                  className={`chip ${loader === l.id ? "chip--on" : ""}`}
                  disabled={l.soon}
                  onClick={() => {
                    sfx.click();
                    if (!l.soon && l.id !== loader) {
                      setLoader(l.id as Loader);
                      setLoaderVersion("");
                    }
                  }}
                >
                  {l.label}
                  {l.soon && <small> · {t("create.soon")}</small>}
                </button>
              ))}
            </div>
            <span className="muted small">{t(`create.loaderHint.${loader}` as Key)}</span>
          </div>

          <div className="field-row">
            <label className="field">
              <span>{t("create.version")}</span>
              <select
                value={mc}
                onChange={(e) => {
                  setMc(e.target.value);
                  setLoaderVersion("");
                }}
                disabled={!versions}
              >
                {!versions && <option>{t("create.loading")}</option>}
                {shown.map((v) => (
                  <option key={v.id} value={v.id}>
                    {v.id}
                    {v.kind === "snapshot" ? " (snapshot)" : ""}
                  </option>
                ))}
              </select>
            </label>
            {loader !== "vanilla" && (
              <label className="field">
                <span>{t("create.loaderVersion")}</span>
                <select value={loaderVersion} onChange={(e) => setLoaderVersion(e.target.value)}>
                  <option value="">{t("create.latest")}</option>
                  {loaderVersions.map((v) => (
                    <option key={v} value={v}>
                      {v}
                    </option>
                  ))}
                </select>
              </label>
            )}
          </div>
          {(loader === "forge" || loader === "neoforge") && <p className="muted small">{t("create.forgeSlow")}</p>}
          <label className="check">
            <input type="checkbox" checked={snapshots} onChange={(e) => setSnapshots(e.target.checked)} />
            {t("create.snapshots")}
          </label>

          {editing && (
            <div className="field-row">
              <label className="field">
                <span>
                  {t("create.memory")}: {memory ? `${(memory / 1024).toFixed(1)} GB` : `${t("create.memoryDefault")} (${(settings.memoryMb / 1024).toFixed(1)} GB)`}
                </span>
                <input
                  type="range"
                  min={1024}
                  max={16384}
                  step={512}
                  value={memory ?? settings.memoryMb}
                  onChange={(e) => setMemory(Number(e.target.value))}
                />
              </label>
              <label className="field">
                <span>{t("create.jvm")}</span>
                <input value={jvm} placeholder="-XX:+UseG1GC" onChange={(e) => setJvm(e.target.value)} />
              </label>
            </div>
          )}

          {editing && (
            <>
              <h3 className="section-title">{t("launch.section")}</h3>
              <div className="field-row">
                <div className="field">
                  <span>{t("launch.size")}</span>
                  <div className="inline-form">
                    <input inputMode="numeric" value={width} disabled={fullscreen} placeholder="854" aria-label={t("launch.width")} onChange={(e) => setWidth(e.target.value.replace(/\D/g, "").slice(0, 5))} style={{ minWidth: 0, width: 90, flex: "none" }} />
                    <span className="muted">×</span>
                    <input inputMode="numeric" value={height} disabled={fullscreen} placeholder="480" aria-label={t("launch.height")} onChange={(e) => setHeight(e.target.value.replace(/\D/g, "").slice(0, 5))} style={{ minWidth: 0, width: 90, flex: "none" }} />
                    <label className="check-inline">
                      {t("launch.fullscreen")}
                      <Toggle on={fullscreen} onChange={setFullscreen} label={t("launch.fullscreen")} />
                    </label>
                  </div>
                </div>
                <label className="field">
                  <span>{t("launch.server")}</span>
                  <input value={server} maxLength={120} placeholder={t("launch.serverPh")} onChange={(e) => setServer(e.target.value)} />
                </label>
              </div>
              <div className="field">
                <span>{t("launch.java")}</span>
                <div className="inline-form">
                  <input value={javaPath || t("launch.javaAuto")} readOnly aria-label={t("launch.java")} />
                  <button
                    className="btn"
                    onClick={async () => {
                      const f = await open({ multiple: false, filters: [{ name: "java.exe", extensions: ["exe"] }] });
                      if (typeof f === "string") setJavaPath(f);
                    }}
                  >
                    {t("launch.javaPick")}
                  </button>
                  {javaPath && (
                    <button className="btn" onClick={() => setJavaPath("")}>
                      {t("launch.javaReset")}
                    </button>
                  )}
                </div>
              </div>
            </>
          )}

          <div className="modal__foot">
            <button className="btn" onClick={onClose}>
              {t("create.cancel")}
            </button>
            <button className="btn btn--primary" disabled={busy || !mc} onClick={save}>
              {editing ? t("create.save") : t("create.create")}
            </button>
          </div>
        </div>
      </Modal>
      {pickingArt && (
        <ArtPicker
          onClose={() => setPickingArt(false)}
          onPick={(a) => {
            setArt(a);
            setPickingArt(false);
          }}
        />
      )}
    </>
  );
}
