// Sets the launcher version everywhere it is written: node scripts/set-version.mjs 1.4.0
import { readFileSync, writeFileSync } from "node:fs";

const v = (process.argv[2] ?? "").trim().replace(/^v/, "");
if (!/^\d+\.\d+\.\d+$/.test(v)) {
  console.error(`Номер версии должен быть вида 1.4.0, а не "${v}"`);
  process.exit(1);
}
const json = (path) => {
  const data = JSON.parse(readFileSync(path, "utf8"));
  data.version = v;
  writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
};
json("package.json");
json("src-tauri/tauri.conf.json");
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8").replace(/^version = "[^"]*"/m, `version = "${v}"`);
writeFileSync("src-tauri/Cargo.toml", cargo);
console.log(`Версия ${v}`);
