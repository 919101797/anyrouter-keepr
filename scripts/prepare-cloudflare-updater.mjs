import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";

const sourceDir = path.resolve(process.env.SOURCE_DIR || process.argv[2] || "");
const outputDir = path.resolve(process.env.OUTPUT_DIR || process.argv[3] || "cloudflare-pages");
const maxPagesFileSizeBytes = Number.parseInt(
  process.env.MAX_PAGES_FILE_SIZE_BYTES || `${25 * 1024 * 1024}`,
  10,
);
const releaseTag = process.env.RELEASE_TAG || process.env.GITHUB_REF_NAME || "";
const publicBaseUrl = (process.env.PUBLIC_BASE_URL || "https://anyrouter-claude-keeper.pages.dev").replace(
  /\/+$/,
  "",
);

if (!releaseTag) {
  throw new Error("RELEASE_TAG is required.");
}

if (!existsSync(sourceDir)) {
  throw new Error(`Release asset directory does not exist: ${sourceDir}`);
}

const latestPath = path.join(sourceDir, "latest.json");
if (!existsSync(latestPath)) {
  throw new Error(`Missing updater metadata: ${latestPath}`);
}

const latest = JSON.parse(readFileSync(latestPath, "utf8"));
const assetFiles = readdirSync(sourceDir, { withFileTypes: true })
  .filter((entry) => entry.isFile() && entry.name !== "latest.json")
  .map((entry) => entry.name);
const deployableAssetFiles = assetFiles.filter(
  (fileName) => statSync(path.join(sourceDir, fileName)).size <= maxPagesFileSizeBytes,
);
const oversizedAssetFiles = assetFiles.filter((fileName) => !deployableAssetFiles.includes(fileName));

if (assetFiles.length === 0) {
  throw new Error(`No release assets found in ${sourceDir}`);
}

const releasePath = encodeURIComponent(releaseTag);
const releaseOutputDir = path.join(outputDir, "releases", releasePath);
mkdirSync(releaseOutputDir, { recursive: true });

for (const fileName of deployableAssetFiles) {
  copyFileSync(path.join(sourceDir, fileName), path.join(releaseOutputDir, fileName));
}

const assetFileSet = new Set(assetFiles);
const oversizedAssetFileSet = new Set(oversizedAssetFiles);
const platforms = latest.platforms || {};
const missingAssets = [];
const removedOversizedPlatforms = [];

for (const [platform, entry] of Object.entries(platforms)) {
  if (!entry || typeof entry !== "object") {
    throw new Error(`Invalid updater platform entry: ${platform}`);
  }

  if (!entry.url || !entry.signature) {
    throw new Error(`Updater platform entry ${platform} must include url and signature.`);
  }

  const fileName = fileNameFromUrl(entry.url);
  if (!assetFileSet.has(fileName)) {
    missingAssets.push(`${platform}: ${fileName}`);
    continue;
  }

  if (oversizedAssetFileSet.has(fileName)) {
    delete platforms[platform];
    removedOversizedPlatforms.push(`${platform}: ${fileName}`);
    continue;
  }

  entry.url = `${publicBaseUrl}/releases/${releasePath}/${encodeURIComponent(fileName)}`;
}

if (missingAssets.length > 0) {
  throw new Error(`latest.json points to assets that were not downloaded: ${missingAssets.join(", ")}`);
}

writeFileSync(path.join(outputDir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);
writeFileSync(path.join(releaseOutputDir, "latest.json"), `${JSON.stringify(latest, null, 2)}\n`);
writeFileSync(
  path.join(outputDir, "_headers"),
  [
    "/latest.json",
    "  Cache-Control: no-store",
    "  X-Content-Type-Options: nosniff",
    "",
    "/releases/*",
    "  Cache-Control: public, max-age=31536000, immutable",
    "  X-Content-Type-Options: nosniff",
    "",
  ].join("\n"),
);
writeFileSync(
  path.join(outputDir, "index.html"),
  [
    "<!doctype html>",
    '<html lang="en">',
    "<head>",
    '  <meta charset="utf-8">',
    "  <title>AnyRouter Keeper Updates</title>",
    "</head>",
    "<body>",
    "  <h1>AnyRouter Keeper Updates</h1>",
    `  <p>Current release: ${escapeHtml(releaseTag)}</p>`,
    '  <p><a href="/latest.json">latest.json</a></p>',
    "</body>",
    "</html>",
    "",
  ].join("\n"),
);

console.log(`Prepared Cloudflare updater feed for ${releaseTag} in ${outputDir}`);
if (removedOversizedPlatforms.length > 0) {
  console.log(
    `Removed oversized updater platform entries because Cloudflare Pages files must be <= ${maxPagesFileSizeBytes} bytes: ${removedOversizedPlatforms.join(
      ", ",
    )}`,
  );
}

function fileNameFromUrl(value) {
  try {
    const url = new URL(value);
    return decodeURIComponent(path.posix.basename(url.pathname));
  } catch {
    return decodeURIComponent(path.basename(value));
  }
}

function escapeHtml(value) {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}
