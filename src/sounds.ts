// Synthesized interface sounds in several "packs", plus rain/wind ambience. No sound files needed.

export type SoundPack = "classic" | "retro" | "soft" | "wood" | "crystal";
export const soundPacks: SoundPack[] = ["classic", "retro", "soft", "wood", "crystal"];

/** Pack a built-in theme uses unless the player picks another. */
export const themeSoundPack: Record<string, SoundPack> = {
  official: "classic",
  anidius: "retro",
  nether: "wood",
  end: "crystal",
  deep: "soft",
  meadow: "wood",
};

let ctx: AudioContext | null = null;
let enabled = true;
let pack: SoundPack = "classic";
let volume = 0.7;

let hoverOn = true;
let clickOn = true;
/** Fine control over the busiest interface sounds. */
export function setSfxFlags(hover: boolean, click: boolean) {
  hoverOn = hover;
  clickOn = click;
}
export function setSoundsEnabled(on: boolean) {
  enabled = on;
  if (!on) stopAmbient();
}
export function setSoundPack(p: SoundPack) {
  pack = p;
}
export function setVolume(v: number) {
  volume = Math.max(0, Math.min(1, v));
  if (ambientGain) ambientGain.gain.value = ambientLevel * volume;
}

function audio(): AudioContext | null {
  if (!enabled) return null;
  if (!ctx) {
    try {
      ctx = new AudioContext();
    } catch {
      return null;
    }
  }
  if (ctx.state === "suspended") void ctx.resume();
  return ctx;
}

function tone(freq: number, start: number, dur: number, type: OscillatorType, vol: number) {
  const a = audio();
  if (!a || volume === 0) return;
  const t = a.currentTime + start;
  const osc = a.createOscillator();
  const gain = a.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, t);
  gain.gain.setValueAtTime(0.0001, t);
  gain.gain.exponentialRampToValueAtTime(Math.max(0.0002, vol * volume), t + 0.008);
  gain.gain.exponentialRampToValueAtTime(0.0001, t + dur);
  osc.connect(gain).connect(a.destination);
  osc.start(t);
  osc.stop(t + dur + 0.02);
}

/** Short filtered noise burst: a wooden/keyboard click. */
function click(start: number, freq: number, vol: number, dur = 0.04) {
  const a = audio();
  if (!a || volume === 0) return;
  const t = a.currentTime + start;
  const len = Math.floor(a.sampleRate * dur);
  const buf = a.createBuffer(1, len, a.sampleRate);
  const data = buf.getChannelData(0);
  for (let i = 0; i < len; i++) data[i] = (Math.random() * 2 - 1) * (1 - i / len) ** 3;
  const src = a.createBufferSource();
  src.buffer = buf;
  const f = a.createBiquadFilter();
  f.type = "bandpass";
  f.frequency.value = freq;
  f.Q.value = 3;
  const g = a.createGain();
  g.gain.value = vol * volume;
  src.connect(f).connect(g).connect(a.destination);
  src.start(t);
}

type Sfx = "click" | "hover" | "open" | "play" | "success" | "error";

