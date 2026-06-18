#!/usr/bin/env node

const { execFileSync } = require("child_process");
const fs = require("fs");
const os = require("os");
const path = require("path");

const root = path.resolve(__dirname, "..", "..");
const pkgPath =
  process.env.MEETILY_APPSTORE_PKG ||
  path.join(root, "target", "release", "bundle", "macos", "meetily_0.4.0_appstore.pkg");
const apiKey = process.env.APPLE_API_KEY || process.env.APPLE_API_KEY_ID;
const apiIssuer = process.env.APPLE_API_ISSUER;
const sourceKeyPath = process.env.APPLE_API_KEY_PATH;

if (os.platform() !== "darwin") {
  console.error("App Store upload must run on macOS.");
  process.exit(1);
}

if (!fs.existsSync(pkgPath)) {
  console.error(`Missing package: ${pkgPath}`);
  process.exit(1);
}

if (!apiKey || !apiIssuer || !sourceKeyPath) {
  console.error("APPLE_API_KEY, APPLE_API_ISSUER, and APPLE_API_KEY_PATH are required.");
  process.exit(1);
}

const privateKeysDir = path.join(os.homedir(), "private_keys");
const altoolKeyPath = path.join(privateKeysDir, `AuthKey_${apiKey}.p8`);
fs.mkdirSync(privateKeysDir, { recursive: true, mode: 0o700 });
if (!fs.existsSync(altoolKeyPath)) {
  fs.copyFileSync(sourceKeyPath.replace("$HOME", os.homedir()), altoolKeyPath);
  fs.chmodSync(altoolKeyPath, 0o600);
}

execFileSync(
  "xcrun",
  ["altool", "--upload-app", "--type", "macos", "--file", pkgPath, "--apiKey", apiKey, "--apiIssuer", apiIssuer],
  { cwd: root, stdio: "inherit" },
);
