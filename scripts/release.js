#!/usr/bin/env node
/**
 * Bumps Memoire's version everywhere it lives, commits it, and optionally
 * pushes to the `release` branch, which runs .github/workflows/release.yml.
 *
 *   bun run release patch            0.3.0 -> 0.3.1
 *   bun run release minor            0.3.0 -> 0.4.0
 *   bun run release major            0.3.0 -> 1.0.0
 *   bun run release 0.5.0-beta.1     an exact version
 *
 * Flags:
 *   --dry-run   show what would change, write nothing
 *   --no-git    only edit the files; don't commit
 *   --push      also push this branch and fast-forward `release` to it
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { execSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const SEMVER = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/;

const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith('--')));
const bump = args.find((a) => !a.startsWith('--'));
const dryRun = flags.has('--dry-run');
const noGit = flags.has('--no-git');
const push = flags.has('--push');

function fail(msg) {
  console.error(`\x1b[31m✗\x1b[0m ${msg}`);
  process.exit(1);
}

function git(cmd, opts = {}) {
  return execSync(`git ${cmd}`, { cwd: root, encoding: 'utf8', ...opts }).trim();
}

if (!bump) {
  fail('Usage: bun run release <patch|minor|major|x.y.z> [--dry-run] [--no-git] [--push]');
}

// Each place the version lives, as a regex whose first group is the version.
// Edited in place so the files keep their formatting.
const FILES = [
  { path: 'package.json', pattern: /^(\s*"version":\s*")([^"]+)(")/m },
  { path: 'src-tauri/tauri.conf.json', pattern: /^(\s*"version":\s*")([^"]+)(")/m },
  // The first `version =` in Cargo.toml is the one under [package].
  { path: 'src-tauri/Cargo.toml', pattern: /^(version\s*=\s*")([^"]+)(")/m },
  {
    path: 'src-tauri/Cargo.lock',
    pattern: /(\[\[package\]\]\nname = "memoire"\nversion = ")([^"]+)(")/,
  },
];

for (const f of FILES) {
  f.text = readFileSync(join(root, f.path), 'utf8');
  const m = f.text.match(f.pattern);
  if (!m) fail(`Couldn't find the version in ${f.path}`);
  f.current = m[2];
}

const current = FILES.find((f) => f.path === 'src-tauri/tauri.conf.json').current;
const mismatched = FILES.filter((f) => f.current !== current);
if (mismatched.length) {
  console.warn(
    `\x1b[33m!\x1b[0m Versions disagree: ${FILES.map((f) => `${f.path}=${f.current}`).join(', ')}. ` +
      `Using ${current} (tauri.conf.json) as the current version.`
  );
}

function nextVersion(from, how) {
  if (SEMVER.test(how)) return how;
  const m = from.match(SEMVER);
  if (!m) fail(`Current version ${from} isn't valid semver`);
  let [major, minor, patch] = m.slice(1, 4).map(Number);
  const pre = m[4];
  switch (how) {
    case 'major':
      return `${major + 1}.0.0`;
    case 'minor':
      return `${major}.${minor + 1}.0`;
    case 'patch':
      // 0.4.0-beta.1 -> 0.4.0: finishing a prerelease ships it.
      return pre ? `${major}.${minor}.${patch}` : `${major}.${minor}.${patch + 1}`;
    default:
      fail(`"${how}" isn't patch, minor, major or a version like 1.2.3`);
  }
}

function compare(a, b) {
  const pa = a.match(SEMVER);
  const pb = b.match(SEMVER);
  for (let i = 1; i <= 3; i++) {
    if (+pa[i] !== +pb[i]) return +pa[i] - +pb[i];
  }
  if (pa[4] && !pb[4]) return -1;
  if (!pa[4] && pb[4]) return 1;
  return (pa[4] ?? '').localeCompare(pb[4] ?? '', undefined, { numeric: true });
}

const next = nextVersion(current, bump);
// The updater only installs versions higher than the running one.
if (compare(next, current) <= 0) {
  fail(`${next} isn't newer than ${current}; installed apps would never update to it`);
}

// A release built without the real public key ships an updater that can't verify anything.
const conf = JSON.parse(FILES.find((f) => f.path === 'src-tauri/tauri.conf.json').text);
const pubkey = conf.plugins?.updater?.pubkey ?? '';
if (!pubkey || pubkey.startsWith('REPLACE_WITH')) {
  fail(
    'plugins.updater.pubkey in tauri.conf.json is still a placeholder. Generate a key first:\n' +
      '  bunx tauri signer generate -w ~/.tauri/memoire.key'
  );
}

if (!noGit && !dryRun) {
  const dirty = git('status --porcelain');
  if (dirty) fail(`Working tree has uncommitted changes. Commit or stash them first:\n${dirty}`);
  const tag = `app-v${next}`;
  if (git(`tag --list ${tag}`)) fail(`Tag ${tag} already exists`);
}

console.log(`${dryRun ? '[dry run] ' : ''}${current} → \x1b[32m${next}\x1b[0m`);
for (const f of FILES) {
  const updated = f.text.replace(f.pattern, `$1${next}$3`);
  console.log(`  ${f.path}`);
  if (!dryRun) writeFileSync(join(root, f.path), updated);
}

if (dryRun || noGit) process.exit(0);

git(`add ${FILES.map((f) => f.path).join(' ')}`);
git(`commit -m "Bump version to ${next}"`, { stdio: 'inherit' });
console.log(`\x1b[32m✓\x1b[0m Committed "Bump version to ${next}"`);

if (!push) {
  console.log('\nTo build the release: git push && git push origin HEAD:release');
  console.log('(or re-run with --push next time)');
  process.exit(0);
}

const branch = git('rev-parse --abbrev-ref HEAD');
git(`push origin ${branch}`, { stdio: 'inherit' });
// Fast-forward only: if `release` has commits this branch lacks, stop rather than clobber them.
git('push origin HEAD:release', { stdio: 'inherit' });
console.log(
  `\x1b[32m✓\x1b[0m Pushed to release. The workflow will create a draft "App v${next}" release;`
);
console.log('  publish the draft on GitHub so installed apps can see it.');
