// Pixel helpers for the skin editor: skin layout, mirroring, colors, shapes, arm conversion.

export const SIZE = 64;
export type RGBA = [number, number, number, number];

export type PartId = "head" | "body" | "rightArm" | "leftArm" | "rightLeg" | "leftLeg";

export interface Box {
  u: number;
  v: number;
  w: number;
  h: number;
  d: number;
  part: PartId;
  layer: "base" | "overlay";
  /** index of the mirrored box (itself for head and body) */
  mirror: number;
}

/** Texture layout of a 64×64 skin. `slim` arms are 3 pixels wide. */
export function boxes(slim: boolean): Box[] {
  const aw = slim ? 3 : 4;
  return [
    { u: 0, v: 0, w: 8, h: 8, d: 8, part: "head", layer: "base", mirror: 0 },
    { u: 32, v: 0, w: 8, h: 8, d: 8, part: "head", layer: "overlay", mirror: 1 },
    { u: 16, v: 16, w: 8, h: 12, d: 4, part: "body", layer: "base", mirror: 2 },
    { u: 16, v: 32, w: 8, h: 12, d: 4, part: "body", layer: "overlay", mirror: 3 },
    { u: 40, v: 16, w: aw, h: 12, d: 4, part: "rightArm", layer: "base", mirror: 6 },
    { u: 40, v: 32, w: aw, h: 12, d: 4, part: "rightArm", layer: "overlay", mirror: 7 },
    { u: 32, v: 48, w: aw, h: 12, d: 4, part: "leftArm", layer: "base", mirror: 4 },
    { u: 48, v: 48, w: aw, h: 12, d: 4, part: "leftArm", layer: "overlay", mirror: 5 },
    { u: 0, v: 16, w: 4, h: 12, d: 4, part: "rightLeg", layer: "base", mirror: 10 },
    { u: 0, v: 32, w: 4, h: 12, d: 4, part: "rightLeg", layer: "overlay", mirror: 11 },
    { u: 16, v: 48, w: 4, h: 12, d: 4, part: "leftLeg", layer: "base", mirror: 8 },
    { u: 0, v: 48, w: 4, h: 12, d: 4, part: "leftLeg", layer: "overlay", mirror: 9 },
  ];
}

/** Is (x, y) on one of the box's six faces (not the unused corners)? */
function onBox(b: Box, x: number, y: number): boolean {
  const lx = x - b.u, ly = y - b.v;
  if (ly < 0 || ly >= b.d + b.h || lx < 0) return false;
  if (ly < b.d) return lx >= b.d && lx < b.d + 2 * b.w;
  return lx < 2 * (b.d + b.w);
}

export function boxAt(x: number, y: number, slim: boolean): { box: Box; index: number } | null {
  const list = boxes(slim);
  for (let i = 0; i < list.length; i++) if (onBox(list[i], x, y)) return { box: list[i], index: i };
  return null;
}

/** The pixel on the opposite side of the body (left ↔ right). */
export function mirrorPixel(x: number, y: number, slim: boolean): [number, number] | null {
  const hit = boxAt(x, y, slim);
  if (!hit) return null;
  const { box: b } = hit;
  const target = boxes(slim)[b.mirror];
  const lx = x - b.u, ly = y - b.v;
  const { w, d } = b;
  let nx: number;
  if (ly < d) {
    if (lx < d + w) nx = d + (w - 1 - (lx - d)); // top
    else nx = d + w + (w - 1 - (lx - d - w)); // bottom
  } else if (lx < d) nx = d + w + (d - 1 - lx); // right side → left side
  else if (lx < d + w) nx = d + (w - 1 - (lx - d)); // front
  else if (lx < 2 * d + w) nx = d - 1 - (lx - d - w); // left side → right side
  else nx = 2 * d + w + (w - 1 - (lx - 2 * d - w)); // back
  return [target.u + nx, target.v + ly];
}

export const partNames: Record<PartId, { ru: string; en: string }> = {
  head: { ru: "Голова", en: "Head" },
  body: { ru: "Тело", en: "Body" },
  rightArm: { ru: "Правая рука", en: "Right arm" },
  leftArm: { ru: "Левая рука", en: "Left arm" },
  rightLeg: { ru: "Правая нога", en: "Right leg" },
  leftLeg: { ru: "Левая нога", en: "Left leg" },
};

// ---------- colors ----------

export function hexToRgba(hex: string): RGBA {
  const h = hex.replace("#", "");
  const full = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  return [parseInt(full.slice(0, 2), 16) || 0, parseInt(full.slice(2, 4), 16) || 0, parseInt(full.slice(4, 6), 16) || 0, 255];
}

export function rgbaToHex(c: ArrayLike<number>): string {
  return "#" + [c[0], c[1], c[2]].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("");
}

export function getPx(d: Uint8ClampedArray, x: number, y: number): RGBA {
  const k = (y * SIZE + x) * 4;
  return [d[k], d[k + 1], d[k + 2], d[k + 3]];
}

export function setPx(d: Uint8ClampedArray, x: number, y: number, c: RGBA) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return;
  d.set(c, (y * SIZE + x) * 4);
}

export function shade(c: RGBA, amount: number, lighten: boolean): RGBA {
  if (c[3] === 0) return c;
  const f = (v: number) => (lighten ? v + (255 - v) * amount : v * (1 - amount));
  return [f(c[0]), f(c[1]), f(c[2]), c[3]];
}

