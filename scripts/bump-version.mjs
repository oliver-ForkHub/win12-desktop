#!/usr/bin/env node
/**
 * bump-version.mjs
 *
 * Bump the version in the repo files that need to change before a release.
 *
 * Usage:
 *   node scripts/bump-version.mjs 0.2.9                 # uses today's date
 *   node scripts/bump-version.mjs 0.2.9 2026-09-11
 *
 * Files updated:
 *   package.json
 *   tauri/src-tauri/Cargo.toml
 *   tauri/src-tauri/Cargo.lock
 *   tauri/src-tauri/tauri.conf.json
 *   snap/snapcraft.yaml
 *   io.github.win12_online.win12_desktop.metainfo.xml
 *
 * For the metainfo.xml only a new <release> entry is added (with a TODO
 * comment where release notes should be written).
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, '..');

const args = process.argv.slice(2);
if (args.length < 1) {
  console.error('Usage: node scripts/bump-version.mjs NEW_VERSION [RELEASE_DATE]');
  console.error('  NEW_VERSION   e.g. 0.2.9');
  console.error('  RELEASE_DATE  e.g. 2026-09-11 (default: today)');
  process.exit(1);
}

const newVersion = args[0];
const releaseDate = args[1] || new Date().toISOString().slice(0, 10);

if (!/^\d+\.\d+\.\d+$/.test(newVersion)) {
  console.error(`Error: version should be MAJOR.MINOR.PATCH, got "${newVersion}"`);
  process.exit(1);
}
if (!/^\d{4}-\d{2}-\d{2}$/.test(releaseDate)) {
  console.error(`Error: release date should be YYYY-MM-DD, got "${releaseDate}"`);
  process.exit(1);
}

function rel(...p) {
  return path.join(ROOT, ...p);
}

const changed = [];
function record(file, oldValue, newValue) {
  changed.push(`${file}: ${oldValue} -> ${newValue}`);
}

// ---------- package.json ----------
{
  const file = rel('package.json');
  const pkg = JSON.parse(readFileSync(file, 'utf8'));
  const oldVersion = pkg.version;
  if (oldVersion !== newVersion) {
    pkg.version = newVersion;
    writeFileSync(file, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
    record(file, oldVersion, newVersion);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

// ---------- tauri/src-tauri/Cargo.toml ----------
{
  const file = rel('tauri', 'src-tauri', 'Cargo.toml');
  const text = readFileSync(file, 'utf8');
  const match = /^version = "([^"]+)"/m.exec(text);
  const oldVersion = match ? match[1] : '?';
  const updated = text.replace(/^version = "[^"]+"/m, `version = "${newVersion}"`);
  if (updated !== text) {
    writeFileSync(file, updated, 'utf8');
    record(file, oldVersion, newVersion);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

// ---------- tauri/src-tauri/Cargo.lock ----------
{
  const file = rel('tauri', 'src-tauri', 'Cargo.lock');
  const text = readFileSync(file, 'utf8');
  let oldVersion = '?';
  let updated = text;
  // Cargo.lock groups package metadata under [[package]] blocks.
  const blocks = text.split(/\n(?=\[\[package\]\])/);
  const newBlocks = blocks.map((block) => {
    if (!/\[\[package\]\]/.test(block)) return block;
    const nameMatch = /^name = "([^"]+)"/m.exec(block);
    if (nameMatch && nameMatch[1] === 'win12-desktop') {
      const versionMatch = /^version = "([^"]+)"/m.exec(block);
      if (versionMatch) oldVersion = versionMatch[1];
      return block.replace(/^version = "[^"]+"/m, `version = "${newVersion}"`);
    }
    return block;
  });
  updated = newBlocks.join('\n');
  if (updated !== text) {
    writeFileSync(file, updated, 'utf8');
    record(file, oldVersion, newVersion);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

// ---------- tauri/src-tauri/tauri.conf.json ----------
{
  const file = rel('tauri', 'src-tauri', 'tauri.conf.json');
  const conf = JSON.parse(readFileSync(file, 'utf8'));
  const oldVersion = conf.version;
  if (oldVersion !== newVersion) {
    conf.version = newVersion;
    writeFileSync(file, JSON.stringify(conf, null, 2) + '\n', 'utf8');
    record(file, oldVersion, newVersion);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

// ---------- snap/snapcraft.yaml ----------
{
  const file = rel('snap', 'snapcraft.yaml');
  const text = readFileSync(file, 'utf8');
  const match = /^version:\s*"([^"]*)"/m.exec(text);
  const oldVersion = match ? match[1] : '?';
  const updated = text.replace(/^version:\s*"[^"]*"/m, `version: "${newVersion}"`);
  if (updated !== text) {
    writeFileSync(file, updated, 'utf8');
    record(file, oldVersion, newVersion);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

// ---------- io.github.win12_online.win12_desktop.metainfo.xml ----------
{
  const file = rel('io.github.win12_online.win12_desktop.metainfo.xml');
  const text = readFileSync(file, 'utf8');

  // Refuse to add a duplicate release version.
  const existingRelease = new RegExp(`<release[^>]*version=["']${newVersion}["']`).test(text);
  if (existingRelease) {
    console.error(`Error: metainfo.xml already has a release entry for version ${newVersion}`);
    process.exit(1);
  }

  const releaseBlock = [
    `    <release version="${newVersion}" date="${releaseDate}">`,
    `      <description>`,
    `        <!-- TODO: 填写本次发版的更新日志 -->`,
    `        <p></p>`,
    `      </description>`,
    `    </release>`,
    ''
  ].join('\n');

  const marker = '</releases>';
  if (!text.includes(marker)) {
    console.error('Error: metainfo.xml does not contain <releases> block');
    process.exit(1);
  }
  const updated = text.replace(marker, releaseBlock + marker);
  if (updated !== text) {
    writeFileSync(file, updated, 'utf8');
    record(file, `(no ${newVersion} release)`, `added release ${newVersion} ${releaseDate}`);
  } else {
    record(file, 'unchanged', newVersion);
  }
}

console.log('\nVersion bumped to ' + newVersion + ':\n');
for (const line of changed) {
  console.log('  ' + line);
}
console.log(
  '\nRemember to fill in the TODO placeholder for the metainfo.xml release notes.'
);
