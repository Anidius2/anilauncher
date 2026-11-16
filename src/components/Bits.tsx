import { useEffect, type ReactNode } from "react";
import { fileSrc } from "../api";
import { frames } from "../themes";
import { sfx } from "../sounds";
import { Icon } from "./Icon";

/** Profile picture inside a decorative frame. */
export function Avatar({ path, name, frame, size = 52 }: { path?: string | null; name: string; frame: string; size?: number }) {
  const f = frames.find((x) => x.id === frame) ?? frames[0];
  const src = fileSrc(path);
  const pad = f.id === "none" ? 0 : Math.max(2, Math.round(size / 17));
  return (
    <span
      className={`avatar ${f.animated ? "avatar--spin" : ""} ${f.fx ? `avatar--fx avatar--${f.fx}` : ""}`}
      style={{ width: size, height: size, padding: pad, background: f.css, borderRadius: size * 0.27 }}
    >
      {f.fx && (
        <span className="avatar__fx" aria-hidden="true">
          {Array.from({ length: 8 }, (_, i) => (
            <i key={i} style={{ left: `${8 + i * 11}%`, animationDelay: `${(i * 0.37) % 2}s` }} />
          ))}
        </span>
      )}
      <span className="avatar__inner" style={{ borderRadius: size * 0.27 - pad }}>
        {src ? (
          <img src={src} alt="" />
        ) : (
          <span className="avatar__letter" style={{ fontSize: size * 0.46 }}>
            {(name || "A").slice(0, 1).toUpperCase()}
          </span>
        )}
      </span>
    </span>
  );
}

export function Modal({ title, onClose, children, wide, xl }: { title: string; onClose: () => void; children: ReactNode; wide?: boolean; xl?: boolean }) {
  useEffect(() => {
    sfx.open();
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="modal-back" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal ${wide ? "modal--wide" : ""} ${xl ? "modal--xl" : ""}`} role="dialog" aria-modal="true" aria-label={title}>
        <div className="modal__head">
          <h2>{title}</h2>
          <button className="icon-btn" aria-label="Закрыть" onClick={onClose}>
            <Icon name="close" />
          </button>
        </div>
        <div className="modal__body">{children}</div>
      </div>
    </div>
  );
}

/** Default pixel landscape used when a build has no art yet. */
export function DefaultArt() {
  return (
    <svg className="art-fill" viewBox="0 0 1200 400" preserveAspectRatio="xMidYMid slice" aria-hidden="true">
      <defs>
        <linearGradient id="defSky" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#2b1b4e" />
          <stop offset=".55" stopColor="#b4467a" />
          <stop offset="1" stopColor="#f59e5b" />
        </linearGradient>
      </defs>
      <rect width="1200" height="400" fill="url(#defSky)" />
      <rect x="760" y="150" width="96" height="96" fill="#fde68a" />
      <rect x="772" y="162" width="72" height="72" fill="#fff7d1" />
      <rect className="cloud" x="120" y="70" width="140" height="18" fill="#f5c6d8" opacity=".55" />
      <rect className="cloud" x="150" y="56" width="80" height="14" fill="#f5c6d8" opacity=".55" />
      <rect className="cloud cloud--slow" x="930" y="96" width="180" height="16" fill="#f5c6d8" opacity=".45" />
      <polygon points="0,250 80,250 80,220 180,220 180,190 260,190 260,230 380,230 380,200 460,200 460,240 600,240 600,210 700,210 700,250 860,250 860,215 980,215 980,190 1080,190 1080,235 1200,235 1200,400 0,400" fill="#5b2a5e" />
      <polygon points="0,300 120,300 120,275 260,275 260,295 420,295 420,262 540,262 540,288 700,288 700,270 820,270 820,300 960,300 960,278 1100,278 1100,296 1200,296 1200,400 0,400" fill="#3a1d47" />
      <polygon points="0,340 160,340 160,325 340,325 340,345 520,345 520,322 680,322 680,342 880,342 880,328 1040,328 1040,348 1200,348 1200,400 0,400" fill="#221430" />
    </svg>
  );
}

export function Toggle({ on, onChange, label }: { on: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      className={`toggle ${on ? "toggle--on" : ""}`}
      role="switch"
      aria-checked={on}
      aria-label={label}
      onClick={() => {
        sfx.click();
        onChange(!on);
      }}
    >
      <span className="toggle__knob" />
    </button>
  );
}

export function formatPlaytime(sec: number, lang: "ru" | "en"): string {
  const h = Math.floor(sec / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (lang === "ru") return h > 0 ? `${h} ч ${m} мин` : `${m} мин`;
  return h > 0 ? `${h} h ${m} min` : `${m} min`;
}

export function loaderLabel(loader: string): string {
  const names: Record<string, string> = { fabric: "Fabric", quilt: "Quilt", forge: "Forge", neoforge: "NeoForge" };
  return names[loader] ?? "Vanilla";
}