const packs: Record<SoundPack, Record<Sfx, () => void>> = {
  classic: {
    click: () => tone(660, 0, 0.07, "square", 0.035),
    hover: () => tone(1320, 0, 0.03, "sine", 0.012),
    open: () => { tone(523, 0, 0.08, "triangle", 0.05); tone(784, 0.05, 0.1, "triangle", 0.045); },
    play: () => [523, 659, 784, 1047].forEach((f, i) => tone(f, i * 0.07, 0.16, "square", 0.04)),
    success: () => { tone(784, 0, 0.1, "triangle", 0.05); tone(1175, 0.08, 0.18, "triangle", 0.05); },
    error: () => { tone(220, 0, 0.12, "sawtooth", 0.035); tone(165, 0.09, 0.16, "sawtooth", 0.03); },
  },
  retro: {
    click: () => { tone(988, 0, 0.04, "square", 0.04); tone(1319, 0.035, 0.04, "square", 0.03); },
    hover: () => tone(1976, 0, 0.025, "square", 0.01),
    open: () => [659, 880, 1109].forEach((f, i) => tone(f, i * 0.045, 0.06, "square", 0.04)),
    play: () => [392, 523, 659, 784, 1047, 1319].forEach((f, i) => tone(f, i * 0.06, 0.12, "square", 0.045)),
    success: () => [1047, 1319, 1568, 2093].forEach((f, i) => tone(f, i * 0.05, 0.09, "square", 0.04)),
    error: () => [330, 262, 196].forEach((f, i) => tone(f, i * 0.08, 0.12, "square", 0.045)),
  },
  soft: {
    click: () => tone(880, 0, 0.12, "sine", 0.04),
    hover: () => tone(1760, 0, 0.06, "sine", 0.008),
    open: () => { tone(587, 0, 0.25, "sine", 0.04); tone(880, 0.06, 0.3, "sine", 0.03); },
    play: () => [440, 554, 659, 880].forEach((f, i) => tone(f, i * 0.09, 0.5, "sine", 0.04)),
    success: () => { tone(659, 0, 0.3, "sine", 0.045); tone(988, 0.1, 0.45, "sine", 0.04); },
    error: () => { tone(294, 0, 0.3, "sine", 0.05); tone(247, 0.12, 0.35, "sine", 0.045); },
  },
  wood: {
    click: () => click(0, 1800, 0.5),
    hover: () => click(0, 3200, 0.08, 0.02),
    open: () => { click(0, 1200, 0.45); click(0.06, 1700, 0.35); },
    play: () => { [1000, 1300, 1700].forEach((f, i) => click(i * 0.07, f, 0.5)); tone(523, 0.2, 0.25, "triangle", 0.04); },
    success: () => { click(0, 2000, 0.4); tone(784, 0.05, 0.2, "triangle", 0.04); },
    error: () => { click(0, 500, 0.6, 0.08); click(0.1, 400, 0.5, 0.08); },
  },
  crystal: {
    click: () => { tone(1568, 0, 0.18, "sine", 0.03); tone(2349, 0.01, 0.15, "sine", 0.015); },
    hover: () => tone(3136, 0, 0.08, "sine", 0.006),
    open: () => [1175, 1568, 2093].forEach((f, i) => tone(f, i * 0.05, 0.35, "sine", 0.025)),
    play: () => [784, 1175, 1568, 2349, 3136].forEach((f, i) => tone(f, i * 0.07, 0.6, "sine", 0.03)),
    success: () => [1568, 2093, 2637].forEach((f, i) => tone(f, i * 0.06, 0.5, "sine", 0.03)),
    error: () => { tone(466, 0, 0.35, "sine", 0.04); tone(440, 0.05, 0.4, "triangle", 0.03); },
  },
};

export const sfx: Record<Sfx, () => void> = {
  click: () => clickOn && packs[pack].click(),
  hover: () => hoverOn && packs[pack].hover(),
  open: () => packs[pack].open(),
  play: () => packs[pack].play(),
  success: () => packs[pack].success(),
  error: () => packs[pack].error(),
};

export function previewPack(p: SoundPack) {
  const prev = pack;
  pack = p;
  packs[p].click();
  setTimeout(() => packs[p].success(), 180);
  pack = prev;
}

// ---------- ambience (rain / wind) ----------

let ambientSrc: AudioBufferSourceNode | null = null;
let ambientGain: GainNode | null = null;
let ambientKind: string | null = null;
let ambientLevel = 0;

export function startAmbient(kind: "rain" | "wind" | null) {
  if (kind === ambientKind) return;
  stopAmbient();
  if (!kind) return;
  const a = audio();
  if (!a) return;
  const len = a.sampleRate * 4;
  const buf = a.createBuffer(1, len, a.sampleRate);
  const d = buf.getChannelData(0);
  let last = 0;
  for (let i = 0; i < len; i++) {
    const white = Math.random() * 2 - 1;
    // brown-ish noise for wind, lighter for rain
    last = kind === "wind" ? (last + 0.02 * white) / 1.02 : (last + 0.25 * white) / 1.25;
    d[i] = last * (kind === "wind" ? 3.5 : 1.6);
  }
  const src = a.createBufferSource();
  src.buffer = buf;
  src.loop = true;
  const f = a.createBiquadFilter();
  f.type = kind === "wind" ? "lowpass" : "highpass";
  f.frequency.value = kind === "wind" ? 500 : 900;
  const g = a.createGain();
  ambientLevel = kind === "wind" ? 0.05 : 0.035;
  g.gain.value = 0;
  g.gain.linearRampToValueAtTime(ambientLevel * volume, a.currentTime + 2);
  src.connect(f).connect(g).connect(a.destination);
  src.start();
  ambientSrc = src;
  ambientGain = g;
  ambientKind = kind;
}

export function stopAmbient() {
  try {
    ambientSrc?.stop();
  } catch {
    /* already stopped */
  }
  ambientSrc = null;
  ambientGain = null;
  ambientKind = null;
}
