import { Fragment, useEffect, useRef, useState } from "react";
import { api, artSrc, errText, type Feed, type Instance } from "../api";
import { useStore } from "../store";
import { GameRemote } from "./GameRemote";
import { DefaultArt, loaderLabel } from "../components/Bits";
import { Icon } from "../components/Icon";
import { BuildDialog } from "./BuildDialog";
import { ArtPicker } from "./ArtPicker";
import { sfx, startAmbient, stopAmbient } from "../sounds";
import { LiveScene, autoWeather } from "../components/LiveScene";
import { Widgets } from "./Widgets";
import { QuickStart } from "./Extras";
import { HeroDecor, ageToday } from "../components/Seasonal";
import { fileSrc, type Shot } from "../api";
import type { Key } from "../i18n";

function BuildCard({ inst, onPick, active }: { inst: Instance; onPick: () => void; active: boolean }) {
  const { play, running, preparing, feed } = useStore();
  const src = artSrc(inst.art) ?? mojangArt(feed, inst.id);
  const busy = running.includes(inst.id) || !!preparing[inst.id];
  return (
    <div className={`card build-card ${active ? "card--active" : ""}`}>
      <button className="build-card__main" onClick={onPick} onMouseEnter={sfx.hover}>
        <span className="build-card__art">{src ? <img className="art-fill" src={src} alt="" loading="lazy" /> : <DefaultArt />}</span>
        <span className="build-card__text">
          <span className="build-card__name">{inst.name}</span>
          <span className="build-card__meta">
            {loaderLabel(inst.loader)} · {inst.mcVersion}
          </span>
        </span>
      </button>
      <button className="card-play" aria-label="Play" disabled={busy} onClick={() => void play(inst.id)}>
        <Icon name="play" size={16} />
      </button>
    </div>
  );
}

function PlayButton() {
  const { t, selected, settings, accounts, running, preparing, play, go, toast, linked } = useStore();
  const [creating, setCreating] = useState(false);
  const [remote, setRemote] = useState(false);
  const playText = settings.ui.playLabel.trim() || t("home.play");
  if (settings.edition === "bedrock") {
    return (
      <button
        className="play"
        onClick={async () => {
          sfx.play();
          try {
            await api.launchBedrock();
          } catch (e) {
            toast(errText(e), "error");
          }
        }}
      >
        {playText}
      </button>
    );
  }
  if (!accounts.accounts.length) {
    return (
      <button className="play play--setup" onClick={() => go("profile")}>
        {t("home.playSignIn")}
      </button>
    );
  }
  if (!selected) {
    return (
      <>
        <button className="play play--setup" onClick={() => setCreating(true)}>
          {t("home.playCreate")}
        </button>
        {creating && <BuildDialog onClose={() => setCreating(false)} />}
      </>
    );
  }
  const prog = preparing[selected.id];
  if (prog) {
    const pct = prog.total > 0 ? Math.round((prog.done / prog.total) * 100) : 0;
    return (
      <div className="play play--busy" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
        <span className="play__fill" style={{ width: `${pct}%` }} />
        <span className="play__label">
          {t(`stage.${prog.stage}` as Key)} · {pct}%
        </span>
      </div>
    );
  }
  if (running.includes(selected.id)) {
    return (
      <div className="play-group">
        <span className="play play--running">{t("home.playing")}</span>
        {selected.id in linked && (
          <button className="icon-btn icon-btn--big" aria-label={settings.lang === "ru" ? "Пульт игры" : "Game remote"} title={settings.lang === "ru" ? "Пульт игры" : "Game remote"} onClick={() => setRemote(true)}>
            <Icon name="sparkle" />
          </button>
        )}
        {remote && <GameRemote inst={selected} onClose={() => setRemote(false)} />}
        <button
          className="icon-btn icon-btn--big"
          aria-label={t("home.stop")}
          title={t("home.stop")}
          onClick={() => api.stopInstance(selected.id).catch((e) => toast(errText(e), "error"))}
        >
          <Icon name="stop" />
        </button>
      </div>
    );
  }
  return (
    <button className="play" onClick={() => void play(selected.id)}>
      {playText}
    </button>
  );
}

/** Shown until the player has an account and a build: what to do, in order. */
function GettingStarted({ onCreate }: { onCreate: () => void }) {
  const { t, accounts, instances, go } = useStore();
  const signed = accounts.accounts.length > 0;
  const built = instances.length > 0;
  if (signed && built) return null;
  const steps = [
    { done: signed, title: t("start.s1"), text: t("start.s1t"), action: t("start.s1b"), run: () => go("profile") },
    { done: built, title: t("start.s2"), text: t("start.s2t"), action: t("start.s2b"), run: onCreate },
    { done: false, title: t("start.s3"), text: t("start.s3t"), action: null, run: null },
  ];
  const current = steps.findIndex((x) => !x.done);
  return (
    <section className="start">
      <h2>{t("start.title")}</h2>
      <ol className="start__steps">
        {steps.map((x, i) => (
          <li key={i} className={`start__step ${x.done ? "start__step--done" : ""} ${i === current ? "start__step--now" : ""}`}>
            <span className="start__num">{x.done ? <Icon name="check" size={16} /> : i + 1}</span>
            <span className="start__text">
              <strong>{x.title}</strong>
              <span className="muted small">{x.text}</span>
            </span>
            {x.action && !x.done && (
              <button className={`btn ${i === current ? "btn--primary" : ""}`} disabled={i !== current} onClick={x.run ?? undefined}>
                {x.action}
              </button>
            )}
          </li>
        ))}
      </ol>
    </section>
  );
}

