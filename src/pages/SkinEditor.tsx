import { useCallback, useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { confirm } from "../components/Confirm";
import { MOUSE, Mesh, Object3D, Raycaster, Vector2 } from "three";
import {
  CrouchAnimation,
  FlyingAnimation,
  IdleAnimation,
  RunningAnimation,
  SkinViewer,
  WalkingAnimation,
  WaveAnimation,
  type PlayerAnimation,
} from "skinview3d";
import { api, errText, type SavedSkin } from "../api";
import { useStore } from "../store";
import { Toggle } from "../components/Bits";
import { Icon } from "../components/Icon";
import { sfx } from "../sounds";
import type { Key } from "../i18n";
import {
  SIZE,
  boxAt,
  brushPixels,
  convertArms,
  floodFill,
  getPx,
  hexToRgba,
  linePixels,
  mirrorPixel,
  noisy,
  partNames,
  rectPixels,
  rgbaToHex,
  same,
  setPx,
  shade,
  type PartId,
} from "../skinTools";

type Tool = "pencil" | "eraser" | "line" | "rect" | "fill" | "replace" | "shade" | "noise" | "picker";
type Target = "auto" | "base" | "overlay";
type Pose = "none" | "idle" | "walk" | "run" | "wave" | "crouch" | "fly";

const toolList: Array<{ id: Tool; icon: string; key: string }> = [
  { id: "pencil", icon: "pencil", key: "b" },
  { id: "eraser", icon: "eraser", key: "e" },
  { id: "line", icon: "line", key: "l" },
  { id: "rect", icon: "square", key: "u" },
  { id: "fill", icon: "bucket", key: "g" },
  { id: "replace", icon: "swap", key: "r" },
  { id: "shade", icon: "sun", key: "d" },
  { id: "noise", icon: "sparkle", key: "n" },
  { id: "picker", icon: "picker", key: "i" },
];
const parts: PartId[] = ["head", "body", "rightArm", "leftArm", "rightLeg", "leftLeg"];
const poses: Pose[] = ["none", "idle", "walk", "run", "wave", "crouch", "fly"];
const PALETTE_KEY = "anilauncher.palette";

/** A plain original starter character (not any official default skin). */
function starter(): Uint8ClampedArray {
  const d = new Uint8ClampedArray(SIZE * SIZE * 4);
  const rect = (x: number, y: number, w: number, h: number, hex: string) => {
    const c = hexToRgba(hex);
    for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) setPx(d, i, j, c);
  };
  const skin = "#e0ac7e", hair = "#f2c94c", shirt = "#7c3aed", pants = "#d4d4d8", shoe = "#dc2626";
  rect(8, 0, 16, 8, skin); rect(0, 8, 32, 8, skin); rect(8, 0, 8, 8, hair); rect(0, 8, 32, 2, hair);
  rect(9, 12, 2, 1, "#ffffff"); rect(13, 12, 2, 1, "#ffffff"); rect(10, 12, 1, 1, "#15803d"); rect(13, 12, 1, 1, "#15803d");
  rect(11, 14, 2, 1, "#8a5a3b");
  rect(20, 16, 16, 4, shirt); rect(16, 20, 24, 12, shirt);
  rect(44, 16, 8, 4, skin); rect(40, 20, 16, 12, skin); rect(40, 20, 16, 4, shirt);
  rect(36, 48, 8, 4, skin); rect(32, 52, 16, 12, skin); rect(32, 52, 16, 4, shirt);
  rect(4, 16, 8, 4, pants); rect(0, 20, 16, 12, pants); rect(0, 30, 16, 2, shoe);
  rect(20, 48, 8, 4, pants); rect(16, 52, 16, 12, pants); rect(16, 62, 16, 2, shoe);
  return d;
}

function makeAnimation(p: Pose): PlayerAnimation | null {
  switch (p) {
    case "idle": return new IdleAnimation();
    case "walk": { const a = new WalkingAnimation(); a.speed = 0.6; return a; }
    case "run": { const a = new RunningAnimation(); a.speed = 0.6; return a; }
    case "wave": return new WaveAnimation();
    case "crouch": return new CrouchAnimation();
    case "fly": return new FlyingAnimation();
    default: return null;
  }
}

