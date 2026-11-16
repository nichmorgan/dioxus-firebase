#!/usr/bin/env node
// Generate a local Dioxus.toml from Dioxus.toml.example + ANDROID_APPLICATION_ID.
// Empty / unset → com.example.demo (Maestro / CI default).
// Dioxus.toml is gitignored; the tracked template stays generic.
// Maestro flows use appId: ${ANDROID_APPLICATION_ID}; pass -e when running maestro.
"use strict";

const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const defaultId = "com.example.demo";
const templatePath = path.join(root, "Dioxus.toml.example");
const dioxusPath = path.join(root, "Dioxus.toml");
const envPath = path.join(root, ".env");

function readAppIdFromEnvFile() {
  if (!fs.existsSync(envPath)) {
    return "";
  }
  const lines = fs.readFileSync(envPath, "utf8").split(/\r?\n/);
  let value = "";
  for (const line of lines) {
    const match = line.match(/^\s*ANDROID_APPLICATION_ID=(.*)$/);
    if (match) {
      value = match[1].trim().replace(/^["']|["']$/g, "");
    }
  }
  return value;
}

let appId = (process.env.ANDROID_APPLICATION_ID || "").trim();
if (!appId) {
  appId = readAppIdFromEnvFile().trim();
}
if (!appId) {
  appId = defaultId;
}

if (!fs.existsSync(templatePath)) {
  console.error("Dioxus.toml.example: file not found");
  process.exit(1);
}

const text = fs.readFileSync(templatePath, "utf8");
const pattern = /^(identifier\s*=\s*)".*"/gm;
const matches = text.match(pattern);
if (!matches || matches.length !== 1) {
  console.error("Dioxus.toml.example: could not find [bundle] identifier");
  process.exit(1);
}

const updated = text.replace(pattern, `$1"${appId}"`);
fs.writeFileSync(dioxusPath, updated, "utf8");
console.log(`Synced Dioxus.toml identifier → ${appId}`);