export function noisy(base: RGBA, strength: number): RGBA {
  const k = 1 + (Math.random() * 2 - 1) * strength;
  const f = (v: number) => Math.max(0, Math.min(255, v * k));
  return [f(base[0]), f(base[1]), f(base[2]), 255];
}

export function same(a: RGBA, b: RGBA) {
  return a[0] === b[0] && a[1] === b[1] && a[2] === b[2] && a[3] === b[3];
}

// ---------- shapes ----------

export function linePixels(x0: number, y0: number, x1: number, y1: number): Array<[number, number]> {
  const out: Array<[number, number]> = [];
  const dx = Math.abs(x1 - x0), dy = -Math.abs(y1 - y0);
  const sx = x0 < x1 ? 1 : -1, sy = y0 < y1 ? 1 : -1;
  let err = dx + dy;
  let x = x0, y = y0;
  for (;;) {
    out.push([x, y]);
    if (x === x1 && y === y1) break;
    const e2 = 2 * err;
    if (e2 >= dy) {
      err += dy;
      x += sx;
    }
    if (e2 <= dx) {
      err += dx;
      y += sy;
    }
  }
  return out;
}

export function rectPixels(x0: number, y0: number, x1: number, y1: number, filled: boolean): Array<[number, number]> {
  const out: Array<[number, number]> = [];
  const [ax, bx] = [Math.min(x0, x1), Math.max(x0, x1)];
  const [ay, by] = [Math.min(y0, y1), Math.max(y0, y1)];
  for (let y = ay; y <= by; y++)
    for (let x = ax; x <= bx; x++) if (filled || x === ax || x === bx || y === ay || y === by) out.push([x, y]);
  return out;
}

/** Square brush around a pixel, kept inside the same box so faces don't bleed. */
export function brushPixels(x: number, y: number, size: number, slim: boolean): Array<[number, number]> {
  if (size <= 1) return [[x, y]];
  const hit = boxAt(x, y, slim);
  const out: Array<[number, number]> = [];
  const r0 = -Math.floor((size - 1) / 2);
  for (let j = 0; j < size; j++)
    for (let i = 0; i < size; i++) {
      const px = x + r0 + i, py = y + r0 + j;
      if (px < 0 || py < 0 || px >= SIZE || py >= SIZE) continue;
      if (hit && boxAt(px, py, slim)?.index !== hit.index) continue;
      out.push([px, py]);
    }
  return out;
}

/** Flood fill; `scope` "part" stays inside the clicked box. */
export function floodFill(d: Uint8ClampedArray, x: number, y: number, color: RGBA, scope: "part" | "all", slim: boolean): Array<[number, number]> {
  const target = getPx(d, x, y);
  if (same(target, color)) return [];
  const hit = scope === "part" ? boxAt(x, y, slim) : null;
  const seen = new Uint8Array(SIZE * SIZE);
  const out: Array<[number, number]> = [];
  const stack: Array<[number, number]> = [[x, y]];
  while (stack.length) {
    const [cx, cy] = stack.pop()!;
    if (cx < 0 || cy < 0 || cx >= SIZE || cy >= SIZE) continue;
    const k = cy * SIZE + cx;
    if (seen[k]) continue;
    seen[k] = 1;
    if (!same(getPx(d, cx, cy), target)) continue;
    if (hit && boxAt(cx, cy, slim)?.index !== hit.index) continue;
    out.push([cx, cy]);
    stack.push([cx + 1, cy], [cx - 1, cy], [cx, cy + 1], [cx, cy - 1]);
  }
  return out;
}

// ---------- arm width conversion ----------

/** Re-lays an arm box from one width to another (3 ↔ 4), face by face. */
function convertArm(src: Uint8ClampedArray, dst: Uint8ClampedArray, u: number, v: number, from: number, to: number) {
  const d = 4, h = 12;
  // clear the area first
  for (let y = v; y < v + d + h; y++) for (let x = u; x < u + 2 * (d + 4); x++) setPx(dst, x, y, [0, 0, 0, 0]);
  const face = (fx: number, fy: number, fw: number, fh: number, tx: number, tw: number) => {
    for (let y = 0; y < fh; y++)
      for (let x = 0; x < tw; x++) {
        const sx = Math.min(x, fw - 1); // widen by repeating the last column, narrow by dropping it
        setPx(dst, u + tx + x, v + fy + y, getPx(src, u + fx + sx, v + fy + y));
      }
  };
  face(d, 0, from, d, d, to); // top
  face(d + from, 0, from, d, d + to, to); // bottom
  face(0, d, d, h, 0, d); // right side
  face(d, d, from, h, d, to); // front
  face(d + from, d, d, h, d + to, d); // left side
  face(2 * d + from, d, from, h, 2 * d + to, to); // back
}

export function convertArms(data: Uint8ClampedArray, toSlim: boolean): Uint8ClampedArray {
  const out = new Uint8ClampedArray(data);
  const [from, to] = toSlim ? [4, 3] : [3, 4];
  for (const [u, v] of [[40, 16], [40, 32], [32, 48], [48, 48]]) convertArm(data, out, u, v, from, to);
  return out;
}
