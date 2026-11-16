import { useEffect, useRef } from "react";
import { Balloon, Cake, FirBranch, GarlandSegment, Pumpkin, Spider } from "./SeasonArt";

export type SeasonEvent = "newyear" | "halloween" | "birthday" | null;

/** Which celebration is on today (birthday wins). `birthday` is "YYYY-MM-DD". */
export function seasonToday(birthday?: string | null, d = new Date()): SeasonEvent {
  if (birthday) {
    const [, m, day] = birthday.split("-").map(Number);
    if (m === d.getMonth() + 1 && day === d.getDate()) return "birthday";
  }
  const m = d.getMonth(), day = d.getDate();
  if ((m === 11 && day >= 20) || (m === 0 && day <= 10)) return "newyear";
  if ((m === 9 && day >= 25) || (m === 10 && day <= 2)) return "halloween";
  return null;
}

/** Age today from a "YYYY-MM-DD" birthday. */
export function ageToday(birthday: string, d = new Date()): number {
  const [y, m, day] = birthday.split("-").map(Number);
  let age = d.getFullYear() - y;
  if (d.getMonth() + 1 < m || (d.getMonth() + 1 === m && d.getDate() < day)) age--;
  return age;
}

type P = { x: number; y: number; vx: number; vy: number; z: number; k: number; c: number; a: number; s: number; f: number };

const CONFETTI = ["#f43f5e", "#f59e0b", "#facc15", "#22c55e", "#06b6d4", "#8b5cf6", "#ec4899"];
const LEAVES: [string, string][] = [["#f97316", "#9a3412"], ["#facc15", "#b45309"], ["#dc2626", "#7f1d1d"], ["#d97706", "#78350f"]];

function sprite(w: number, h: number, paint: (g: CanvasRenderingContext2D) => void) {
  const c = document.createElement("canvas");
  c.width = w;
  c.height = h;
  paint(c.getContext("2d")!);
  return c;
}

/** Six-armed ice crystal. */
function flakeSprite(seed: number) {
  return sprite(64, 64, (g) => {
    g.translate(32, 32);
    g.strokeStyle = "#fff";
    g.lineCap = "round";
    g.shadowColor = "rgba(186,230,253,.9)";
    g.shadowBlur = 4;
    const branch = 0.35 + (seed % 3) * 0.12;
    for (let i = 0; i < 6; i++) {
      g.rotate(Math.PI / 3);
      g.lineWidth = 2.4;
      g.beginPath();
      g.moveTo(0, 0);
      g.lineTo(0, -26);
      g.stroke();
      g.lineWidth = 1.6;
      for (const t of [branch, branch + 0.3]) {
        const len = 9 * (1 - t * 0.6);
        g.beginPath();
        g.moveTo(0, -26 * t);
        g.lineTo(-len, -26 * t - len);
        g.moveTo(0, -26 * t);
        g.lineTo(len, -26 * t - len);
        g.stroke();
      }
    }
    g.fillStyle = "#fff";
    g.beginPath();
    g.arc(0, 0, 3, 0, 6.28);
    g.fill();
  });
}

/** Soft out-of-focus snow dot. */
const dotSprite = () =>
  sprite(32, 32, (g) => {
    const r = g.createRadialGradient(16, 16, 0, 16, 16, 16);
    r.addColorStop(0, "rgba(255,255,255,1)");
    r.addColorStop(0.4, "rgba(255,255,255,.75)");
    r.addColorStop(1, "rgba(255,255,255,0)");
    g.fillStyle = r;
    g.fillRect(0, 0, 32, 32);
  });

