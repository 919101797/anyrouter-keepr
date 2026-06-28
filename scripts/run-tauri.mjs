import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { delimiter, join } from "node:path";

const env = { ...process.env };
const pathKey = Object.keys(env).find((key) => key.toLowerCase() === "path") ?? "PATH";

if (process.platform === "win32") {
  const cargoBin = join(homedir(), ".cargo", "bin");
  const currentPath = env[pathKey] ?? "";
  const pathEntries = currentPath.split(delimiter).map((entry) => entry.toLowerCase());

  if (existsSync(join(cargoBin, "cargo.exe")) && !pathEntries.includes(cargoBin.toLowerCase())) {
    env[pathKey] = `${cargoBin}${delimiter}${currentPath}`;
  }
}

const tauriCli = join("node_modules", "@tauri-apps", "cli", "tauri.js");
const child = spawn(process.execPath, [tauriCli, ...process.argv.slice(2)], {
  env,
  stdio: "inherit",
});

child.on("error", (error) => {
  console.error(`Failed to start Tauri CLI: ${error.message}`);
  process.exit(1);
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 1);
});
