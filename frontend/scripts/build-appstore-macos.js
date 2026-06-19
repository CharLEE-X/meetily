#!/usr/bin/env node

const { execFileSync, execSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
const frontend = path.resolve(__dirname, "..");
const signingDir = path.join(root, ".signing");
const profilePath = process.env.MEETILY_APPSTORE_PROFILE
  ? path.resolve(process.env.MEETILY_APPSTORE_PROFILE)
  : path.join(signingDir, "meetily-mac-app-store-connect.provisionprofile");
const appStoreConfigPath = path.join(frontend, "src-tauri", "tauri.appstore.conf.json");
const appStoreConfig = JSON.parse(fs.readFileSync(appStoreConfigPath, "utf8"));
const packageVersion = appStoreConfig.version || JSON.parse(
  fs.readFileSync(path.join(frontend, "src-tauri", "tauri.conf.json"), "utf8"),
).version;
const appPath = path.join(root, "target", "release", "bundle", "macos", "meetily.app");
const pkgPath = path.join(root, "target", "release", "bundle", "macos", `meetily_${packageVersion}_appstore.pkg`);
const appEntitlements = path.join(frontend, "src-tauri", "entitlements.appstore.plist");
const nestedEntitlements = path.join(frontend, "src-tauri", "entitlements.appstore.nested.plist");
const embeddedProfile = path.join(appPath, "Contents", "embedded.provisionprofile");
const nestedExecutables = [
  path.join(appPath, "Contents", "MacOS", "ffmpeg"),
  path.join(appPath, "Contents", "MacOS", "llama-helper"),
];
const appIdentity =
  process.env.MEETILY_APPSTORE_APP_IDENTITY ||
  "3rd Party Mac Developer Application: Adrian Witaszak (35M6G2GGQB)";
const installerIdentity =
  process.env.MEETILY_APPSTORE_INSTALLER_IDENTITY ||
  "3rd Party Mac Developer Installer: Adrian Witaszak (35M6G2GGQB)";

function run(command, args, options = {}) {
  console.log(`\n$ ${[command, ...args].join(" ")}`);
  execFileSync(command, args, {
    cwd: options.cwd || frontend,
    stdio: "inherit",
    env: { ...process.env, ...options.env },
  });
}

function detectFeature() {
  if (process.env.TAURI_GPU_FEATURE) return process.env.TAURI_GPU_FEATURE;
  try {
    return execSync("node scripts/auto-detect-gpu.js", {
      cwd: frontend,
      encoding: "utf8",
      stdio: ["pipe", "pipe", "inherit"],
    }).trim();
  } catch {
    return "";
  }
}

if (os.platform() !== "darwin") {
  console.error("Mac App Store packaging must run on macOS.");
  process.exit(1);
}

if (!fs.existsSync(profilePath)) {
  console.error(`Missing provisioning profile: ${profilePath}`);
  process.exit(1);
}

const feature = detectFeature();
const featureArgs =
  feature && feature !== "none"
    ? ["--", "--no-default-features", "--features", feature]
    : ["--", "--no-default-features"];
const env = {
  APPLE_SIGNING_IDENTITY: appIdentity,
};

run(
  "pnpm",
  [
    "tauri",
    "build",
    "--no-bundle",
    "--config",
    "src-tauri/tauri.appstore.conf.json",
    ...featureArgs,
  ],
  { env },
);
run(
  "pnpm",
  [
    "tauri",
    "bundle",
    "--bundles",
    "app",
    "--config",
    "src-tauri/tauri.appstore.conf.json",
  ],
  { env },
);

if (fs.existsSync(embeddedProfile)) {
  fs.chmodSync(embeddedProfile, 0o644);
}

for (const executable of nestedExecutables) {
  if (fs.existsSync(executable)) {
    run("codesign", ["--force", "--sign", appIdentity, "--entitlements", nestedEntitlements, executable], {
      cwd: root,
    });
  }
}

run(
  "codesign",
  ["--force", "--sign", appIdentity, "--entitlements", appEntitlements, appPath],
  { cwd: root },
);

run("xcrun", ["productbuild", "--sign", installerIdentity, "--component", appPath, "/Applications", pkgPath], {
  cwd: root,
});

console.log(`\nCreated App Store package: ${pkgPath}`);
