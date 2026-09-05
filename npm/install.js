#!/usr/bin/env node
"use strict";

const { createHash } = require("crypto");
const { execFileSync } = require("child_process");
const fs = require("fs");
const https = require("https");
const os = require("os");
const path = require("path");

const VERSION = require("./package.json").version;
const REPO = "tsirysndr/tunein-cli";
const BIN_DIR = path.join(__dirname, "bin");
const BIN_PATH = path.join(BIN_DIR, "tunein");

const TARGETS = {
  "darwin arm64": "aarch64-apple-darwin",
  "darwin x64": "x86_64-apple-darwin",
  "linux x64": "x86_64-unknown-linux-gnu",
  "linux arm64": "aarch64-unknown-linux-gnu",
  "linux arm": "armv7-unknown-linux-gnueabihf",
  "freebsd x64": "x86_64-unknown-freebsd",
  "freebsd arm64": "aarch64-unknown-freebsd",
  "netbsd x64": "x86_64-unknown-netbsd",
  "netbsd arm64": "aarch64-unknown-netbsd",
};

function resolveTarget() {
  const key = `${process.platform} ${process.arch}`;
  const target = TARGETS[key];
  if (!target) {
    throw new Error(
      `Unsupported platform: ${key}. ` +
        `See https://github.com/${REPO}/releases for available builds.`
    );
  }
  return target;
}

function fetch(url, redirects = 5) {
  return new Promise((resolve, reject) => {
    https
      .get(url, { headers: { "User-Agent": "tunein-cli-npm" } }, (res) => {
        if (
          res.statusCode >= 300 &&
          res.statusCode < 400 &&
          res.headers.location
        ) {
          res.resume();
          if (redirects === 0) {
            reject(new Error(`Too many redirects fetching ${url}`));
            return;
          }
          resolve(fetch(res.headers.location, redirects - 1));
          return;
        }
        if (res.statusCode !== 200) {
          res.resume();
          reject(new Error(`Download failed: ${url} (HTTP ${res.statusCode})`));
          return;
        }
        const chunks = [];
        res.on("data", (chunk) => chunks.push(chunk));
        res.on("end", () => resolve(Buffer.concat(chunks)));
        res.on("error", reject);
      })
      .on("error", reject);
  });
}

async function install() {
  if (fs.existsSync(BIN_PATH)) {
    return BIN_PATH;
  }

  const target = resolveTarget();
  const asset = `tunein_v${VERSION}_${target}.tar.gz`;
  const url = `https://github.com/${REPO}/releases/download/v${VERSION}/${asset}`;

  console.log(`Downloading tunein v${VERSION} for ${target}...`);
  const [tarball, checksumFile] = await Promise.all([
    fetch(url),
    fetch(`${url}.sha256`),
  ]);

  const expected = checksumFile.toString("utf8").trim().split(/\s+/)[0];
  const actual = createHash("sha256").update(tarball).digest("hex");
  if (expected !== actual) {
    throw new Error(
      `Checksum mismatch for ${asset}: expected ${expected}, got ${actual}`
    );
  }

  fs.mkdirSync(BIN_DIR, { recursive: true });
  const tmp = path.join(BIN_DIR, `${asset}.${process.pid}`);
  fs.writeFileSync(tmp, tarball);
  try {
    execFileSync("tar", ["-xzf", tmp, "-C", BIN_DIR]);
  } finally {
    fs.rmSync(tmp, { force: true });
  }

  if (!fs.existsSync(BIN_PATH)) {
    throw new Error(`Archive ${asset} did not contain the tunein binary`);
  }
  fs.chmodSync(BIN_PATH, 0o755);
  console.log(`Installed tunein to ${BIN_PATH}`);
  return BIN_PATH;
}

module.exports = { install, BIN_PATH };

if (require.main === module) {
  install().catch((err) => {
    console.error(`tunein-cli: ${err.message}`);
    process.exit(1);
  });
}
