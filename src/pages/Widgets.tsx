import { useEffect, useMemo, useState } from "react";
import { api, fileSrc, type Session, type Shot } from "../api";
import { useStore } from "../store";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";

const DAY = 86400;

function startOfDay(d: Date) {
  const x = new Date(d);
  x.setHours(0, 0, 0, 0);
  return Math.floor(x.getTime() / 1000);
}

/** Seconds played on each day starting at `from` (sessions crossing midnight are split). */
function perDay(sessions: Session[], from: number, days: number) {
  const out = new Array(days).fill(0);
  for (const s of sessions) {
    for (let i = 0; i < days; i++) {
      const a = from + i * DAY, b = a + DAY;
      const overlap = Math.min(b, s.end) - Math.max(a, s.start);
      if (overlap > 0) out[i] += overlap;
    }
  }
  return out;
}

function WeekWidget({ sessions }: { sessions: Session[] }) {
  const { t, settings } = useStore();
  const today = startOfDay(new Date());
  const from = today - 6 * DAY;
  const days = perDay(sessions, from, 7);
  const total = days.reduce((a, b) => a + b, 0);
  const max = Math.max(3600, ...days);
  const names = t("days.short").split(",");
  return (
    <div className="widget">
      <div className="widget__head">
        <strong>{t("w.week")}</strong>
        <span className="widget__big">{t("w.weekTotal", { h: Math.floor(total / 3600), m: Math.floor((total % 3600) / 60) })}</span>
      </div>
      {total === 0 && <span className="muted small">{t("w.noPlay")}</span>}
      <div className="week">
        {days.map((sec, i) => {
          const d = new Date((from + i * DAY) * 1000);
          const wd = (d.getDay() + 6) % 7;
          const label = `${Math.floor(sec / 3600)}:${String(Math.floor((sec % 3600) / 60)).padStart(2, "0")}`;
          return (
            <div key={i} className="week__col" title={`${d.toLocaleDateString(settings.lang === "ru" ? "ru-RU" : "en-GB")} · ${label}`}>
              <span className="week__bar" style={{ height: `${Math.max(3, (sec / max) * 100)}%`, opacity: sec ? 1 : 0.25 }} />
              <span className={`week__day ${i === 6 ? "week__day--today" : ""}`}>{names[wd]}</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function ShotsWidget() {
  const { t, go } = useStore();
  const [shots, setShots] = useState<Shot[] | null>(null);
  useEffect(() => {
    api.listScreenshots().then((s) => setShots(s.slice(0, 4))).catch(() => setShots([]));
  }, []);
  return (
    <div className="widget">
      <div className="widget__head">
        <strong>{t("w.shots")}</strong>
        <button className="link small" onClick={() => go("shots")}>{t("shots.title")} →</button>
      </div>
      {shots?.length === 0 && <span className="muted small">{t("w.noShots")}</span>}
      <div className="widget-shots">
        {shots?.map((s) => (
          <button key={s.path} className="widget-shots__item" onClick={() => go("shots")} onMouseEnter={sfx.hover} title={s.instanceName}>
            <img src={fileSrc(s.path) ?? ""} alt="" loading="lazy" />
          </button>
        ))}
      </div>
    </div>
  );
}

function UpdatesWidget() {
  const { t, feed, settings } = useStore();
  const [latest, setLatest] = useState<{ release: string; snapshot: string; releaseTime: string; snapshotTime: string } | null>(null);
  useEffect(() => {
    api.minecraftLatest().then(setLatest).catch(() => setLatest(null));
  }, []);
  const note = feed?.patchNotes.find((n) => n.image);
  const date = (s: string) => (s ? new Date(s).toLocaleDateString(settings.lang === "ru" ? "ru-RU" : "en-GB", { day: "numeric", month: "short" }) : "");
  const snapshotNewer = latest && latest.snapshot !== latest.release;
  return (
    <div className="widget">
      <div className="widget__head">
        <strong>{t("w.updates")}</strong>
      </div>
      {latest && (
        <div className="versions">
          <span className="ver ver--release">
            <small>{t("w.release")}</small>
            <b>{latest.release}</b>
            <small>{date(latest.releaseTime)}</small>
          </span>
          {snapshotNewer && (
            <span className="ver ver--snapshot">
              <small>{t("w.snapshot")}</small>
              <b>{latest.snapshot}</b>
              <small>{date(latest.snapshotTime)}</small>
            </span>
          )}
        </div>
      )}
      {note && (
        <div className="widget-note">
          {note.image && <img src={note.image} alt="" loading="lazy" />}
          <span className="small">
            <span className="muted">{t("w.latestNotes")}: </span>
            {note.title}
          </span>
        </div>
      )}
    </div>
  );
}

function CalendarWidget({ sessions }: { sessions: Session[] }) {
  const { t, settings } = useStore();
  const weeks = 26;
  const today = new Date();
  const todayStart = startOfDay(today);
  const wd = (today.getDay() + 6) % 7;
  const from = todayStart - (weeks * 7 - 1 - (6 - wd)) * DAY;
  const days = useMemo(() => perDay(sessions, from, weeks * 7), [sessions, from]);
  const played = days.filter((d) => d > 60).length;
  const total = Math.round(days.reduce((a, b) => a + b, 0) / 3600);
  const level = (s: number) => (s <= 60 ? 0 : s < 1800 ? 1 : s < 3600 ? 2 : s < 7200 ? 3 : 4);
  return (
    <div className="widget widget--wide">
      <div className="widget__head">
        <strong>{t("w.calendar")}</strong>
        <span className="muted small">{t("w.calendarHint", { days: played, h: total })}</span>
      </div>
      <div className="calendar" role="img" aria-label={t("w.calendarHint", { days: played, h: total })}>
        {days.map((s, i) => {
          const ts = from + i * DAY;
          const future = ts > todayStart;
          const d = new Date(ts * 1000);
          return (
            <span
              key={i}
              className={`calendar__cell calendar__cell--${future ? "future" : level(s)}`}
              title={`${d.toLocaleDateString(settings.lang === "ru" ? "ru-RU" : "en-GB")} · ${Math.round(s / 60)} min`}
            />
          );
        })}
      </div>
      <div className="calendar__legend small muted">
        {t("w.less")}
        {[0, 1, 2, 3, 4].map((l) => (
          <span key={l} className={`calendar__cell calendar__cell--${l}`} />
        ))}
        {t("w.more")}
      </div>
    </div>
  );
}

export function Widgets() {
  const { settings, go, t, running } = useStore();
  const [sessions, setSessions] = useState<Session[]>([]);
  useEffect(() => {
    api
      .playSessions(Math.floor(Date.now() / 1000) - 190 * DAY)
      .then(setSessions)
      .catch(() => setSessions([]));
  }, [running.length]);
  const w = settings.widgets;
  if (!w.week && !w.shots && !w.updates && !w.calendar) return null;
  return (
    <section>
      <div className="section-head">
        <h2>{settings.lang === "ru" ? "Твоя игра" : "Your play"}</h2>
        <button className="link" onClick={() => go("settings")}>
          <Icon name="settings" size={14} /> {t("w.customize")}
        </button>
      </div>
      <div className="widgets">
        {w.week && <WeekWidget sessions={sessions} />}
        {w.shots && <ShotsWidget />}
        {w.updates && <UpdatesWidget />}
        {w.calendar && <CalendarWidget sessions={sessions} />}
      </div>
    </section>
  );
}
