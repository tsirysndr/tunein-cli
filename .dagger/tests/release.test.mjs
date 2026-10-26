import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const script = fileURLToPath(new URL('../ci/scripts/release.sh', import.meta.url));
async function fixture(t, { exists = true, createFails = false } = {}) {
  const root = await mkdtemp(join(tmpdir(), 'dagger-release-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  await mkdir(join(root, 'bin'));
  await mkdir(join(root, 'assets'));
  const log = join(root, 'calls.jsonl');
  await writeFile(log, '');
  await writeFile(join(root, 'bin', 'gh'), `#!/usr/bin/env node
import { appendFileSync } from 'node:fs';
const args = process.argv.slice(2);
appendFileSync(process.env.GH_MOCK_LOG, JSON.stringify(args) + '\\n');
if (args[1] === 'view') process.exit(${exists ? 0 : 1});
if (args[1] === 'create') process.exit(${createFails ? 1 : 0});
`, { mode: 0o755 });
  // Make the stub's module format independent of the host Node default.
  await writeFile(join(root, 'bin', 'package.json'), '{"type":"module"}');
  const env = { ...process.env, PATH: `${join(root, 'bin')}:${process.env.PATH}`, GH_MOCK_LOG: log, RELEASE_TAG: 'v1.2.3' };
  return {
    assets: join(root, 'assets'),
    run: mode => spawnSync('bash', [script, mode], { cwd: join(root, 'assets'), env, encoding: 'utf8' }),
    calls: async () => (await readFile(log, 'utf8')).trim().split('\n').filter(Boolean).map(JSON.parse),
  };
}
test('existing release uploads assets only, excluding checkout metadata and directories', async t => {
  const f = await fixture(t);
  for (const name of ['app.tar.gz', 'app.tar.gz.sha256', 'driver.zip', 'app.deb', 'app.rpm', 'launch-agent.plist']) {
    await writeFile(join(f.assets, name), 'fixture');
  }
  await mkdir(join(f.assets, 'debian'));
  const result = f.run('create');
  assert.equal(result.status, 0, result.stderr);
  const calls = await f.calls();
  assert.equal(calls.some(args => args[1] === 'create'), false);
  assert.deepEqual(calls.at(-1), ['release', 'upload', 'v1.2.3', './app.tar.gz', './driver.zip', './app.deb', './app.rpm', './app.tar.gz.sha256', '--clobber']);
});
test('failed release creation never proceeds to upload', async t => {
  const f = await fixture(t, { exists: false, createFails: true });
  await writeFile(join(f.assets, 'app.tar.gz'), 'fixture');
  assert.notEqual(f.run('create').status, 0);
  assert.equal((await f.calls()).some(args => args[1] === 'upload'), false);
});
test('upload-only mode rejects an empty asset directory without creating a release', async t => {
  const f = await fixture(t);
  assert.notEqual(f.run('upload').status, 0);
  assert.deepEqual(await f.calls(), []);
});
