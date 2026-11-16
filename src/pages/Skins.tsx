import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { api, errText, fileSrc, type SavedSkin, type SkinProfile } from "../api";
import { useStore } from "../store";
import { Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { SkinThumb, SkinView } from "../components/SkinView";
import { SkinEditor } from "./SkinEditor";
import { sfx } from "../sounds";

type Selected = { kind: "account" } | { kind: "saved"; skin: SavedSkin };

function CapeThumb({ url }: { url: string }) {
  const [src, setSrc] = useState<string | null>(null);
  useEffect(() => {
    api.textureData(url).then(setSrc).catch(() => setSrc(null));
  }, [url]);
  return (
    <span className="cape-thumb" style={src ? { backgroundImage: `url("${src}")` } : undefined} aria-hidden="true" />
  );
}

export function Skins() {
  const { t, toast, accounts } = useStore();
  const active = accounts.accounts.find((a) => a.active);
  const licensed = active?.kind === "msa" && !active.needsName;
  const [skins, setSkins] = useState<SavedSkin[]>([]);
  const [profile, setProfile] = useState<SkinProfile | null>(null);
  const [selected, setSelected] = useState<Selected>({ kind: "account" });
  const [skinData, setSkinData] = useState<string | null>(null);
  const [capeData, setCapeData] = useState<string | null>(null);
  const [animate, setAnimate] = useState(true);
  const [editing, setEditing] = useState<SavedSkin | null | "new">(null);
  const [busy, setBusy] = useState(false);
  const [name, setName] = useState("");

  const loadSkins = useCallback(async () => {
    try {
      setSkins(await api.listSkins());
    } catch (e) {
      toast(errText(e), "error");
    }
  }, [toast]);

  useEffect(() => {
    void loadSkins();
    if (licensed) api.skinProfile().then(setProfile).catch((e) => toast(errText(e), "error"));
  }, [loadSkins, licensed, toast]);

  useEffect(() => {
    const offs = [
      listen<SavedSkin>("skin-added", () => {
        sfx.success();
        toast(t("skins.added"), "ok");
        void loadSkins();
      }),
      listen<string>("skin-error", (e) => toast(e.payload, "error")),
    ];
    return () => offs.forEach((p) => void p.then((f) => f()));
  }, [loadSkins, toast, t]);

  // What the 3D view shows
  useEffect(() => {
    const src = selected.kind === "saved" ? selected.skin.file : profile?.skinUrl;
    if (selected.kind === "saved") setName(selected.skin.name);
    if (!src) {
      setSkinData(null);
      return;
    }
    api.textureData(src).then(setSkinData).catch(() => setSkinData(null));
  }, [selected, profile?.skinUrl]);

  useEffect(() => {
    const cape = profile?.capes.find((c) => c.active);
    if (!cape) {
      setCapeData(null);
      return;
    }
    api.textureData(cape.url).then(setCapeData).catch(() => setCapeData(null));
  }, [profile]);

  const slim = selected.kind === "saved" ? selected.skin.variant === "slim" : profile?.variant === "slim";

  const addFile = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "PNG", extensions: ["png"] }] });
      if (typeof file !== "string") return;
      const base = file.split(/[\\/]/).pop()?.replace(/\.png$/i, "") ?? "Skin";
      const s = await api.importSkin(file, base, "classic");
      sfx.success();
      await loadSkins();
      setSelected({ kind: "saved", skin: s });
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const wear = async (s: SavedSkin) => {
    setBusy(true);
    try {
      setProfile(await api.uploadSkin(s.id));
      sfx.success();
      toast(t("skins.worn"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const sel = selected.kind === "saved" ? selected.skin : null;

  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("skins.title")}</h1>
        <div className="page-head__tools">
          <button className="btn" onClick={addFile}>
            <Icon name="download" size={16} /> {t("skins.addFile")}
          </button>
          <button className="btn" onClick={() => api.openNameMC().catch((e) => toast(errText(e), "error"))} title={t("skins.namemcHint")}>
            <Icon name="link" size={16} /> {t("skins.namemc")}
          </button>
          <button className="btn btn--primary" onClick={() => setEditing("new")}>
            <Icon name="palette" size={16} /> {t("skins.new")}
          </button>
        </div>
      </div>

      <div className="skins-layout">
        <section className="panel skin-stage">
          <div className="skin-stage__view">
            {skinData ? <SkinView skin={skinData} cape={selected.kind === "account" ? capeData : null} slim={slim} animate={animate} width={300} height={400} /> : <div className="skin-stage__empty"><Icon name="skins" size={48} /></div>}
          </div>
          <label className="check-inline">
            {t("skins.walk")}
            <Toggle on={animate} onChange={setAnimate} label={t("skins.walk")} />
          </label>
          {sel ? (
            <>
              <label className="field">
                <span>{t("skins.name")}</span>
                <input
                  value={name}
                  maxLength={40}
                  onChange={(e) => setName(e.target.value)}
                  onBlur={() => void api.updateSkin(sel.id, name, sel.variant).then(loadSkins)}
                />
              </label>
              <div className="seg seg--panel">
                {(["classic", "slim"] as const).map((v) => (
                  <button
                    key={v}
                    className={`seg__btn ${sel.variant === v ? "seg__btn--on" : ""}`}
                    onClick={async () => {
                      await api.updateSkin(sel.id, name, v);
                      const list = await api.listSkins();
                      setSkins(list);
                      const again = list.find((x) => x.id === sel.id);
                      if (again) setSelected({ kind: "saved", skin: again });
                    }}
                  >
                    {v === "classic" ? t("skins.classic") : t("skins.slim")}
                  </button>
                ))}
              </div>
              <div className="inline-form">
                <button className="btn btn--primary" disabled={!licensed || busy} onClick={() => void wear(sel)}>
                  {t("skins.wear")}
                </button>
                <button className="btn" onClick={() => setEditing(sel)}>
                  <Icon name="edit" size={16} /> {t("skins.edit")}
                </button>
                <button
                  className="icon-btn icon-btn--danger"
                  aria-label={t("skins.delete")}
                  title={t("skins.delete")}
                  onClick={async () => {
                    await api.deleteSkin(sel.id);
                    setSelected({ kind: "account" });
                    await loadSkins();
                  }}
                >
                  <Icon name="trash" size={18} />
                </button>
              </div>
            </>
          ) : (
            <>
              <strong>{t("skins.current")}</strong>
              {licensed && skinData && (
                <button
                  className="btn"
                  onClick={async () => {
                    try {
                      const s = await api.saveSkinData(null, active?.name ?? "Skin", slim ? "slim" : "classic", skinData);
                      await loadSkins();
                      setSelected({ kind: "saved", skin: s });
                    } catch (e) {
                      toast(errText(e), "error");
                    }
                  }}
                >
                  {t("skins.fromAccount")}
                </button>
              )}
            </>
          )}
          {!licensed && <p className="notice small">{t("skins.needLicense")}</p>}
        </section>

        <div className="skins-right">
          <section className="panel">
            <h2>{t("skins.collection")}</h2>
            <p className="muted small">{t("skins.namemcHint")}</p>
            <div className="skin-grid">
              {licensed && (
                <button className={`skin-tile ${selected.kind === "account" ? "skin-tile--on" : ""}`} onClick={() => setSelected({ kind: "account" })} onMouseEnter={sfx.hover}>
                  {skinData && selected.kind === "account" ? <SkinThumb src={skinData} slim={slim} /> : <Icon name="user" size={32} />}
                  <span className="small ellipsis">{t("skins.current")}</span>
                </button>
              )}
              {skins.map((s) => (
                <button
                  key={s.id}
                  className={`skin-tile ${sel?.id === s.id ? "skin-tile--on" : ""}`}
                  onMouseEnter={sfx.hover}
                  onClick={() => {
                    sfx.click();
                    setSelected({ kind: "saved", skin: s });
                  }}
                >
                  <SkinThumb src={fileSrc(s.file) ?? ""} slim={s.variant === "slim"} />
                  <span className="small ellipsis">{s.name}</span>
                </button>
              ))}
            </div>
            {skins.length === 0 && <p className="muted">{t("skins.empty")}</p>}
          </section>

          {licensed && (
            <section className="panel">
              <h2>{t("skins.capes")}</h2>
              {profile && profile.capes.length === 0 && <p className="muted">{t("skins.noCapes")}</p>}
              {profile && profile.capes.length > 0 && (
                <div className="cape-grid">
                  <button
                    className={`cape-tile ${!profile.capes.some((c) => c.active) ? "cape-tile--on" : ""}`}
                    onClick={async () => {
                      try {
                        setProfile(await api.setCape(null));
                      } catch (e) {
                        toast(errText(e), "error");
                      }
                    }}
                  >
                    <span className="cape-thumb cape-thumb--none" />
                    <span className="small">{t("skins.noCape")}</span>
                  </button>
                  {profile.capes.map((c) => (
                    <button
                      key={c.id}
                      className={`cape-tile ${c.active ? "cape-tile--on" : ""}`}
                      onClick={async () => {
                        try {
                          setProfile(await api.setCape(c.id));
                          sfx.success();
                        } catch (e) {
                          toast(errText(e), "error");
                        }
                      }}
                    >
                      <CapeThumb url={c.url} />
                      <span className="small ellipsis">{c.alias}</span>
                    </button>
                  ))}
                </div>
              )}
            </section>
          )}
        </div>
      </div>

      {editing && (
        <SkinEditor
          skin={editing === "new" ? null : editing}
          onClose={() => setEditing(null)}
          onSaved={() => void loadSkins()}
        />
      )}
    </div>
  );
}