/** A maple-ish leaf with veins and stem. */
function leafSprite([light, dark]: [string, string], oak: boolean) {
  return sprite(64, 64, (g) => {
    g.translate(32, 34);
    const path = new Path2D();
    const N = 140;
    for (let i = 0; i <= N; i++) {
      const t = (i / N) * Math.PI * 2;
      let r: number;
      if (oak) r = 17 + 4 * Math.cos(t * 7) * Math.sin(t / 2);
      else r = 14 + 11 * Math.pow(Math.abs(Math.cos(t * 2.5)), 3) + 4 * Math.pow(Math.abs(Math.cos(t * 7.5)), 6);
      const stretch = oak ? 1.45 : 1;
      const x = Math.sin(t) * r * (oak ? 0.7 : 1);
      const y = -Math.cos(t) * r * stretch * (oak ? 1 : 1) * (t > 2.4 && t < 3.9 ? 0.55 : 1);
      if (i === 0) path.moveTo(x, y);
      else path.lineTo(x, y);
    }
    path.closePath();
    const gr = g.createLinearGradient(-20, -24, 20, 24);
    gr.addColorStop(0, light);
    gr.addColorStop(1, dark);
    g.fillStyle = gr;
    g.fill(path);
    g.strokeStyle = "rgba(0,0,0,.25)";
    g.lineWidth = 1;
    g.stroke(path);
    g.strokeStyle = "rgba(60,25,5,.55)";
    g.lineWidth = 1.2;
    g.beginPath();
    g.moveTo(0, 26);
    g.lineTo(0, -20);
    for (const a of oak ? [-0.6, 0.6, -0.9, 0.9] : [-1.0, 1.0, -0.45, 0.45]) {
      g.moveTo(0, 4);
      g.lineTo(Math.sin(a) * 18, 4 - Math.cos(a) * 18);
    }
    g.stroke();
    g.strokeStyle = dark;
    g.lineWidth = 2;
    g.beginPath();
    g.moveTo(0, 14);
    g.quadraticCurveTo(1, 24, -3, 29);
    g.stroke();
  });
}

function drawBat(g: CanvasRenderingContext2D, x: number, y: number, s: number, flap: number) {
  g.save();
  g.translate(x, y);
  g.scale(s, s);
  g.fillStyle = "#140c1f";
  for (const side of [-1, 1]) {
    const tipY = -14 + flap * 22;
    g.beginPath();
    g.moveTo(side * 3, -3);
    g.quadraticCurveTo(side * 14, -10 + flap * 8, side * 30, tipY);
    g.quadraticCurveTo(side * 26, tipY + 6, side * 25, tipY + 10);
    g.quadraticCurveTo(side * 20, tipY + 7, side * 17, tipY + 12);
    g.quadraticCurveTo(side * 12, tipY + 8, side * 9, tipY + 13 - flap * 4);
    g.quadraticCurveTo(side * 6, 2, side * 3, 4);
    g.closePath();
    g.fill();
  }
  g.beginPath();
  g.ellipse(0, 1, 4.5, 7, 0, 0, 6.28);
  g.fill();
  g.beginPath();
  g.moveTo(-4, -4); g.lineTo(-3.5, -11); g.lineTo(-1, -6);
  g.moveTo(4, -4); g.lineTo(3.5, -11); g.lineTo(1, -6);
  g.fill();
  g.fillStyle = "#fb923c";
  g.shadowColor = "#fb923c";
  g.shadowBlur = 4;
  g.fillRect(-2.4, -3, 1.6, 1.6);
  g.fillRect(0.8, -3, 1.6, 1.6);
  g.restore();
}

