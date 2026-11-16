import { useEffect, useRef } from "react";
import { SkinViewer, WalkingAnimation, IdleAnimation } from "skinview3d";

/** Rotatable 3D player model. `skin` and `cape` are image URLs (data: URLs work best). */
export function SkinView({
  skin,
  cape,
  slim,
  animate = true,
  width = 300,
  height = 400,
}: {
  skin: string | null;
  cape?: string | null;
  slim?: boolean;
  animate?: boolean;
  width?: number;
  height?: number;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const viewer = useRef<SkinViewer | null>(null);

  useEffect(() => {
    if (!canvas.current) return;
    const v = new SkinViewer({ canvas: canvas.current, width, height, zoom: 0.85 });
    v.autoRotate = false;
    v.controls.enablePan = false;
    viewer.current = v;
    return () => {
      v.dispose();
      viewer.current = null;
    };
  }, [width, height]);

  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    if (skin) void v.loadSkin(skin, { model: slim ? "slim" : "default" });
    else v.loadSkin(null);
  }, [skin, slim, width, height]);

  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    if (cape) void v.loadCape(cape);
    else v.loadCape(null);
  }, [cape, width, height]);

  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    v.animation = animate ? new WalkingAnimation() : new IdleAnimation();
    if (v.animation) v.animation.speed = animate ? 0.6 : 1;
  }, [animate, width, height]);

  return <canvas ref={canvas} className="skin-view" />;
}

/** Draws the front of a skin (16×32 "pixels") at (x, y) with the given scale. */
export function drawSkinFront(g: CanvasRenderingContext2D, img: HTMLImageElement, x: number, y: number, s: number, slim?: boolean) {
  g.imageSmoothingEnabled = false;
  const tall = img.height >= 64;
  const aw = slim ? 3 : 4;
  const part = (sx: number, sy: number, w: number, h: number, dx: number, dy: number, flip = false) => {
    g.save();
    if (flip) {
      g.translate(x + (dx + w) * s, y + dy * s);
      g.scale(-1, 1);
      g.drawImage(img, sx, sy, w, h, 0, 0, w * s, h * s);
    } else g.drawImage(img, sx, sy, w, h, x + dx * s, y + dy * s, w * s, h * s);
    g.restore();
  };
  part(8, 8, 8, 8, 4, 0);
  part(20, 20, 8, 12, 4, 8);
  part(44, 20, aw, 12, 4 - aw, 8);
  if (tall) part(36, 52, aw, 12, 12, 8);
  else part(44, 20, aw, 12, 12, 8, true);
  part(4, 20, 4, 12, 4, 20);
  if (tall) part(20, 52, 4, 12, 8, 20);
  else part(4, 20, 4, 12, 8, 20, true);
  part(40, 8, 8, 8, 4, 0);
  if (tall) {
    part(20, 36, 8, 12, 4, 8);
    part(44, 36, aw, 12, 4 - aw, 8);
    part(52, 52, aw, 12, 12, 8);
    part(4, 36, 4, 12, 4, 20);
    part(4, 52, 4, 12, 8, 20);
  }
}

/** Flat front view of a skin, drawn on a small canvas. */
export function SkinThumb({ src, slim, scale = 4 }: { src: string; slim?: boolean; scale?: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const img = new Image();
    img.onload = () => {
      const c = ref.current;
      if (!c) return;
      const g = c.getContext("2d");
      if (!g) return;
      g.imageSmoothingEnabled = false;
      g.clearRect(0, 0, c.width, c.height);
      const s = scale;
      const tall = img.height >= 64;
      const aw = slim ? 3 : 4;
      const part = (sx: number, sy: number, w: number, h: number, dx: number, dy: number, flip = false) => {
        g.save();
        if (flip) {
          g.translate((dx + w) * s, dy * s);
          g.scale(-1, 1);
          g.drawImage(img, sx, sy, w, h, 0, 0, w * s, h * s);
        } else {
          g.drawImage(img, sx, sy, w, h, dx * s, dy * s, w * s, h * s);
        }
        g.restore();
      };
      // base layer (front faces)
      part(8, 8, 8, 8, 4, 0);
      part(20, 20, 8, 12, 4, 8);
      part(44, 20, aw, 12, 4 - aw, 8);
      if (tall) part(36, 52, aw, 12, 12, 8);
      else part(44, 20, aw, 12, 12, 8, true);
      part(4, 20, 4, 12, 4, 20);
      if (tall) part(20, 52, 4, 12, 8, 20);
      else part(4, 20, 4, 12, 8, 20, true);
      // outer layer
      part(40, 8, 8, 8, 4, 0);
      if (tall) {
        part(20, 36, 8, 12, 4, 8);
        part(44, 36, aw, 12, 4 - aw, 8);
        part(52, 52, aw, 12, 12, 8);
        part(4, 36, 4, 12, 4, 20);
        part(4, 52, 4, 12, 8, 20);
      }
    };
    img.src = src;
  }, [src, slim, scale]);
  return <canvas ref={ref} width={16 * scale} height={32 * scale} className="skin-thumb" />;
}
