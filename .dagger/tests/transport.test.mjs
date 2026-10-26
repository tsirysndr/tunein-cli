import test from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import { readFile } from 'node:fs/promises';
import { connectWith, rawImpl } from '../sdk/purescript/src/Dagger/Internal/Runtime.js';

async function server(t, handler) {
  const server = createServer(handler);
  server.listen(0, '127.0.0.1');
  await once(server, 'listening');
  t.after(() => { server.closeAllConnections(); server.close(); });
  return timeoutMs => connectWith({ port: server.address().port, token: 'test', timeoutMs })();
}
function query(session) {
  let cancel;
  const result = new Promise((resolve, reject) => {
    cancel = rawImpl(session)('{ ok }')({})(e => () => reject(e))(v => () => resolve(v))();
  });
  return { result, cancel };
}
test('slow headers use the configured deadline without fetch', async t => {
  const oldFetch = globalThis.fetch;
  globalThis.fetch = () => { throw new Error('fetch must not enforce a second timeout'); };
  t.after(() => { globalThis.fetch = oldFetch; });
  const session = await server(t, (req, res) => {
    assert.equal(req.url, '/query');
    assert.equal(req.headers.authorization, 'Basic dGVzdDo=');
    setTimeout(() => res.end('{"data":{"ok":true}}'), 60);
  });
  assert.deepEqual(await query(session(1000)).result, { ok: true });
});
test('deadline covers headers and body', async t => {
  for (const headers of [false, true]) {
    const session = await server(t, (_, res) => { if (headers) { res.writeHead(200); res.write('{'); } });
    await assert.rejects(query(session(30)).result, /Dagger request timed out/);
  }
});
test('cancellation suppresses callbacks', async t => {
  const session = await server(t, (_, res) => { setTimeout(() => res.end('{"data":{}}'), 60); });
  let callbacks = 0;
  const cancel = rawImpl(session(1000))('{ ok }')({})(() => () => callbacks++)(() => () => callbacks++)();
  cancel();
  await new Promise(resolve => setTimeout(resolve, 100));
  assert.equal(callbacks, 0);
});
test('HTTP, malformed JSON, and structured execution errors survive transport', async t => {
  for (const [status, body, expected] of [[302, '', /HTTP error 302/], [200, 'bad', SyntaxError], [200, '{"errors":[{"message":"failed","extensions":{"_type":"EXEC_ERROR","exitCode":101}}]}', { name: 'DaggerExecError', extensions: { _type: 'EXEC_ERROR', exitCode: 101 } }]]) {
    const session = await server(t, (_, res) => { res.writeHead(status); res.end(body); });
    await assert.rejects(query(session(1000)).result, expected);
  }
});
test('every committed module uses the patched transport', async () => {
  const source = await readFile(new URL('../sdk/purescript/src/Dagger/Internal/Runtime.js', import.meta.url), 'utf8');
  for (const module of ['ci']) {
    assert.equal(await readFile(new URL(`../${module}/dagger-sdk/src/Dagger/Internal/Runtime.js`, import.meta.url), 'utf8'), source);
  }
});
