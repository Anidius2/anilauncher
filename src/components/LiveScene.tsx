import { useEffect, useRef } from "react";

export type Weather = "auto" | "clear" | "rain" | "snow" | "fireflies";

/** Deterministic random numbers so the landscape is the same on every start. */
function rng(seed: number) {
  let s = seed >>> 0;
  return () => {
    s = (s * 1664525 + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

type RGB = [number, number, number];
const hex = (h: string): RGB => [parseInt(h.slice(1, 3), 16), parseInt(h.slice(3, 5), 16), parseInt(h.slice(5, 7), 16)];
const mix = (a: RGB, b: RGB, t: number): RGB => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
const css = (c: RGB, k = 1) => `rgb(${Math.round(c[0] * k)},${Math.round(c[1] * k)},${Math.round(c[2] * k)})`;

// Sky colors through the day: [hour, top, bottom]
const sky: Array<[number, string, string]> = [
  [0, "#070b1f", "#151c45"],
  [4.5, "#0d1433", "#2a2456"],
  [5.8, "#2b3a78", "#c46a6a"],
  [6.8, "#4f7fc4", "#f5b06b"],
  [8.5, "#5aa7e6", "#bfe3ff"],
  [16.5, "#4f9ee0", "#cde8ff"],
  [18.3, "#3d5aa8", "#f29a5c"],
  [19.4, "#2a2a6b", "#c2577a"],
  [20.6, "#121738", "#3a2b5e"],
  [24, "#070b1f", "#151c45"],
];

function skyAt(h: number): [RGB, RGB] {
  for (let i = 0; i < sky.length - 1; i++) {
    const [h0, t0, b0] = sky[i], [h1, t1, b1] = sky[i + 1];
    if (h >= h0 && h <= h1) {
      const t = (h - h0) / (h1 - h0);
      return [mix(hex(t0), hex(t1), t), mix(hex(b0), hex(b1), t)];
    }
  }
  return [hex(sky[0][1]), hex(sky[0][2])];
}

/** 0 at night, 1 in full day. */
function daylight(h: number) {
  if (h < 5 || h > 21) return 0.18;
  if (h < 7.5) return 0.18 + ((h - 5) / 2.5) * 0.82;
  if (h > 18) return 1 - ((h - 18) / 3) * 0.82;
  return 1;
}

/** Weather of the day when set to auto: the same all day, a bit of snow in winter. */
export function autoWeather(d = new Date()): Exclude<Weather, "auto"> {
  const seed = d.getFullYear() * 400 + d.getMonth() * 32 + d.getDate();
  const r = rng(seed)();
  const winter = d.getMonth() === 11 || d.getMonth() <= 1;
  if (winter && r < 0.35) return "snow";
  if (r < 0.2) return "rain";
  const h = d.getHours();
  if (!winter && (h >= 21 || h < 4) && r < 0.6) return "fireflies";
  return "clear";
}

/**
 * Animated pixel landscape: the sky follows the real time of day (sunrise, sunset, stars and moon),
 * clouds drift, hills scroll slowly, and there can be rain, snow or fireflies.
 */
export function LiveScene({ weather = "auto", animate = true, hour }: { weather?: Weather; animate?: boolean; hour?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas) return;
    const g = canvas.getContext("2d");
    if (!g) return;
    const W = 320;
    let H = 107;
    const resize = () => {
      const r = canvas.getBoundingClientRect();
      H = Math.max(60, Math.round((W * r.height) / Math.max(1, r.width)));
      canvas.width = W;
      canvas.height = H;
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(canvas);

    const rand = rng(20261116);
    // hills: three layers of stepped heights, wider than the screen so they can scroll
    const layers = [0.62, 0.72, 0.82].map((base, i) => {
      const cols: number[] = [];
      let y = base;
      for (let x = 0; x < W * 2; x += 8 + i * 2) {
        y = Math.min(base + 0.08, Math.max(base - 0.12 + i * 0.03, y + (rand() - 0.5) * 0.07));
        cols.push(y);
      }
      return { cols, step: 8 + i * 2, speed: 0.6 + i * 0.9 };
    });
    const trees = Array.from({ length: 14 }, () => ({ x: rand() * W * 2, s: 4 + Math.floor(rand() * 4) }));
    const stars = Array.from({ length: 70 }, () => ({ x: rand() * W, y: rand() * 0.55, p: rand() * 6.28 }));
    const clouds = Array.from({ length: 6 }, () => ({ x: rand() * W, y: 0.08 + rand() * 0.25, w: 18 + rand() * 30, v: 0.6 + rand() * 0.8 }));
    const wx: Exclude<Weather, "auto"> = weather === "auto" ? autoWeather() : weather;
    const drops = Array.from({ length: wx === "rain" ? 140 : wx === "snow" ? 90 : wx === "fireflies" ? 24 : 0 }, () => ({
      x: rand() * W,
      y: rand(),
      v: 0.5 + rand() * 0.8,
      p: rand() * 6.28,
    }));
    let bird: { x: number; y: number } | null = null;
    let shooting: { x: number; y: number; life: number } | null = null;

    let frame = 0;
    let raf = 0;
    let last = 0;
    const draw = (now: number) => {
      raf = requestAnimationFrame(draw);
      if (animate && now - last < 33) return; // ~30 fps is plenty for pixel art
      last = now;
      frame++;
      const d = new Date();
      const h = hour ?? d.getHours() + d.getMinutes() / 60 + d.getSeconds() / 3600;
      const light = daylight(h) * (wx === "rain" ? 0.75 : wx === "snow" ? 0.9 : 1);
      const [top, bottom] = skyAt(h);
      const grey: RGB = [120, 130, 145];
      const t2 = wx === "rain" ? 0.45 : wx === "snow" ? 0.25 : 0;
      const grad = g.createLinearGradient(0, 0, 0, H);
      grad.addColorStop(0, css(mix(top, grey, t2 * light)));
      grad.addColorStop(1, css(mix(bottom, grey, t2 * light)));
      g.fillStyle = grad;
      g.fillRect(0, 0, W, H);

      // stars and a shooting star at night
      const night = 1 - Math.min(1, (light - 0.18) / 0.5);
      if (night > 0.02 && wx !== "rain") {
        for (const s of stars) {
          const tw = 0.5 + 0.5 * Math.sin(frame * 0.05 + s.p);
          g.fillStyle = `rgba(255,255,240,${night * tw * 0.9})`;
          g.fillRect(Math.floor(s.x), Math.floor(s.y * H), 1, 1);
        }
        if (!shooting && Math.random() < 0.002) shooting = { x: Math.random() * W * 0.7, y: Math.random() * H * 0.25, life: 1 };
        if (shooting) {
          g.fillStyle = `rgba(255,255,255,${shooting.life * night})`;
          for (let i = 0; i < 6; i++) g.fillRect(Math.floor(shooting.x - i * 2), Math.floor(shooting.y - i), 1, 1);
          shooting.x += 4;
          shooting.y += 2;
          shooting.life -= 0.04;
          if (shooting.life <= 0) shooting = null;
        }
      }

      // sun (day) and moon (night) moving along an arc
      const arc = (p: number) => ({ x: W * (0.08 + 0.84 * p), y: H * (0.62 - 0.5 * Math.sin(Math.PI * p)) });
      if (h >= 5.5 && h <= 20) {
        const s = arc((h - 5.5) / 14.5);
        g.fillStyle = `rgba(253,230,138,${wx === "rain" ? 0.35 : 1})`;
        g.fillRect(Math.round(s.x) - 6, Math.round(s.y) - 6, 12, 12);
        g.fillStyle = `rgba(255,247,209,${wx === "rain" ? 0.35 : 1})`;
        g.fillRect(Math.round(s.x) - 4, Math.round(s.y) - 4, 8, 8);
      }
      if (h >= 19 || h <= 6.5) {
        const p = h >= 19 ? (h - 19) / 11.5 : (h + 5) / 11.5;
        const m = arc(p);
        g.fillStyle = "rgba(226,232,240,.95)";
        g.fillRect(Math.round(m.x) - 5, Math.round(m.y) - 5, 10, 10);
        g.fillStyle = "rgba(148,163,184,.9)";
        g.fillRect(Math.round(m.x) - 2, Math.round(m.y) - 3, 2, 2);
        g.fillRect(Math.round(m.x) + 1, Math.round(m.y) + 1, 2, 2);
      }

      // clouds
      for (const c of clouds) {
        if (animate) c.x += c.v * 0.12;
        if (c.x > W + c.w) c.x = -c.w;
        const k = 0.55 + 0.45 * light;
        g.fillStyle = wx === "rain" ? css([110, 118, 135], k) : css([245, 245, 255], k);
        g.globalAlpha = wx === "clear" || wx === "fireflies" ? 0.75 : 0.95;
        const x = Math.round(c.x), y = Math.round(c.y * H);
        g.fillRect(x, y, Math.round(c.w), 4);
        g.fillRect(x + 4, y - 3, Math.round(c.w * 0.5), 3);
        g.globalAlpha = 1;
      }

      // a bird now and then during the day
      if (light > 0.7 && wx === "clear") {
        if (!bird && Math.random() < 0.003) bird = { x: -5, y: H * (0.15 + Math.random() * 0.25) };
        if (bird) {
          bird.x += 0.9;
          const flap = Math.floor(frame / 6) % 2;
          g.fillStyle = "rgba(30,30,40,.8)";
          g.fillRect(Math.round(bird.x), Math.round(bird.y), 1, 1);
          g.fillRect(Math.round(bird.x) - 1, Math.round(bird.y) - flap, 1, 1);
          g.fillRect(Math.round(bird.x) + 1, Math.round(bird.y) - flap, 1, 1);
          if (bird.x > W + 5) bird = null;
        }
      }

      // hills, far to near, darker at night
      const hillColors: RGB[] = wx === "snow" ? [[196, 205, 220], [170, 182, 200], [140, 152, 172]] : [[92, 130, 92], [66, 105, 64], [44, 78, 46]];
      layers.forEach((L, i) => {
        const off = animate ? (frame * L.speed * 0.05) % (L.cols.length * L.step - W) : 0;
        g.fillStyle = css(hillColors[i], 0.35 + 0.65 * light);
        for (let k = 0; k < L.cols.length; k++) {
          const x = Math.floor(k * L.step - off);
          if (x > W || x + L.step < 0) continue;
          const y = Math.floor(L.cols[k] * H);
          g.fillRect(x, y, L.step, H - y);
        }
        if (i === 1) {
          for (const tr of trees) {
            const x = Math.floor(tr.x - off);
            if (x < -10 || x > W + 10) continue;
            const k = Math.min(L.cols.length - 1, Math.max(0, Math.floor((tr.x) / L.step)));
            const gy = Math.floor(L.cols[k] * H);
            g.fillStyle = css([92, 64, 40], 0.35 + 0.65 * light);
            g.fillRect(x + Math.floor(tr.s / 2) - 1, gy - tr.s, 2, tr.s);
            g.fillStyle = css(wx === "snow" ? [225, 232, 240] : [34, 90, 44], 0.35 + 0.65 * light);
            g.fillRect(x, gy - tr.s * 2, tr.s, tr.s + 1);
          }
        }
      });

      // weather
      if (wx === "rain") {
        g.fillStyle = "rgba(190,210,240,.55)";
        for (const r of drops) {
          if (animate) {
            r.y += 0.03 * r.v;
            r.x -= 0.6 * r.v;
            if (r.y > 1) { r.y = -0.05; r.x = Math.random() * (W + 40); }
          }
          g.fillRect(Math.floor(r.x), Math.floor(r.y * H), 1, 3);
        }
      } else if (wx === "snow") {
        g.fillStyle = "rgba(255,255,255,.9)";
        for (const f of drops) {
          if (animate) {
            f.y += 0.004 * f.v;
            f.x += Math.sin(frame * 0.03 + f.p) * 0.2;
            if (f.y > 1) { f.y = -0.02; f.x = Math.random() * W; }
          }
          g.fillRect(Math.floor(f.x), Math.floor(f.y * H), f.v > 1 ? 2 : 1, f.v > 1 ? 2 : 1);
        }
      } else if (wx === "fireflies" && night > 0.3) {
        for (const f of drops) {
          if (animate) {
            f.x += Math.sin(frame * 0.02 + f.p) * 0.25;
            f.y = 0.7 + 0.2 * Math.sin(frame * 0.01 + f.p * 2);
          }
          const glow = 0.4 + 0.6 * Math.max(0, Math.sin(frame * 0.08 + f.p * 3));
          g.fillStyle = `rgba(253,224,71,${glow * night})`;
          const sz = glow > 0.75 ? 2 : 1;
          g.fillRect(Math.floor(f.x), Math.floor(f.y * H), sz, sz);
          if (glow > 0.85) {
            g.fillStyle = `rgba(253,224,71,${0.25 * night})`;
            g.fillRect(Math.floor(f.x) - 1, Math.floor(f.y * H) - 1, 4, 4);
          }
        }
      }
      if (!animate) cancelAnimationFrame(raf);
    };
    raf = requestAnimationFrame(draw);
    const onVis = () => {
      if (document.hidden) cancelAnimationFrame(raf);
      else raf = requestAnimationFrame(draw);
    };
    document.addEventListener("visibilitychange", onVis);
    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      document.removeEventListener("visibilitychange", onVis);
    };
  }, [weather, animate, hour]);

  return <canvas ref={ref} className="art-fill live-scene" aria-hidden="true" />;
}