function toDataUrl(d: Uint8ClampedArray): string {
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = SIZE;
  c.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(d), SIZE, SIZE), 0, 0);
  return c.toDataURL("image/png");
}

async function fromDataUrl(url: string): Promise<Uint8ClampedArray> {
  const img = new Image();
  await new Promise<void>((res, rej) => {
    img.onload = () => res();
    img.onerror = () => rej(new Error("image"));
    img.src = url;
  });
  const c = document.createElement("canvas");
  c.width = SIZE;
  c.height = SIZE;
  const g = c.getContext("2d")!;
  g.drawImage(img, 0, 0);
  if (img.height === 32) {
    // legacy 64×32: copy right limbs to the left
    g.drawImage(img, 0, 16, 16, 16, 16, 48, 16, 16);
    g.drawImage(img, 40, 16, 16, 16, 32, 48, 16, 16);
  }
  return g.getImageData(0, 0, SIZE, SIZE).data;
}

export function SkinEditor({ skin, onClose, onSaved }: { skin: SavedSkin | null; onClose: () => void; onSaved: () => void }) {
  const { t, toast, settings } = useStore();
  const lang = settings.lang;
  const data = useRef<Uint8ClampedArray>(starter());
  const undo = useRef<Uint8ClampedArray[]>([]);
  const redo = useRef<Uint8ClampedArray[]>([]);
  const [rev, setRev] = useState(0); // bumps on every change
  const [dirty, setDirty] = useState(false);

  const [tool, setTool] = useState<Tool>("pencil");
  const [primary, setPrimary] = useState("#7c3aed");
  const [secondary, setSecondary] = useState("#ffffff");
  const [brush, setBrush] = useState(1);
  const [mirror, setMirror] = useState(false);
  const [target, setTarget] = useState<Target>("auto");
  const [scope, setScope] = useState<"part" | "all">("part");
  const [lighten, setLighten] = useState(false);
  const [strength, setStrength] = useState(0.15);
  const [filled, setFilled] = useState(false);
  const [palette, setPalette] = useState<string[]>(() => {
    try {
      const raw = localStorage.getItem(PALETTE_KEY);
      if (raw) return JSON.parse(raw) as string[];
    } catch {
      /* no storage */
    }
    return ["#000000", "#ffffff", "#e0ac7e", "#c68642", "#8d5524", "#f2c94c", "#7c3aed", "#2563eb", "#16a34a", "#dc2626", "#facc15", "#374151"];
  });
  const [recent, setRecent] = useState<string[]>([]);

  const [slim, setSlim] = useState(skin?.variant === "slim");
  const [name, setName] = useState(skin?.name ?? (lang === "ru" ? "Мой скин" : "My skin"));
  const [showBase, setShowBase] = useState(true);
  const [showOverlay, setShowOverlay] = useState(true);
  const [hidden, setHidden] = useState<Record<PartId, boolean>>({ head: false, body: false, rightArm: false, leftArm: false, rightLeg: false, leftLeg: false });
  const [pose, setPose] = useState<Pose>("none");
  const [grid, setGrid] = useState(true);
  const [hover, setHover] = useState<[number, number] | null>(null);
  const [busy, setBusy] = useState(false);

  const view3d = useRef<HTMLCanvasElement>(null);
  const holder3d = useRef<HTMLDivElement>(null);
  const viewer = useRef<SkinViewer | null>(null);
  const tex2d = useRef<HTMLCanvasElement>(null);
  const shapeStart = useRef<[number, number] | null>(null);
  const preview = useRef<Array<[number, number]>>([]);
  const strokeDone = useRef<Set<number>>(new Set());
  const painting = useRef(false);
  const button = useRef(0);

  const bump = () => setRev((r) => r + 1);

  useEffect(() => {
    try {
      localStorage.setItem(PALETTE_KEY, JSON.stringify(palette));
    } catch {
      /* no storage */
    }
  }, [palette]);

  // ---------- loading ----------
  useEffect(() => {
    if (!skin) return;
    api
      .textureData(skin.file)
      .then(fromDataUrl)
      .then((d) => {
        data.current = d;
        undo.current = [];
        redo.current = [];
        bump();
      })
      .catch((e) => toast(errText(e), "error"));
  }, [skin, toast]);

  // ---------- 3D viewer ----------
  useEffect(() => {
    const canvas = view3d.current, holder = holder3d.current;
    if (!canvas || !holder) return;
    const v = new SkinViewer({ canvas, width: holder.clientWidth, height: holder.clientHeight, zoom: 0.72 });
    v.controls.mouseButtons = { LEFT: -1 as unknown as MOUSE, MIDDLE: MOUSE.DOLLY, RIGHT: MOUSE.ROTATE };
    v.controls.enablePan = false;
    v.fov = 50;
    viewer.current = v;
    void v.loadSkin(toDataUrl(data.current), { model: slim ? "slim" : "default" });
    const ro = new ResizeObserver(() => v.setSize(holder.clientWidth, holder.clientHeight));
    ro.observe(holder);
    return () => {
      ro.disconnect();
      v.dispose();
      viewer.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // push texture changes into the 3D model without recreating it
  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    const ctx = v.skinCanvas.getContext("2d");
    if (!ctx || v.skinCanvas.width !== SIZE) {
      void v.loadSkin(toDataUrl(data.current), { model: slim ? "slim" : "default" });
      return;
    }
    ctx.clearRect(0, 0, SIZE, SIZE);
    ctx.putImageData(new ImageData(new Uint8ClampedArray(data.current), SIZE, SIZE), 0, 0);
    const tex = (v as unknown as { skinTexture: { needsUpdate: boolean } | null }).skinTexture;
    if (tex) tex.needsUpdate = true;
  }, [rev, slim]);

  useEffect(() => {
    const v = viewer.current;
    if (v) v.playerObject.skin.modelType = slim ? "slim" : "default";
  }, [slim]);

  useEffect(() => {
    const v = viewer.current;
    if (!v) return;
    const s = v.playerObject.skin;
    for (const p of parts) {
      s[p].visible = !hidden[p];
      s[p].innerLayer.visible = showBase;
      s[p].outerLayer.visible = showOverlay;
    }
  }, [hidden, showBase, showOverlay]);

  useEffect(() => {
    const v = viewer.current;
    if (v) v.animation = makeAnimation(pose);
  }, [pose]);

  // ---------- 2D texture view ----------
  useEffect(() => {
    const c = tex2d.current;
    if (!c) return;
    const g = c.getContext("2d")!;
    const z = c.width / SIZE;
    for (let y = 0; y < SIZE; y++)
      for (let x = 0; x < SIZE; x++) {
        g.fillStyle = (x + y) % 2 ? "#262626" : "#2f2f2f";
        g.fillRect(x * z, y * z, z, z);
      }
    const tmp = document.createElement("canvas");
    tmp.width = SIZE;
    tmp.height = SIZE;
    tmp.getContext("2d")!.putImageData(new ImageData(new Uint8ClampedArray(data.current), SIZE, SIZE), 0, 0);
    g.imageSmoothingEnabled = false;
    g.drawImage(tmp, 0, 0, c.width, c.height);
    if (grid) {
      g.strokeStyle = "rgba(255,255,255,.07)";
      g.beginPath();
      for (let i = 0; i <= SIZE; i++) {
        g.moveTo(i * z + 0.5, 0);
        g.lineTo(i * z + 0.5, c.height);
        g.moveTo(0, i * z + 0.5);
        g.lineTo(c.width, i * z + 0.5);
      }
      g.stroke();
      // part outlines
      g.strokeStyle = "rgba(250,204,21,.35)";
      for (const [x, y, w, h] of [[0, 0, 32, 16], [32, 0, 32, 16], [0, 16, 16, 16], [16, 16, 24, 16], [40, 16, 16, 16], [0, 32, 16, 16], [16, 32, 24, 16], [40, 32, 16, 16], [0, 48, 16, 16], [16, 48, 16, 16], [32, 48, 16, 16], [48, 48, 16, 16]])
        g.strokeRect(x * z + 0.5, y * z + 0.5, w * z - 1, h * z - 1);
    }
    g.fillStyle = "rgba(255,255,255,.35)";
    for (const [x, y] of preview.current) g.fillRect(x * z, y * z, z, z);
    if (hover) {
      g.strokeStyle = "#ffffff";
      g.lineWidth = 2;
      g.strokeRect(hover[0] * z + 1, hover[1] * z + 1, z - 2, z - 2);
      g.lineWidth = 1;
    }
  }, [rev, grid, hover]);

  // ---------- painting core ----------
  const snapshot = () => {
    undo.current.push(new Uint8ClampedArray(data.current));
    if (undo.current.length > 150) undo.current.shift();
    redo.current = [];
    setDirty(true);
  };

  const remember = (c: string) => setRecent((r) => [c, ...r.filter((x) => x !== c)].slice(0, 10));

  const withMirror = useCallback(
    (pts: Array<[number, number]>) => {
      if (!mirror) return pts;
      const out = [...pts];
      for (const [x, y] of pts) {
        const m = mirrorPixel(x, y, slim);
        if (m) out.push(m);
      }
      return out;
    },
    [mirror, slim],
  );

  /** Applies the current tool at pixels (already expanded by brush). */
  const apply = (pts: Array<[number, number]>, colorHex: string) => {
    const d = data.current;
    const color = hexToRgba(colorHex);
    for (const [x, y] of withMirror(pts)) {
      const k = y * SIZE + x;
      if (tool === "eraser") setPx(d, x, y, [0, 0, 0, 0]);
      else if (tool === "shade") {
        if (strokeDone.current.has(k)) continue;
        strokeDone.current.add(k);
        setPx(d, x, y, shade(getPx(d, x, y), strength, lighten));
      } else if (tool === "noise") {
        if (strokeDone.current.has(k)) continue;
        strokeDone.current.add(k);
        setPx(d, x, y, noisy(color, strength));
      } else setPx(d, x, y, color);
    }
    bump();
  };

  const pick = (x: number, y: number) => {
    const c = getPx(data.current, x, y);
    if (c[3] === 0) return;
    const hex = rgbaToHex(c);
    setPrimary(hex);
    remember(hex);
    sfx.click();
  };

  /** Click/press at a texture pixel (from 2D or 3D). */
  const pressAt = (x: number, y: number, alt: boolean, colorHex: string) => {
    if (alt || tool === "picker") {
      pick(x, y);
      return;
    }
    if (tool === "fill") {
      snapshot();
      const color = hexToRgba(colorHex);
      const pts = floodFill(data.current, x, y, color, scope, slim);
      const m = mirror ? mirrorPixel(x, y, slim) : null;
      if (m) pts.push(...floodFill(data.current, m[0], m[1], color, scope, slim));
      for (const [px, py] of pts) setPx(data.current, px, py, color);
      remember(colorHex);
      bump();
      return;
    }
    if (tool === "replace") {
      snapshot();
      const from = getPx(data.current, x, y);
      const to = hexToRgba(colorHex);
      for (let j = 0; j < SIZE; j++) for (let i = 0; i < SIZE; i++) if (same(getPx(data.current, i, j), from)) setPx(data.current, i, j, to);
      remember(colorHex);
      bump();
      return;
    }
    snapshot();
    strokeDone.current = new Set();
    painting.current = true;
    if (tool === "pencil" || tool === "noise") remember(colorHex);
    apply(brushPixels(x, y, brush, slim), colorHex);
  };

  const dragAt = (x: number, y: number, colorHex: string) => {
    if (!painting.current) return;
    apply(brushPixels(x, y, brush, slim), colorHex);
  };

  // ---------- 3D picking ----------
  const raycaster = useRef(new Raycaster());
  const hitPixel = (e: React.PointerEvent<HTMLCanvasElement>): [number, number] | null => {
    const v = viewer.current;
    if (!v) return null;
    const r = e.currentTarget.getBoundingClientRect();
    const ndc = new Vector2(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
    raycaster.current.setFromCamera(ndc, v.camera);
    const meshes: Mesh[] = [];
    const visible = (o: Object3D | null): boolean => {
      for (let c = o; c; c = c.parent) if (!c.visible) return false;
      return true;
    };
    for (const p of parts) {
      const part = v.playerObject.skin[p];
      const tag = (root: Object3D, layer: string) =>
        root.traverse((o) => {
          if ((o as Mesh).isMesh) {
            o.userData.layer = layer;
            if (visible(o)) meshes.push(o as Mesh);
          }
        });
      tag(part.innerLayer, "base");
      tag(part.outerLayer, "overlay");
    }
    const hits = raycaster.current.intersectObjects(meshes, false);
    for (const h of hits) {
      if (!h.uv) continue;
      const layer = h.object.userData.layer as string;
      if (target !== "auto" && layer !== target) continue;
      const fx = h.uv.x * SIZE, fy = (1 - h.uv.y) * SIZE;
      const x = Math.min(SIZE - 1, Math.floor(fx)), y = Math.min(SIZE - 1, Math.floor(fy));
      if (target === "auto" && layer === "overlay" && getPx(data.current, x, y)[3] === 0 && tool !== "picker") continue;
      return [x, y];
    }
    return null;
  };

  // ---------- 2D pointer ----------
  const at2d = (e: React.PointerEvent<HTMLCanvasElement>): [number, number] | null => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = Math.floor(((e.clientX - r.left) / r.width) * SIZE);
    const y = Math.floor(((e.clientY - r.top) / r.height) * SIZE);
    return x >= 0 && y >= 0 && x < SIZE && y < SIZE ? [x, y] : null;
  };

  const colorFor = (btn: number) => (btn === 2 ? secondary : primary);

  // ---------- history & actions ----------
  const doUndo = () => {
    const prev = undo.current.pop();
    if (!prev) return;
    redo.current.push(new Uint8ClampedArray(data.current));
    data.current = prev;
    bump();
  };
  const doRedo = () => {
    const next = redo.current.pop();
    if (!next) return;
    undo.current.push(new Uint8ClampedArray(data.current));
    data.current = next;
    bump();
  };

  const doSave = async () => {
    setBusy(true);
    try {
      await api.saveSkinData(skin?.id ?? null, name.trim() || "Skin", slim ? "slim" : "classic", toDataUrl(data.current));
      sfx.success();
      toast(t("editor.saved"), "ok");
      setDirty(false);
      onSaved();
    } catch (e) {
      toast(errText(e), "error");
    } finally {
      setBusy(false);
    }
  };

  const doClose = async () => {
    if (dirty && !(await confirm(t("pe.unsaved"), { title: "AniLauncher", kind: "warning" }))) return;
    onClose();
  };

  const doImport = async () => {
    try {
      const file = await open({ multiple: false, filters: [{ name: "PNG", extensions: ["png"] }] });
      if (typeof file !== "string") return;
      const d = await fromDataUrl(await api.readPng(file));
      snapshot();
      data.current = d;
      bump();
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const doExport = async () => {
    try {
      const dest = await save({ defaultPath: `${name || "skin"}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
      if (!dest) return;
      await api.writePng(dest, toDataUrl(data.current));
      toast(t("pe.exported"), "ok");
    } catch (e) {
      toast(errText(e), "error");
    }
  };

  const clearOverlay = () => {
    snapshot();
    const d = data.current;
    for (const [x, y, w, h] of [[32, 0, 32, 16], [0, 32, 56, 16], [0, 48, 16, 16], [48, 48, 16, 16]])
      for (let j = y; j < y + h; j++) for (let i = x; i < x + w; i++) setPx(d, i, j, [0, 0, 0, 0]);
    bump();
  };

  // ---------- keyboard ----------
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      if (el.tagName === "INPUT" || el.tagName === "TEXTAREA") return;
      const k = e.key.toLowerCase();
      if (e.ctrlKey || e.metaKey) {
        if (k === "z") {
          e.preventDefault();
          if (e.shiftKey) doRedo();
          else doUndo();
        } else if (k === "y") {
          e.preventDefault();
          doRedo();
        } else if (k === "s") {
          e.preventDefault();
          void doSave();
        }
        return;
      }
      const tl = toolList.find((x) => x.key === k);
      if (tl) setTool(tl.id);
      else if (k === "x") {
        setPrimary(secondary);
        setSecondary(primary);
      } else if (k === "m") setMirror((m) => !m);
      else if (k === "[") setBrush((b) => Math.max(1, b - 1));
      else if (k === "]") setBrush((b) => Math.min(4, b + 1));
      else if (k === "escape") void doClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const hoverPart = hover ? boxAt(hover[0], hover[1], slim) : null;
  const shapeTool = tool === "line" || tool === "rect";

  return (
    <div className="pe" role="dialog" aria-modal="true" aria-label={t("editor.title")}>
      <header className="pe__top">
        <div className="pe__title">
          <Icon name="palette" size={20} />
          <input className="pe__name" value={name} maxLength={40} onChange={(e) => setName(e.target.value)} aria-label={t("skins.name")} />
          <div className="seg seg--panel">
            <button className={`seg__btn ${!slim ? "seg__btn--on" : ""}`} onClick={() => setSlim(false)}>{t("skins.classic")}</button>
            <button className={`seg__btn ${slim ? "seg__btn--on" : ""}`} onClick={() => setSlim(true)}>{t("skins.slim")}</button>
          </div>
        </div>
        <div className="pe__actions">
          <button className="icon-btn" title={`${t("editor.undo")} (Ctrl+Z)`} aria-label={t("editor.undo")} onClick={doUndo} disabled={!undo.current.length}><Icon name="undo" size={18} /></button>
          <button className="icon-btn" title={`${t("editor.redo")} (Ctrl+Y)`} aria-label={t("editor.redo")} onClick={doRedo} disabled={!redo.current.length}><Icon name="redo" size={18} /></button>
          <span className="muted small">{t("pe.history", { n: undo.current.length })}</span>
          <button className="btn btn--small" onClick={doImport}><Icon name="download" size={16} /> {t("pe.import")}</button>
          <button className="btn btn--small" onClick={doExport}><Icon name="upload" size={16} /> {t("pe.export")}</button>
          <button className="btn btn--primary btn--small" disabled={busy} onClick={() => void doSave()}>{t("editor.save")}</button>
          <button className="icon-btn" aria-label={t("pe.close")} title={t("pe.close")} onClick={() => void doClose()}><Icon name="close" /></button>
        </div>
      </header>

      <div className="pe__body">
        <aside className="pe__tools" aria-label={t("pe.tools")}>
          {toolList.map((x) => (
            <button
              key={x.id}
              className={`tool ${tool === x.id ? "tool--on" : ""}`}
              title={t(`pe.tool.${x.id}` as Key)}
              aria-label={t(`pe.tool.${x.id}` as Key)}
              aria-pressed={tool === x.id}
              onClick={() => {
                sfx.click();
                setTool(x.id);
              }}
            >
              <Icon name={x.icon} size={18} />
            </button>
          ))}
          <span className="editor__sep" />
          <button className={`tool ${mirror ? "tool--on" : ""}`} title={t("pe.mirror")} aria-label={t("pe.mirror")} aria-pressed={mirror} onClick={() => setMirror((m) => !m)}>
            <Icon name="mirror" size={18} />
          </button>
          <div className="pe__colors">
            <label className="pe__chip pe__chip--primary" title={t("pe.primary")} style={{ background: primary }}>
              <input type="color" value={primary} onChange={(e) => setPrimary(e.target.value)} aria-label={t("pe.primary")} />
            </label>
            <label className="pe__chip pe__chip--secondary" title={t("pe.secondary")} style={{ background: secondary }}>
              <input type="color" value={secondary} onChange={(e) => setSecondary(e.target.value)} aria-label={t("pe.secondary")} />
            </label>
            <button className="pe__swap" title={t("pe.swap")} aria-label={t("pe.swap")} onClick={() => { setPrimary(secondary); setSecondary(primary); }}>
              <Icon name="swap" size={14} />
            </button>
          </div>
        </aside>

        <section className="pe__stage">
          <div className="pe__viewport" ref={holder3d}>
            <canvas
              ref={view3d}
              className="pe__canvas3d"
              style={{ cursor: tool === "picker" ? "copy" : "crosshair" }}
              onContextMenu={(e) => e.preventDefault()}
              onPointerDown={(e) => {
                if (e.button !== 0) return;
                const px = hitPixel(e);
                if (!px) return;
                e.currentTarget.setPointerCapture(e.pointerId);
                button.current = 0;
                pressAt(px[0], px[1], e.altKey, primary);
                setHover(px);
              }}
              onPointerMove={(e) => {
                const px = hitPixel(e);
                setHover(px);
                if (px && painting.current && !shapeTool) dragAt(px[0], px[1], primary);
              }}
              onPointerUp={() => (painting.current = false)}
              onPointerLeave={() => setHover(null)}
            />
            <div className="pe__overlay pe__overlay--top">
              <label className="pe__mini">
                {t("pe.pose")}
                <select value={pose} onChange={(e) => setPose(e.target.value as Pose)}>
                  {poses.map((p) => <option key={p} value={p}>{t(`pe.pose.${p}` as Key)}</option>)}
                </select>
              </label>
              <button className="ghost" onClick={() => viewer.current?.resetCameraPose()}>{t("pe.resetView")}</button>
            </div>
            <div className="pe__overlay pe__overlay--bottom">
              <span>{t("pe.hint3d")}</span>
              {hover && (
                <span>
                  {t("pe.pixel", { x: hover[0], y: hover[1] })}
                  {hoverPart ? ` · ${partNames[hoverPart.box.part][lang]} · ${hoverPart.box.layer === "base" ? t("pe.showBase") : t("pe.showOverlay")}` : ""}
                </span>
              )}
            </div>
            {shapeTool && <div className="pe__overlay pe__overlay--center">{lang === "ru" ? "Линия и прямоугольник рисуются на текстуре справа" : "Lines and rectangles are drawn on the texture on the right"}</div>}
          </div>
        </section>

        <aside className="pe__panel">
          <div className="pe__block">
            <div className="pe__block-head">
              <strong>{t("pe.texture")}</strong>
              <label className="check-inline">{t("pe.grid")}<Toggle on={grid} onChange={setGrid} label={t("pe.grid")} /></label>
            </div>
            <canvas
              ref={tex2d}
              width={SIZE * 5}
              height={SIZE * 5}
              className="pe__tex"
              style={{ cursor: tool === "picker" ? "copy" : "crosshair" }}
              onContextMenu={(e) => e.preventDefault()}
              onPointerDown={(e) => {
                const p = at2d(e);
                if (!p) return;
                e.currentTarget.setPointerCapture(e.pointerId);
                button.current = e.button;
                if (shapeTool && !e.altKey) {
                  shapeStart.current = p;
                  preview.current = [p];
                  bump();
                  return;
                }
                pressAt(p[0], p[1], e.altKey, colorFor(e.button));
              }}
              onPointerMove={(e) => {
                const p = at2d(e);
                setHover(p);
                if (!p) return;
                if (shapeStart.current) {
                  const [sx, sy] = shapeStart.current;
                  preview.current = withMirror(tool === "line" ? linePixels(sx, sy, p[0], p[1]) : rectPixels(sx, sy, p[0], p[1], filled));
                  bump();
                } else dragAt(p[0], p[1], colorFor(button.current));
              }}
              onPointerUp={(e) => {
                painting.current = false;
                if (shapeStart.current) {
                  const p = at2d(e) ?? shapeStart.current;
                  const [sx, sy] = shapeStart.current;
                  shapeStart.current = null;
                  preview.current = [];
                  snapshot();
                  const pts = tool === "line" ? linePixels(sx, sy, p[0], p[1]) : rectPixels(sx, sy, p[0], p[1], filled);
                  const c = colorFor(button.current);
                  remember(c);
                  const color = hexToRgba(c);
                  for (const [x, y] of withMirror(pts)) setPx(data.current, x, y, color);
                  bump();
                }
              }}
              onPointerLeave={() => setHover(null)}
            />
          </div>

          <div className="pe__block">
            <strong>{t("pe.options")}</strong>
            {(tool === "pencil" || tool === "eraser" || tool === "shade" || tool === "noise") && (
              <label className="field">
                <span>{t("pe.size")}: {brush}px</span>
                <input type="range" min={1} max={4} value={brush} onChange={(e) => setBrush(Number(e.target.value))} />
              </label>
            )}
            {tool === "fill" && (
              <div className="seg seg--panel seg--full">
                {(["part", "all"] as const).map((s) => (
                  <button key={s} className={`seg__btn ${scope === s ? "seg__btn--on" : ""}`} onClick={() => setScope(s)}>{t(`pe.scope.${s}` as Key)}</button>
                ))}
              </div>
            )}
            {tool === "shade" && (
              <div className="seg seg--panel seg--full">
                <button className={`seg__btn ${!lighten ? "seg__btn--on" : ""}`} onClick={() => setLighten(false)}>{t("pe.darken")}</button>
                <button className={`seg__btn ${lighten ? "seg__btn--on" : ""}`} onClick={() => setLighten(true)}>{t("pe.lighten")}</button>
              </div>
            )}
            {(tool === "shade" || tool === "noise") && (
              <label className="field">
                <span>{t("pe.strength")}: {Math.round(strength * 100)}%</span>
                <input type="range" min={0.03} max={0.4} step={0.01} value={strength} onChange={(e) => setStrength(Number(e.target.value))} />
              </label>
            )}
            {tool === "rect" && (
              <label className="check-inline">{t("pe.filled")}<Toggle on={filled} onChange={setFilled} label={t("pe.filled")} /></label>
            )}
            <span className="small muted">{t("pe.target")}</span>
            <div className="seg seg--panel seg--full">
              {(["auto", "base", "overlay"] as const).map((x) => (
                <button key={x} className={`seg__btn ${target === x ? "seg__btn--on" : ""}`} onClick={() => setTarget(x)}>{t(`pe.target.${x}` as Key)}</button>
              ))}
            </div>
          </div>

          <div className="pe__block">
            <div className="pe__block-head">
              <strong>{t("pe.palette")}</strong>
              <button className="link small" onClick={() => setPalette((p) => (p.includes(primary) ? p : [...p, primary].slice(-32)))}>+ {t("pe.addColor")}</button>
            </div>
            <div className="pe__hex">
              <input
                key={primary}
                defaultValue={primary}
                maxLength={7}
                aria-label="HEX"
                onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
                onBlur={(e) => {
                  const v = e.target.value.trim().startsWith("#") ? e.target.value.trim() : `#${e.target.value.trim()}`;
                  if (/^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(v)) setPrimary(rgbaToHex(hexToRgba(v)));
                }}
              />
              <span className="pe__hex-chip" style={{ background: primary }} />
            </div>
            <div className="pe__swatches">
              {palette.map((c) => (
                <button
                  key={c}
                  className={`pe__swatch ${c === primary ? "pe__swatch--on" : ""}`}
                  style={{ background: c }}
                  title={c}
                  aria-label={c}
                  onClick={() => setPrimary(c)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    setPalette((p) => p.filter((x) => x !== c));
                  }}
                />
              ))}
            </div>
            {recent.length > 0 && (
              <>
                <span className="small muted">{t("pe.recent")}</span>
                <div className="pe__swatches">
                  {recent.map((c) => (
                    <button key={c} className="pe__swatch" style={{ background: c }} title={c} aria-label={c} onClick={() => setPrimary(c)} />
                  ))}
                </div>
              </>
            )}
            <span className="small muted">{t("pe.paletteHint")}</span>
          </div>

          <div className="pe__block">
            <strong>{t("pe.layers")}</strong>
            <div className="pe__toggles">
              <label className="check-inline">{t("pe.showBase")}<Toggle on={showBase} onChange={setShowBase} label={t("pe.showBase")} /></label>
              <label className="check-inline">{t("pe.showOverlay")}<Toggle on={showOverlay} onChange={setShowOverlay} label={t("pe.showOverlay")} /></label>
            </div>
            <span className="small muted">{t("pe.parts")}</span>
            <div className="pe__parts">
              {parts.map((p) => (
                <button key={p} className={`chip ${!hidden[p] ? "chip--on" : ""}`} onClick={() => setHidden((h) => ({ ...h, [p]: !h[p] }))}>
                  {partNames[p][lang]}
                </button>
              ))}
            </div>
            <div className="pe__row">
              <button className="btn btn--small" onClick={() => { snapshot(); data.current = convertArms(data.current, !slim); setSlim(!slim); bump(); }}>{t("pe.convertArms")}</button>
              <button className="btn btn--small" onClick={clearOverlay}>{t("pe.clearOverlay")}</button>
              <button className="btn btn--small" onClick={() => { snapshot(); data.current = starter(); bump(); }}>{t("pe.new")}</button>
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}
