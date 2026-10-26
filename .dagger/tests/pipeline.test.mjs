import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { readdir, readFile } from 'node:fs/promises';

// Build with `cd .dagger/ci && spago build --strict` first.
const output = process.env.CI_OUTPUT_DIR
  ? pathToFileURL(resolve(process.env.CI_OUTPUT_DIR) + '/')
  : new URL('../ci/output/', import.meta.url);
const prefix = process.env.CI_MODULE_PREFIX ? `${process.env.CI_MODULE_PREFIX}.` : '';
const load = name => import(new URL(`${name}/index.js`, output));
const [Dagger, Query, Aff, Either, Release, Assets] = await Promise.all([
  load('Dagger'), load('Dagger.Query'), load('Effect.Aff'), load('Data.Either'),
  load(`${prefix}CI.Release`), load(`${prefix}CI.Assets`),
]);
const run = aff => new Promise((resolve, reject) => Aff.runAff_(result => () => {
  if (result instanceof Either.Left) reject(result.value0);
  else resolve(result.value0);
})(aff)());

async function engine(t, { files = [], exists = true, fail = false } = {}) {
  const commands = [];
  const server = createServer(async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    const { query, variables } = JSON.parse(body);
    const fields = query.slice(query.indexOf('{')).replace(/\([^)]*\)/g, '').match(/\b\w+\b/g);
    const leaf = fields.at(-1);
    const args = Object.values(variables);
    const execs = args.filter(Array.isArray);
    commands.push(...execs);
    if (fail && leaf === 'sync') {
      res.end(JSON.stringify({ errors: [{ message: 'publication failed', extensions: { _type: 'EXEC_ERROR', exitCode: 1 } }] }));
      return;
    }
    let value = leaf === 'entries' ? files
      : leaf === 'exists' ? !args.includes('directory.deb')
      : leaf === 'exitCode' ? (exists ? 0 : 1) : 'fixture-id';
    for (const field of fields.reverse()) value = { [field]: value };
    res.end(JSON.stringify({ data: value }));
  });
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(() => { server.closeAllConnections(); server.close(); });
  const client = Dagger.connectWith({ port: server.address().port, token: 'fixture', timeoutMs: 1000 })();
  return { client, commands, args: {
    assets: Query.directory(client), token: Query.setSecret('fixture')('fixture')(client),
    tag: 'v1.2.3', runId: 'fixture',
  } };
}

test('existing release uploads only regular release assets as literal arguments', async t => {
  const f = await engine(t, { files: ['app.tar.gz', 'app.tar.gz.sha256', 'driver.zip', 'directory.deb', 'launch-agent.plist'] });
  await run(Release.publishRelease(f.client)(f.args));
  const gh = f.commands.filter(args => args[0] === 'gh');
  assert.equal(gh.some(args => args[2] === 'create'), false);
  assert.deepEqual(gh.at(-1), ['gh', 'release', 'upload', 'v1.2.3', '--clobber', './app.tar.gz', './app.tar.gz.sha256', './driver.zip']);
});
test('new release creation precedes upload and engine failures propagate', async t => {
  const f = await engine(t, { files: ['app.tar.gz'], exists: false, fail: true });
  await assert.rejects(run(Release.publishRelease(f.client)(f.args)), /publication failed/);
  const gh = f.commands.filter(args => args[0] === 'gh');
  assert.equal(gh.findIndex(args => args[2] === 'create') < gh.findIndex(args => args[2] === 'upload'), true);
});
test('empty uploads fail before invoking a publishing command', async t => {
  const f = await engine(t);
  await assert.rejects(run(Release.uploadRelease(f.client)(f.args)), /No release assets/);
  assert.equal(f.commands.length, 0);
});
test('asset selection excludes checkout metadata', () => {
  assert.equal(Assets.isReleaseAsset('service.plist'), false);
  assert.equal(Assets.isReleaseAsset('README.md'), false);
  assert.equal(Assets.isReleaseAsset('driver.zip.sha256'), true);
});
test('handwritten pipeline code never launches a shell', async () => {
  const directory = new URL('../ci/src/CI/', import.meta.url);
  for (const name of await readdir(directory)) {
    if (!name.endsWith('.purs')) continue;
    const source = await readFile(new URL(name, directory), 'utf8');
    assert.doesNotMatch(source, /withExec(?:With)?\s*\[\s*"(?:sh|bash|\/bin\/sh|\/bin\/bash)"/, name);
  }
});

test('package versions reject paths, whitespace and metadata injection', async () => {
  const Package = await load(`${prefix}CI.Package`);
  for (const version of ['v1.2.3', '1.0.0-beta.21', '2.0+build.4']) assert.equal(Package.validVersion(version), true, version);
  for (const version of ['', 'v', '../1', '1/2', '1\nVersion: bad', '1&bad', '$(id)', '-1']) assert.equal(Package.validVersion(version), false, version);
});