/** Full-window particles: snow (New Year), bats and leaves (Halloween), confetti (birthday). */
export function SeasonalLayer({ event, animate, density = 1 }: { event: SeasonEvent; animate: boolean; density?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const c = ref.current;
    if (!c || !event) return;
    const g = c.getContext("2d")!;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    let W = 0, H = 0;
    const fit = () => {
      W = window.innerWidth;
      H = window.innerHeight;
      c.width = W * dpr;
      c.height = H * dpr;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    fit();
    window.addEventListener("resize", fit);

    const flakes = [0, 1, 2].map(flakeSprite);
    const dot = dotSprite();
    const leaves = LEAVES.flatMap((col) => [leafSprite(col, false), leafSprite(col, true)]);
    const rnd = (a: number, b: number) => a + Math.random() * (b - a);

    const make = (top = false): P => {
      const z = Math.random(); // depth: 0 far … 1 near
      const y = top ? -40 : Math.random() * H;
      if (event === "birthday")
        return { x: Math.random() * W, y: top ? -20 : y, vx: rnd(-0.8, 0.8), vy: rnd(1.4, 3.2), z, k: Math.floor(Math.random() * 10), c: Math.floor(Math.random() * CONFETTI.length), a: Math.random() * 6.28, s: rnd(-0.12, 0.12), f: Math.random() * 6.28 };
      if (event === "halloween")
        return { x: Math.random() * W, y, vx: rnd(-0.4, 0.4), vy: rnd(0.5, 1.2) * (0.6 + z), z, k: Math.floor(Math.random() * leaves.length), c: 0, a: Math.random() * 6.28, s: rnd(-0.03, 0.03), f: Math.random() * 6.28 };
      return { x: Math.random() * W, y, vx: 0, vy: 0.3 + z * 1.4, z, k: Math.random() < 0.28 ? Math.floor(Math.random() * 3) : -1, c: 0, a: Math.random() * 6.28, s: rnd(-0.01, 0.01), f: Math.random() * 6.28 };
    };
    const count = Math.max(4, Math.round((event === "birthday" ? 170 : event === "halloween" ? 34 : 150) * density));
    let parts: P[] = Array.from({ length: count }, () => make());
    const bats = event === "halloween" ? Array.from({ length: Math.max(1, Math.round(5 * Math.min(density, 1.6))) }, (_, i) => ({ x: i < 2 ? 200 + Math.random() * (W - 400) : -100 - i * 260, y: 60 + i * 60, v: rnd(1.1, 2), s: rnd(1.1, 1.6), ph: Math.random() * 6 })) : [];
    let frame = 0;
    let raf = 0;
    const start = performance.now();

    const draw = () => {
      raf = requestAnimationFrame(draw);
      frame++;
      g.clearRect(0, 0, W, H);
      const t = (performance.now() - start) / 1000;
      if (event === "birthday" && t > 12 && parts.length > 45 * density && frame % 2 === 0) parts = parts.slice(0, parts.length - 1);
      for (const p of parts) {
        p.a += p.s;
        p.f += 0.08;
        if (event === "newyear") {
          p.x += Math.sin(frame * 0.012 + p.a * 3) * (0.3 + p.z * 0.4);
          if (p.k >= 0) {
            const size = 10 + p.z * 14;
            g.globalAlpha = 0.65 + p.z * 0.35;
            g.save();
            g.translate(p.x, p.y);
            g.rotate(p.a);
            g.drawImage(flakes[p.k], -size / 2, -size / 2, size, size);
            g.restore();
          } else {
            const size = 3 + p.z * 9;
            g.globalAlpha = 0.35 + p.z * 0.6;
            g.drawImage(dot, p.x - size / 2, p.y - size / 2, size, size);
          }
        } else if (event === "halloween") {
          const size = 26 + p.z * 26;
          g.globalAlpha = 0.75 + p.z * 0.25;
          g.save();
          g.translate(p.x, p.y);
          g.rotate(p.a + Math.sin(p.f * 0.3) * 0.6);
          g.scale(Math.cos(p.f * 0.25), 1); // flutter
          g.drawImage(leaves[p.k], -size / 2, -size / 2, size, size);
          g.restore();
          p.x += p.vx + Math.sin(frame * 0.02 + p.a) * 0.9;
        } else {
          g.globalAlpha = 1;
          const col = CONFETTI[p.c];
          g.save();
          g.translate(p.x, p.y);
          g.rotate(p.a);
          const flip = Math.cos(p.f);
          if (p.k < 6) {
            g.scale(1, flip);
            g.fillStyle = flip > 0 ? col : shade(col);
            g.fillRect(-5, -3, 10, 6);
          } else if (p.k < 8) {
            g.scale(flip, 1);
            g.fillStyle = col;
            g.beginPath();
            g.arc(0, 0, 4, 0, 6.28);
            g.fill();
          } else {
            g.strokeStyle = col;
            g.lineWidth = 3;
            g.lineCap = "round";
            g.beginPath();
            for (let i = 0; i <= 12; i++) {
              const yy = -14 + i * 2.4;
              const xx = Math.sin(i * 0.9 + p.f) * 4;
              i ? g.lineTo(xx, yy) : g.moveTo(xx, yy);
            }
            g.stroke();
          }
          g.restore();
          p.x += p.vx + Math.sin(p.f * 0.5) * 0.6;
        }
        p.y += p.vy;
        if (p.y > H + 40) Object.assign(p, make(true));
      }
      g.globalAlpha = 1;
      for (const b of bats) {
        b.x += b.v;
        const flap = (Math.sin(frame * 0.32 + b.ph) + 1) / 2;
        const y = b.y + Math.sin(frame * 0.04 + b.ph) * 16 - flap * 3;
        drawBat(g, b.x, y, b.s, flap);
        if (b.x > W + 60) {
          b.x = -60 - Math.random() * 500;
          b.y = 40 + Math.random() * (H * 0.5);
        }
      }
      if (!animate) cancelAnimationFrame(raf);
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener("resize", fit);
    };
  }, [event, animate, density]);
  if (!event) return null;
  return <canvas ref={ref} className="seasonal" aria-hidden="true" />;
}

