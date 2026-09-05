#!/usr/bin/env node
"use strict";

const { spawn } = require("child_process");
const fs = require("fs");
const { install, BIN_PATH } = require("./install");

async function main() {
  // The binary is normally downloaded by the postinstall script, but fall
  // back to installing on first run (e.g. npm installed with --ignore-scripts).
  if (!fs.existsSync(BIN_PATH)) {
    await install();
  }

  const child = spawn(BIN_PATH, process.argv.slice(2), { stdio: "inherit" });
  child.on("error", (err) => {
    console.error(`tunein-cli: failed to start tunein: ${err.message}`);
    process.exit(1);
  });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    } else {
      process.exit(code === null ? 1 : code);
    }
  });
}

main().catch((err) => {
  console.error(`tunein-cli: ${err.message}`);
  process.exit(1);
});
