import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { api, errText, type Session } from "../api";
import { useStore } from "../store";
import { Modal } from "../components/Bits";
import { Icon } from "../components/Icon";
import { drawSkinFront } from "../components/SkinView";
import { sfx } from "../sounds";

const W = 1000, H = 560, DAY = 86400;

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((res, rej) => {
    const i = new Image();
    i.onload = () => res(i);
    i.onerror = rej;
    i.src = src;
  });
}

/** A shareable picture: skin, nickname, playtime, favorite build and activity. */
export function PlayerCard({ onClose }: { onClose: () => void }) {
  const { t, settings, accounts, instances, toast } = useStore();
  const ref = useRef<HTMLCanvasElement>(null);
  const [ready, setReady] = useState(false);
  const active = accounts.accounts.find((a) => a.active);
  const nick = settings.streamer ? "Player" : settings.profile.nick || active?.name || "Player";

  useEffect(() => {
    (async () => {
      const c = ref.current;
      if (!c) return;
      const g = c.getContext("2d")!;
      await document.fonts.ready;
      const css = getComputedStyle(document.documentElement);
      const v = (k: string) => css.getPropertyValue(k).trim() || "#888";
      const heading = css.getPropertyValue("--font-heading").trim() || "sans-serif";
      // background
      const bg = g.createLinearGradient(0, 0, W, H);
      bg.addColorStop(0, v("--side"));
      bg.addColorStop(1, v("--bg"));
      g.fillStyle = bg;
      g.fillRect(0, 0, W, H);
      const glow = g.createRadialGradient(220, 300, 10, 220, 300, 320);
      glow.addColorStop(0, v("--accent-hi") + "55");
      glow.addColorStop(1, "transparent");
      g.fillStyle = glow;
      g.fillRect(0, 0, W, H);
      // pixel ground under the player
      g.fillStyle = v("--accent");
      for (let x = 60; x < 380; x += 20) g.fillRect(x, 500 - ((x / 20) % 2) * 6, 20, 60);
      // skin (or avatar)
      try {
        let drawn = false;
        if (active?.skinUrl && !settings.streamer) {
          const img = await loadImage(await api.textureData(active.skinUrl));
          drawSkinFront(g, img, 140, 150, 10);
          drawn = true;
        }
        if (!drawn && settings.profile.avatar) {
          const img = await loadImage(await api.textureData(settings.profile.avatar));
          g.save();
          g.beginPath();
          g.roundRect(100, 170, 240, 240, 48);
          g.clip();
          g.drawImage(img, 100, 170, 240, 240);
          g.restore();
          drawn = true;
        }
        if (!drawn) {
          g.fillStyle = v("--accent");
          g.beginPath();
          g.roundRect(100, 170, 240, 240, 48);
          g.fill();
          g.fillStyle = v("--on-accent");
          g.font = `700 140px ${heading}`;
          g.textAlign = "center";
          g.fillText(nick.slice(0, 1).toUpperCase(), 220, 340);
          g.textAlign = "left";
        }
      } catch {
        /* card still works without the skin */
      }
      // text
      g.fillStyle = v("--muted");
      g.font = `600 20px Inter, sans-serif`;
      g.fillText("AniLauncher", 440, 92);
      g.fillStyle = v("--text");
      g.font = `700 64px ${heading}`;
      g.fillText(nick, 440, 160);
      const total = instances.reduce((a, i) => a + i.playSeconds, 0);
      const fav = [...instances].sort((a, b) => b.playSeconds - a.playSeconds)[0];
      const stats: Array<[string, string]> = [
        [String(Math.round(total / 3600)), t("card.hours")],
        [String(instances.length), t("card.builds")],
      ];
      let sessions: Session[] = [];
      try {
        sessions = await api.playSessions(Math.floor(Date.now() / 1000) - 120 * DAY);
      } catch {
        /* no stats yet */
      }
      stats.forEach(([n, label], i) => {
        const x = 440 + i * 200;
        g.fillStyle = v("--panel");
        g.beginPath();
        g.roundRect(x, 196, 180, 104, 16);
        g.fill();
        g.fillStyle = v("--accent-hi");
        g.font = `700 44px ${heading}`;
        g.fillText(n, x + 20, 252);
        g.fillStyle = v("--muted");
        g.font = `600 17px Inter, sans-serif`;
        g.fillText(label, x + 20, 282);
      });
      if (fav && fav.playSeconds > 0) {
        g.fillStyle = v("--muted");
        g.font = `600 17px Inter, sans-serif`;
        g.fillText(t("card.favorite"), 440, 340);
        g.fillStyle = v("--text");
        g.font = `700 28px Inter, sans-serif`;
        g.fillText(`${fav.name}  ·  ${Math.round(fav.playSeconds / 3600)} h`, 440, 374);
      }
      // 16-week activity grid
      g.fillStyle = v("--muted");
      g.font = `600 17px Inter, sans-serif`;
      g.fillText(t("card.activity"), 440, 420);
      const today = new Date();
      today.setHours(0, 0, 0, 0);
      const t0 = Math.floor(today.getTime() / 1000);
      const wd = (today.getDay() + 6) % 7;
      const from = t0 - (16 * 7 - 1 - (6 - wd)) * DAY;
      for (let d = 0; d < 16 * 7; d++) {
        const a = from + d * DAY;
        let sec = 0;
        for (const s of sessions) sec += Math.max(0, Math.min(a + DAY, s.end) - Math.max(a, s.start));
        const lvl = sec <= 60 ? 0 : sec < 1800 ? 0.35 : sec < 3600 ? 0.6 : sec < 7200 ? 0.85 : 1;
        const col = Math.floor(d / 7), row = d % 7;
        g.globalAlpha = a > t0 ? 0.25 : 1;
        g.fillStyle = lvl === 0 ? v("--panel") : v("--accent-hi");
        if (lvl > 0) g.globalAlpha = lvl;
        g.beginPath();
        g.roundRect(440 + col * 30, 436 + row * 15, 24, 11, 3);
        g.fill();
        g.globalAlpha = 1;
      }
      const stamps = instances.map((i) => i.created).filter((c) => c > 1_000_000_000);
      if (stamps.length) {
        const since = Math.min(...stamps);
        g.fillStyle = v("--muted");
        g.font = `500 15px Inter, sans-serif`;
        g.textAlign = "right";
        g.fillText(t("card.since", { date: new Date(since * 1000).toLocaleDateString(settings.lang === "ru" ? "ru-RU" : "en-GB") }), W - 40, 60);
        g.textAlign = "left";
      }
      setReady(true);
    })();
  }, [active?.skinUrl, instances, nick, settings.lang, settings.profile.avatar, settings.streamer, t]);

  const data = () => ref.current?.toDataURL("image/png") ?? "";

  return (
    <Modal title={t("card.title")} onClose={onClose} wide xl>
      <canvas ref={ref} width={W} height={H} className="player-card" />
      <div className="modal__foot">
        <button
          className="btn"
          disabled={!ready}
          onClick={async () => {
            try {
              await api.copyPng(data());
              sfx.success();
              toast(t("card.copied"), "ok");
            } catch (e) {
              toast(errText(e), "error");
            }
          }}
        >
          <Icon name="copy" size={16} /> {t("card.copy")}
        </button>
        <button
          className="btn btn--primary"
          disabled={!ready}
          onClick={async () => {
            try {
              const dest = await save({ defaultPath: `${nick}-card.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
              if (!dest) return;
              await api.savePng(dest, data());
              toast(t("card.saved"), "ok");
            } catch (e) {
              toast(errText(e), "error");
            }
          }}
        >
          <Icon name="download" size={16} /> {t("card.save")}
        </button>
      </div>
    </Modal>
  );
}
