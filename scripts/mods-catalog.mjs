// Builds mods-catalog/catalog.json from the mod jars put into mods-catalog/.
// Runs on GitHub (see .github/workflows/mods-catalog.yml); can be run locally: node scripts/mods-catalog.mjs
// Reads each jar's own description file (fabric.mod.json / quilt.mod.json / mods.toml), so nothing is typed by hand.
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, statSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const dir = "mods-catalog";
const unzip = (jar, file, binary = false) => {
  try {
    return execFileSync("unzip", ["-p", jar, file], { encoding: binary ? "buffer" : "utf8", maxBuffer: 16 << 20 });
  } catch {
    return null;
  }
};
const list = (jar) => execFileSync("unzip", ["-Z1", jar], { encoding: "utf8" }).split("\n");
const toml = (text, key) => text.match(new RegExp(`^\\s*${key}\\s*=\\s*"""([\\s\\S]*?)"""|^\\s*${key}\\s*=\\s*"([^"]*)"`, "m"))?.slice(1).find(Boolean)?.trim();
const cleanMc = (v) => (Array.isArray(v) ? v[0] : v || "").replace(/^[~^>=<\s]+/, "").split(/[\s,]/)[0];
const cmp = (a, b) => {
  const pa = a.split(/[.+-]/).map((x) => parseInt(x, 10) || 0), pb = b.split(/[.+-]/).map((x) => parseInt(x, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) - (pb[i] ?? 0);
  return 0;
};

const extra = existsSync(join(dir, "descriptions.json")) ? JSON.parse(readFileSync(join(dir, "descriptions.json"), "utf8")) : {};
const mods = new Map();
mkdirSync(join(dir, "icons"), { recursive: true });
for (const file of readdirSync(dir).filter((f) => f.endsWith(".jar")).sort()) {
  const jar = join(dir, file);
  const names = list(jar);
  let info = null;
  const fabric = unzip(jar, "fabric.mod.json");
  const quilt = names.includes("quilt.mod.json") ? unzip(jar, "quilt.mod.json") : null;
  const forge = names.includes("META-INF/neoforge.mods.toml") ? ["neoforge", unzip(jar, "META-INF/neoforge.mods.toml")] : names.includes("META-INF/mods.toml") ? ["forge", unzip(jar, "META-INF/mods.toml")] : null;
  if (fabric) {
    const j = JSON.parse(fabric.replace(/^﻿/, ""));
    info = { id: j.id, name: j.name, description: j.description, author: (j.authors ?? []).map((a) => (typeof a === "string" ? a : a.name)).join(", "), icon: typeof j.icon === "string" ? j.icon : null, version: j.version, mc: cleanMc(j.depends?.minecraft), loaders: ["fabric", "quilt"], deps: Object.keys(j.depends ?? {}).filter((d) => d === "fabric-api" || d === "fabric") };
  } else if (quilt) {
    const q = JSON.parse(quilt).quilt_loader;
    info = { id: q.id, name: q.metadata?.name, description: q.metadata?.description, author: Object.keys(q.metadata?.contributors ?? {}).join(", "), icon: q.metadata?.icon, version: q.version, mc: "", loaders: ["quilt"], deps: [] };
  } else if (forge) {
    const t = forge[1];
    info = { id: toml(t, "modId"), name: toml(t, "displayName"), description: toml(t, "description"), author: toml(t, "authors") ?? "", icon: toml(t, "logoFile"), version: toml(t, "version"), mc: "", loaders: [forge[0]], deps: [] };
  }
  if (!info?.id) {
    console.warn(`skip ${file}: no mod description inside`);
    continue;
  }
  // "name-1.0.1+mc1.21.11.jar" says the real numbers when the jar's file has placeholders
  const fromName = file.match(/-(\d[\w.]*?)\+mc([\w.\-]+)\.jar$/);
  const version = !info.version || info.version.includes("$") ? fromName?.[1] ?? "?" : info.version.split("+")[0];
  const mc = fromName?.[2] ?? info.mc;
  if (!mc) {
    console.warn(`skip ${file}: unknown Minecraft version (name it like mod-1.0.0+mc1.21.11.jar)`);
    continue;
  }
  const bytes = readFileSync(jar);
  const m = mods.get(info.id) ?? { id: info.id, name: info.name ?? info.id, description: info.description ?? "", author: info.author, icon: null, versions: [] };
  if (info.icon && !m.icon) {
    const png = unzip(jar, info.icon.replace(/^\//, ""), true);
    if (png) {
      writeFileSync(join(dir, "icons", `${info.id}.png`), png);
      m.icon = `icons/${info.id}.png`;
    }
  }
  m.versions.push({ version, mc, loaders: info.loaders, file, sha1: createHash("sha1").update(bytes).digest("hex"), size: statSync(jar).size, deps: info.deps });
  mods.set(info.id, m);
}
const out = [...mods.values()].map((m) => ({ ...m, description: extra[m.id] ?? m.description, versions: m.versions.sort((a, b) => cmp(b.version, a.version) || cmp(b.mc, a.mc)) }));
writeFileSync(join(dir, "catalog.json"), JSON.stringify({ mods: out }, null, 2) + "\n");
console.log(out.map((m) => `${m.name}: ${m.versions.length} files`).join("\n") || "no mods");
