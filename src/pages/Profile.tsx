import { useEffect, useRef, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, errText, type PublicAccount } from "../api";
import { useStore } from "../store";
import { Avatar } from "../components/Bits";
import { Icon } from "../components/Icon";
import { frames } from "../themes";
import { sfx } from "../sounds";
import { PlayerCard } from "./PlayerCard";

const NICK = /^[A-Za-z0-9_]{3,16}$/;

function NameForm({ account, onDone }: { account: PublicAccount; onDone: () => void }) {
  const { t, setAccounts, toast } = useStore();
  const [name, setName] = useState("");
  const [state, setState] = useState<"idle" | "free" | "taken">("idle");
  const [busy, setBusy] = useState(false);
  const valid = NICK.test(name);
  return (
    <div className="name-form">
      <strong>{account.needsName ? t("acc.needName") : t("acc.changeName")}</strong>
      <p className="muted small">{t("acc.nameHint")}</p>
      <div className="inline-form">
        <input
          value={name}
          maxLength={16}
          placeholder={t("acc.nickPh")}
          onChange={(e) => {
            setName(e.target.value);
            setState("idle");
          }}
          aria-label={t("acc.nickPh")}
        />
        <button
          className="btn"
          disabled={!valid || busy}
          onClick={async () => {
            setBusy(true);
            try {
              setState((await api.nameAvailable(account.id, name)) ? "free" : "taken");
            } catch (e) {
              toast(errText(e), "error");
            } finally {
              setBusy(false);
            }
          }}
        >
          {t("acc.check")}
        </button>
        <button
          className="btn btn--primary"
          disabled={!valid || busy || state === "taken"}
          onClick={async () => {
            setBusy(true);
            try {
              setAccounts(await api.setMinecraftName(account.id, name));
              sfx.success();
              onDone();
            } catch (e) {
              toast(errText(e), "error");
            } finally {
              setBusy(false);
            }
          }}
        >
          {t("acc.saveName")}
        </button>
      </div>
      {state !== "idle" && <span className={`pill ${state === "free" ? "pill--ok" : "pill--bad"}`}>{state === "free" ? t("acc.free") : t("acc.taken")}</span>}
    </div>
  );
}

function LoginCard() {
  const { t, setAccounts, toast } = useStore();
  const [waiting, setWaiting] = useState(false);
  const alive = useRef(true);
  useEffect(() => () => void (alive.current = false), []);

  const start = async () => {
    setWaiting(true);
    sfx.open();
    try {
      const view = await api.loginWindow();
      if (!alive.current) return;
      setAccounts(view);
      sfx.success();
      toast(t("toast.loggedIn"), "ok");
    } catch (e) {
      const msg = errText(e);
      if (msg !== "Вход отменён.") toast(msg, "error");
    } finally {
      if (alive.current) setWaiting(false);
    }
  };

  return (
    <button className="btn btn--primary btn--big" onClick={start} disabled={waiting}>
      <svg width="18" height="18" viewBox="0 0 23 23" aria-hidden="true">
        <rect x="1" y="1" width="10" height="10" fill="#f25022" />
        <rect x="12" y="1" width="10" height="10" fill="#7fba00" />
        <rect x="1" y="12" width="10" height="10" fill="#00a4ef" />
        <rect x="12" y="12" width="10" height="10" fill="#ffb900" />
      </svg>
      {waiting ? t("acc.inWindow") : t("acc.signIn")}
    </button>
  );
}