function BuildSelect() {
  const { t, instances, selected, select, feed } = useStore();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const close = (e: MouseEvent) => ref.current && !ref.current.contains(e.target as Node) && setOpen(false);
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, []);
  const src = selected ? artSrc(selected.art) ?? mojangArt(feed, selected.id) : null;
  return (
    <div className="build-select" ref={ref}>
      <button
        className="build-select__btn"
        aria-expanded={open}
        disabled={!instances.length}
        onClick={() => {
          sfx.click();
          setOpen((o) => !o);
        }}
      >
        <span className="build-select__thumb">{src ? <img className="art-fill" src={src} alt="" /> : <DefaultArt />}</span>
        <span className="build-select__text">
          <span className="muted small">{t("home.build")}</span>
          <span className="ellipsis strong">{selected?.name ?? t("home.noBuild")}</span>
        </span>
        <Icon name="chevron" size={16} />
      </button>
      {open && (
        <div className="build-select__menu" role="listbox">
          {instances.map((i) => (
            <button
              key={i.id}
              role="option"
              aria-selected={i.id === selected?.id}
              className={`build-select__item ${i.id === selected?.id ? "is-on" : ""}`}
              onClick={() => {
                sfx.click();
                select(i.id);
                setOpen(false);
              }}
            >
              <span className="ellipsis strong">{i.name}</span>
              <span className="muted small">
                {loaderLabel(i.loader)} · {i.mcVersion}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/** Slideshow of the player's own screenshots with a slow zoom. */
function ShotSlideshow() {
  const [shots, setShots] = useState<Shot[]>([]);
  const [i, setI] = useState(0);
  useEffect(() => {
    api.listScreenshots().then((s) => setShots(s.slice(0, 20))).catch(() => setShots([]));
  }, []);
  useEffect(() => {
    if (shots.length < 2) return;
    const id = setInterval(() => setI((x) => (x + 1) % shots.length), 9000);
    return () => clearInterval(id);
  }, [shots.length]);
  if (!shots.length) return <LiveScene />;
  return (
    <>
      {shots.map((s, k) => (
        <img key={s.path} className={`art-fill slide ${k === i ? "slide--on" : ""}`} src={fileSrc(s.path) ?? ""} alt="" />
      ))}
    </>
  );
}

/** A Mojang art for screens without their own picture; the same build always gets the same one. */
export function mojangArt(feed: Feed | null, seed: string): string | null {
  const pics = [...(feed?.patchNotes ?? []), ...(feed?.news ?? [])].map((n) => n.image).filter((x): x is string => !!x);
  if (!pics.length) return null;
  let h = 0;
  for (const c of seed) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return pics[h % pics.length];
}

function HeroBackground({ buildArt }: { buildArt: string | null }) {
  const { settings, feed, selected } = useStore();
  const bg = settings.background;
  if (bg.mode === "live") return <LiveScene weather={bg.weather} animate={settings.animations} />;
  if (bg.mode === "shots") return <ShotSlideshow />;
  if (bg.mode === "custom" && bg.file) {
    const src = fileSrc(bg.file) ?? "";
    return /\.(mp4|webm)$/i.test(bg.file) ? (
      <video className="art-fill" src={src} autoPlay loop muted playsInline />
    ) : (
      <img className="art-fill" src={src} alt="" />
    );
  }
  // No own art (or no build yet): a Mojang art from the feed; the drawn landscape only when offline
  const src = buildArt ?? mojangArt(feed, selected?.id ?? "");
  return src ? <img key={src} className="art-fill hero__img" src={src} alt="" /> : <DefaultArt />;
}

export function Home() {
  const { t, settings, updateSettings, selected, instances, select, accounts, feed, go, reloadInstances, toast, season } = useStore();
  const [creating, setCreating] = useState(false);
  const [pickingArt, setPickingArt] = useState(false);
  const bedrock = settings.edition === "bedrock";
  const active = accounts.accounts.find((a) => a.active);
  const heroSrc = !bedrock ? artSrc(selected?.art) : feed?.news.find((n) => n.image)?.image ?? null;
  const bg = settings.background;

  // Rain and wind sounds go with the living landscape
  useEffect(() => {
    const wx = bg.weather === "auto" ? autoWeather() : bg.weather;
    if (bg.mode === "live" && settings.ambient && settings.sounds) startAmbient(wx === "rain" ? "rain" : wx === "snow" ? "wind" : null);
    else stopAmbient();
    return () => stopAmbient();
  }, [bg.mode, bg.weather, settings.ambient, settings.sounds]);

  return (
    <div className="page page--home">
      <section className="hero">
        {bedrock ? heroSrc ? <img className="art-fill hero__img" src={heroSrc} alt="" /> : <DefaultArt /> : <HeroBackground buildArt={heroSrc} />}
        <div className="hero__shade" />
        <HeroDecor
          event={settings.events.decor ? season : null}
          name={settings.profile.nick || active?.name || "Anidius"}
          age={settings.birthday ? ageToday(settings.birthday) : null}
          lang={settings.lang}
        />
        <div className="seg hero__seg" role="tablist">
          {(["java", "bedrock"] as const).map((ed) => (
            <button
              key={ed}
              role="tab"
              aria-selected={settings.edition === ed}
              className={`seg__btn ${settings.edition === ed ? "seg__btn--on" : ""}`}
              onClick={() => {
                sfx.click();
                updateSettings({ edition: ed });
              }}
            >
              {ed === "java" ? "Java Edition" : "Bedrock"}
            </button>
          ))}
        </div>
        {!bedrock && selected && bg.mode === "art" && (
          <button className="ghost hero__art-btn" onClick={() => setPickingArt(true)}>
            <Icon name="image" size={16} /> {t("home.changeArt")}
          </button>
        )}
        <div className="hero__text">
          {bedrock ? (
            <>
              <span className="hero__kicker">Microsoft Store</span>
              <h1 className="hero__title">{t("home.bedrockTitle")}</h1>
              <span className="hero__meta">{t("home.bedrockText")}</span>
            </>
          ) : selected ? (
            <>
              <span className="hero__kicker">{bg.mode === "art" ? (selected.art?.kind === "file" ? t("home.artOwn") : t("home.artMojang")) : t(`bg.mode.${bg.mode}` as Key)}</span>
              <h1 className="hero__title">{selected.name}</h1>
              <span className="hero__meta">
                {loaderLabel(selected.loader)} · {selected.mcVersion}
              </span>
            </>
          ) : (
            <>
              <h1 className="hero__title">AniLauncher</h1>
              <span className="hero__meta">{accounts.accounts.length ? t("home.welcome") : t("home.signIn")}</span>
            </>
          )}
        </div>
      </section>

      <div className="playbar">
        {!bedrock ? <BuildSelect /> : <span className="playbar__side" />}
        <div className="playbar__center">
          <PlayButton />
        </div>
        <div className="playbar__side playbar__nick">
          <span className="muted small">{t("home.nick")}</span>
          <span className="strong">{active ? (settings.streamer ? "Player" : active.name) : "—"}</span>
        </div>
      </div>

      <div className="home-sections">
        {!bedrock && <GettingStarted onCreate={() => setCreating(true)} />}
        {settings.streamer && settings.streamerNote.trim() && (
          <div className="yt-note">
            <span className="yt-note__dot" />
            <span className="muted small">{t("yt.note")}:</span>
            <strong>{settings.streamerNote}</strong>
          </div>
        )}
        {settings.ui.homeOrder.filter((b) => !settings.ui.homeHidden.includes(b)).map((b) => (
          <Fragment key={b}>
            {b === "quick" && <QuickStart />}
            {b === "builds" && (
              <section>
          <div className="section-head">
            <h2>{t("home.myBuilds")}</h2>
            <button className="link" onClick={() => go("builds")}>
              {t("home.allBuilds")}
            </button>
          </div>
          <div className="grid grid--4">
            {instances.slice(0, settings.ui.homeBuilds).map((i) => (
              <BuildCard key={i.id} inst={i} active={i.id === selected?.id} onPick={() => select(i.id)} />
            ))}
            <button className="card card--new" onClick={() => setCreating(true)} onMouseEnter={sfx.hover}>
              <Icon name="plus" size={28} />
              {t("home.newBuild")}
            </button>
          </div>
        </section>
            )}
            {b === "widgets" && <Widgets />}
            {b === "news" && (
              <section>
          <div className="section-head">
            <h2>{t("home.news")}</h2>
            <button className="link" onClick={() => go("news")}>
              {t("home.allNews")}
            </button>
          </div>
          <div className="grid grid--3">
            {(feed?.news ?? []).slice(0, 3).map((n) => (
              <button
                key={n.id}
                className="card news-card"
                onMouseEnter={sfx.hover}
                onClick={() => n.link && api.openUrl(n.link).catch((e) => toast(errText(e), "error"))}
              >
                <span className="news-card__img">{n.image && <img className="art-fill" src={n.image} alt="" loading="lazy" />}</span>
                <span className="news-card__text">
                  <span className="news-card__tag">{n.tag}</span>
                  <span className="news-card__title">{n.title}</span>
                </span>
              </button>
            ))}
          </div>
        </section>
            )}
          </Fragment>
        ))}
      </div>

      {creating && <BuildDialog onClose={() => setCreating(false)} />}
      {pickingArt && selected && (
        <ArtPicker
          onClose={() => setPickingArt(false)}
          onPick={async (art) => {
            setPickingArt(false);
            try {
              await api.updateInstance({ ...selected, art });
              await reloadInstances();
            } catch (e) {
              toast(errText(e), "error");
            }
          }}
        />
      )}
    </div>
  );
}
