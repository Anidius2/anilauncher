export interface Theme {
  id: string;
  name: { ru: string; en: string };
  vars: Record<string, string>;
}

const base = (
  bg: string, side: string, panel: string, line: string, text: string, muted: string,
  accent: string, accentHi: string, onAccent: string, heroTint: string,
): Record<string, string> => ({
  "--bg": bg,
  "--side": side,
  "--panel": panel,
  "--line": line,
  "--text": text,
  "--muted": muted,
  "--accent": accent,
  "--accent-hi": accentHi,
  "--on-accent": onAccent,
  "--hero-tint": heroTint,
});

export const themes: Theme[] = [
  { id: "official", name: { ru: "Официальная", en: "Official" },
    vars: base("#1b1b1b", "#242424", "#2e2e2e", "#3b3b3b", "#f2f2f2", "#b8b8b8", "#3c8527", "#6fcf4b", "#ffffff", "#2a1f3d") },
  { id: "anidius", name: { ru: "Anidius", en: "Anidius" },
    vars: base("#0a0a0a", "#131313", "#1d1d1d", "#2f2f2f", "#f5f5f5", "#bdbdbd", "#facc15", "#fde047", "#111111", "#1a1a1a") },
  { id: "nether", name: { ru: "Незер", en: "Nether" },
    vars: base("#170b0b", "#221010", "#311616", "#4b2020", "#fbeeee", "#d6b0b0", "#b8400d", "#fb923c", "#ffffff", "#3b1010") },
  { id: "end", name: { ru: "Энд", en: "The End" },
    vars: base("#0e0b16", "#161124", "#211a34", "#33294e", "#f2edfc", "#bdb1da", "#7c3aed", "#a78bfa", "#ffffff", "#1d1433") },
  { id: "deep", name: { ru: "Глубины", en: "Deep Dark" },
    vars: base("#061216", "#0a1c23", "#0f2d37", "#1b4552", "#e6f6f8", "#a3c9d0", "#0e7490", "#22d3ee", "#ffffff", "#082028") },
  { id: "meadow", name: { ru: "Луга", en: "Meadow" },
    vars: base("#f4f1e6", "#e8e3d1", "#ffffff", "#d4cdb3", "#1f2a16", "#4b5640", "#3c7a24", "#2f6b1f", "#ffffff", "#9ccf7a") },
];

import type { CustomTheme } from "./api";

export const colorVars = ["--bg", "--side", "--panel", "--line", "--text", "--muted", "--accent", "--accent-hi", "--on-accent"] as const;
export const elementVars = ["--play-bg", "--play-text", "--side-active", "--card-bg", "--card-border", "--title-color"] as const;

/** Applies a built-in or custom theme by id. */
export function applyTheme(id: string, custom: CustomTheme[] = []) {
  const root = document.documentElement;
  const own = custom.find((c) => c.id === id);
  const base = themes.find((t) => t.id === (own ? own.base : id)) ?? themes[0];
  for (const k of elementVars) root.style.removeProperty(k);
  for (const [k, v] of Object.entries(base.vars)) root.style.setProperty(k, v);
  root.style.setProperty("--radius", "12px");
  root.style.setProperty("--card-border-w", "2px");
  root.dataset.glow = "off";
  if (own) {
    for (const [k, v] of Object.entries(own.vars)) if (v) root.style.setProperty(k, v);
    root.style.setProperty("--radius", `${own.radius}px`);
    root.style.setProperty("--card-border-w", `${own.border}px`);
    root.dataset.glow = own.glow ? "on" : "off";
  }
  root.dataset.theme = base.id;
}

export interface Frame {
  id: string;
  name: { ru: string; en: string };
  css: string;
  animated?: boolean;
  /** extra animated effect drawn around the avatar */
  fx?: "fire" | "water" | "ender";
}

export const frames: Frame[] = [
  { id: "none", name: { ru: "Без рамки", en: "None" }, css: "transparent" },
  { id: "gold", name: { ru: "Золото", en: "Gold" }, css: "linear-gradient(135deg,#facc15,#f59e0b)" },
  { id: "emerald", name: { ru: "Изумруд", en: "Emerald" }, css: "linear-gradient(135deg,#6ee7b7,#059669)" },
  { id: "amethyst", name: { ru: "Аметист", en: "Amethyst" }, css: "linear-gradient(135deg,#e9d5ff,#9333ea)" },
  { id: "nether", name: { ru: "Незер", en: "Nether" }, css: "linear-gradient(135deg,#fb923c,#b91c1c)" },
  { id: "ice", name: { ru: "Лёд", en: "Ice" }, css: "linear-gradient(135deg,#e0f2fe,#38bdf8)" },
  { id: "anidius", name: { ru: "Anidius", en: "Anidius" }, css: "linear-gradient(135deg,#fde047,#ffffff,#facc15)" },
  { id: "rainbow", name: { ru: "Радуга", en: "Rainbow" }, css: "conic-gradient(from 0deg,#f43f5e,#f59e0b,#facc15,#22c55e,#06b6d4,#8b5cf6,#f43f5e)", animated: true },
  { id: "fire", name: { ru: "Огонь", en: "Fire" }, css: "linear-gradient(0deg,#7c2d12,#ea580c,#f59e0b,#fde047,#ea580c)", fx: "fire" },
  { id: "water", name: { ru: "Вода", en: "Water" }, css: "linear-gradient(90deg,#0c4a6e,#0284c7,#38bdf8,#bae6fd,#0284c7,#0c4a6e)", fx: "water" },
  { id: "ender", name: { ru: "Частицы Энда", en: "End particles" }, css: "linear-gradient(135deg,#1e1033,#581c87,#a855f7,#1e1033)", fx: "ender" },
];