export function Profile() {
  const { t, settings, updateSettings, accounts, setAccounts, toast } = useStore();
  const [offlineName, setOfflineName] = useState("");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [card, setCard] = useState(false);
  const p = settings.profile;
  const setProfile = (patch: Partial<typeof p>) => updateSettings({ profile: { ...p, ...patch } });

  const pickAvatar = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp", "gif"] }] });
      if (typeof file !== "string") return;
      setProfile({ avatar: await api.importMedia(file) });
      sfx.success();
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const act = async (fn: () => Promise<typeof accounts>) => {
    try {
      setAccounts(await fn());
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  return (
    <div className="page">
      <div className="page-head">
        <h1>{t("acc.title")}</h1>
        <button className="btn btn--primary" onClick={() => setCard(true)}>
          <Icon name="image" size={16} /> {t("card.open")}
        </button>
      </div>
      {card && <PlayerCard onClose={() => setCard(false)} />}

      <section className="panel">
        <h2>{t("acc.accounts")}</h2>
        <div className="account-list">
          {accounts.accounts.map((a) => (
            <div key={a.id} className={`account ${a.active ? "account--active" : ""}`}>
              <span className="account__head">
                {a.skinUrl ? (
                  <span className="mc-head" aria-hidden="true">
                    <span style={{ backgroundImage: `url("${a.skinUrl}")` }} />
                    <span className="mc-head__hat" style={{ backgroundImage: `url("${a.skinUrl}")` }} />
                  </span>
                ) : (
                  <Icon name="user" size={24} />
                )}
              </span>
              <span className="account__text">
                <strong>{settings.streamer ? t("yt.hidden") : a.name}</strong>
                <span className="muted small">
                  {a.kind === "msa" ? t("acc.licensed") : t("acc.offline")}
                  {a.active ? ` · ${t("acc.active")}` : ""}
                </span>
              </span>
              <span className="account__actions">
                {!a.active && (
                  <button className="btn btn--small" onClick={() => act(() => api.setActiveAccount(a.id))}>
                    {t("acc.use")}
                  </button>
                )}
                {a.kind === "msa" && !a.needsName && (
                  <button className="btn btn--small" onClick={() => setRenaming(renaming === a.id ? null : a.id)}>
                    {t("acc.changeName")}
                  </button>
                )}
                <button className="btn btn--small btn--danger" onClick={() => act(() => api.removeAccount(a.id))}>
                  {t("acc.remove")}
                </button>
              </span>
              {(a.needsName || renaming === a.id) && <NameForm account={a} onDone={() => setRenaming(null)} />}
            </div>
          ))}
        </div>
        <LoginCard />
      </section>

      <section className="panel">
        <h2>{t("acc.local")}</h2>
        <p className="muted small">{t("acc.localHint")}</p>
        <div className="profile-edit">
          <Avatar path={p.avatar} name={p.nick || "A"} frame={p.frame} size={112} />
          <div className="profile-edit__fields">
            <label className="field">
              <span>{t("acc.launcherNick")}</span>
              <input value={p.nick} maxLength={24} placeholder="Anidius" onChange={(e) => setProfile({ nick: e.target.value })} />
            </label>
            <div className="field">
              <span>{t("acc.avatar")}</span>
              <div className="inline-form">
                <button className="btn" onClick={pickAvatar}>
                  <Icon name="image" size={16} /> {t("acc.avatarPick")}
                </button>
                {p.avatar && (
                  <button className="btn" onClick={() => setProfile({ avatar: null })}>
                    {t("acc.avatarReset")}
                  </button>
                )}
              </div>
            </div>
          </div>
        </div>
        <div className="field">
          <span>{t("acc.frame")}</span>
          <div className="frame-grid">
            {frames.map((f) => (
              <button
                key={f.id}
                className={`frame-pick ${p.frame === f.id ? "frame-pick--on" : ""}`}
                onClick={() => {
                  sfx.click();
                  setProfile({ frame: f.id });
                }}
                onMouseEnter={sfx.hover}
              >
                <Avatar path={p.avatar} name={p.nick || "A"} frame={f.id} size={56} />
                <span className="small">{f.name[settings.lang]}</span>
              </button>
            ))}
          </div>
        </div>
      </section>

      <section className="panel">
        <h2>{t("acc.offlineTitle")}</h2>
        <p className="muted small">{t("acc.offlineHint")}</p>
        {accounts.hasLicense ? (
          <div className="inline-form">
            <input value={offlineName} maxLength={16} placeholder={t("acc.nickPh")} onChange={(e) => setOfflineName(e.target.value)} aria-label={t("acc.nickPh")} />
            <button
              className="btn btn--primary"
              disabled={!NICK.test(offlineName)}
              onClick={async () => {
                await act(() => api.addOffline(offlineName));
                setOfflineName("");
              }}
            >
              {t("acc.add")}
            </button>
          </div>
        ) : (
          <p className="notice">{t("acc.offlineLocked")}</p>
        )}
      </section>
    </div>
  );
}