function shade(hex: string) {
  const n = parseInt(hex.slice(1), 16);
  const f = (v: number) => Math.round(v * 0.62);
  return `rgb(${f(n >> 16)},${f((n >> 8) & 255)},${f(n & 255)})`;
}

const BULBS = ["#ef4444", "#facc15", "#22c55e", "#3b82f6", "#ec4899"];

/** Decorations inside the hero: garland and fir branch, pumpkins and a spider, or cake with balloons. */
export function HeroDecor({ event, name, age, lang }: { event: SeasonEvent; name: string; age: number | null; lang: "ru" | "en" }) {
  if (event === "newyear")
    return (
      <>
        <div className="sa-garland" aria-hidden="true">
          {Array.from({ length: 30 }, (_, i) => (
            <span key={i} style={{ animationDelay: `${(i % 5) * 0.35}s` }} className="sa-garland__cell">
              <GarlandSegment color={BULBS[i % BULBS.length]} />
            </span>
          ))}
        </div>
        <div className="sa-fir-wrap"><FirBranch /></div>
      </>
    );
  if (event === "halloween")
    return (
      <>
        <div className="sa-spider-wrap"><Spider /></div>
        <div className="sa-pumpkins" aria-hidden="true">
          <Pumpkin size={78} face={1} />
          <Pumpkin size={118} />
        </div>
      </>
    );
  if (event === "birthday")
    return (
      <>
        <div className="sa-balloons" aria-hidden="true">
          <span style={{ animationDelay: "0s" }}><Balloon color="#f43f5e" size={44} /></span>
          <span style={{ animationDelay: ".8s" }}><Balloon color="#38bdf8" size={38} /></span>
          <span style={{ animationDelay: "1.6s" }}><Balloon color="#facc15" size={41} /></span>
        </div>
        <div className="bday" role="status">
          <Cake age={age} />
          <span className="bday__text">
            {lang === "ru" ? `С днём рождения, ${name}!` : `Happy birthday, ${name}!`}
            {age !== null && age > 0 && (
              <small>{lang === "ru" ? `Тебе исполнилось ${age} ${ageWord(age)}` : `You're ${age} today`}</small>
            )}
          </span>
        </div>
      </>
    );
  return null;
}

function ageWord(n: number) {
  const a = n % 10, b = n % 100;
  if (a === 1 && b !== 11) return "год";
  if (a >= 2 && a <= 4 && (b < 12 || b > 14)) return "года";
  return "лет";
}
