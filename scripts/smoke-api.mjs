import assert from 'node:assert/strict';

// Writes one uniquely named test note. Use a disposable local data directory.
const base = new URL(process.argv[2] ?? 'http://127.0.0.1:8788');
if (!['127.0.0.1', 'localhost'].includes(base.hostname)) throw new Error('Smoke test is local-only');
const id = `smoke-${Date.now()}`;
const markdown = '# 검증 노트\n\n- [ ] Canonical Markdown\n<script>text only</script>';
const call = (path, options) => fetch(new URL(path, base), options);
for (const path of ['/', '/notes']) {
  const shell = await call(path);
  assert.equal(shell.status, 200);
  assert.match(shell.headers.get('content-type'), /text\/html/);
  assert.match(await shell.text(), /Folio/);
  // The application's Worker sets no-store on every response; assets bypass it.
  assert.notEqual(shell.headers.get('cache-control'), 'no-store');
}
const health = await call('/api/health');
assert.equal(health.status, 200);
assert.equal((await health.json()).status, 'ok');
const saved = await call(`/api/notes/${id}`, {
  method: 'PUT', headers: { 'content-type': 'application/json', origin: base.origin },
  body: JSON.stringify({ markdown }),
});
assert.equal(saved.status, 200, await saved.clone().text());
assert.equal((await saved.json()).title, '검증 노트');
const opened = await call(`/api/notes/${id}`);
assert.equal(opened.headers.get('cache-control'), 'no-store');
assert.equal((await opened.json()).markdown, markdown);
const list = await (await call('/api/notes')).json();
assert.ok(list.some(note => note.id === id));
assert.ok(list.length <= 50);
assert.equal((await call('/api/does-not-exist')).status, 404);
assert.equal((await call('/api/notes/never-created')).status, 404);
assert.equal((await call('/api/notes/bad.id', {
  method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ markdown }),
})).status, 400);
assert.equal((await call(`/api/notes/${id}`, {
  method: 'PUT', headers: { 'content-type': 'application/json', origin: 'https://other.example' }, body: JSON.stringify({ markdown }),
})).status, 403);
assert.equal((await call(`/api/notes/${id}`, {
  method: 'PUT', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ markdown: 'x'.repeat(128 * 1024 + 1) }),
})).status, 400);
assert.equal((await (await call(`/api/notes/${id}`)).json()).markdown, markdown);
assert.equal((await call(`/api/notes/${id}`, {
  method: 'PUT', headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ markdown: 'x'.repeat(128 * 1024 * 6 + 1024) }),
})).status, 413);
console.log(`API smoke passed: ${base.origin} (${id})`);
