#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';

const TIMEOUT = 20_000;
const EXIT_TIMEOUT = 5_000;

function fileKey(uri) {
  const filename = path.normalize(fileURLToPath(uri));
  return process.platform === 'win32' ? filename.toLowerCase() : filename;
}

function deadline(promise, label, ms = TIMEOUT) {
  let timer;
  return Promise.race([
    promise,
    new Promise((_, reject) => {
      timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms);
    }),
  ]).finally(() => clearTimeout(timer));
}

class Client {
  constructor(binary, cwd) {
    this.child = spawn(binary, ['--lsp', '-stdio'], { cwd, stdio: 'pipe' });
    this.buffer = Buffer.alloc(0);
    this.pending = new Map();
    this.diagnostics = new Map();
    this.waiters = new Set();
    this.nextId = 1;
    this.stderr = '';
    this.failure = null;
    this.exited = new Promise((resolve) => {
      this.child.once('error', (error) => {
        this.fail(error);
        resolve({ error });
      });
      this.child.once('close', (code, signal) => {
        this.fail(new Error(`server exited (code ${code}, signal ${signal})`));
        resolve({ code, signal });
      });
    });
    this.child.stderr.on('data', (chunk) => { this.stderr += chunk.toString(); });
    this.child.stdin.on('error', (error) => this.fail(error));
    this.child.stdout.on('error', (error) => this.fail(error));
    this.child.stdout.on('data', (chunk) => {
      try {
        this.buffer = Buffer.concat([this.buffer, chunk]);
        this.parse();
      } catch (error) {
        this.fail(error);
      }
    });
  }

  fail(error) {
    if (this.failure) return;
    this.failure = error;
    for (const { reject } of this.pending.values()) reject(error);
    this.pending.clear();
    for (const waiter of this.waiters) waiter.reject(error);
    this.waiters.clear();
  }

  parse() {
    while (true) {
      const boundary = this.buffer.indexOf('\r\n\r\n');
      if (boundary < 0) return;
      const header = this.buffer.subarray(0, boundary).toString('ascii');
      const match = /^content-length:\s*(\d+)\s*$/im.exec(header);
      if (!match) throw new Error(`Invalid LSP header: ${header}`);
      const length = Number(match[1]);
      const end = boundary + 4 + length;
      if (this.buffer.length < end) return;
      const message = JSON.parse(this.buffer.subarray(boundary + 4, end).toString('utf8'));
      this.buffer = this.buffer.subarray(end);
      this.receive(message);
    }
  }

  receive(message) {
    if (message.method && message.id !== undefined) {
      // Answer common client requests so they cannot block server initialization.
      if (message.method === 'workspace/configuration') {
        this.send({ jsonrpc: '2.0', id: message.id, result: message.params.items.map(() => null) });
      } else if (['client/registerCapability', 'client/unregisterCapability', 'window/workDoneProgress/create', 'workspace/semanticTokens/refresh', 'workspace/inlayHint/refresh', 'workspace/diagnostic/refresh'].includes(message.method)) {
        this.send({ jsonrpc: '2.0', id: message.id, result: null });
      } else {
        this.send({ jsonrpc: '2.0', id: message.id, error: { code: -32601, message: `Unsupported client method: ${message.method}` } });
      }
    } else if (message.method === 'textDocument/publishDiagnostics') {
      this.diagnostics.set(fileKey(message.params.uri), message.params.diagnostics);
      for (const waiter of this.waiters) waiter.check();
    } else if (message.id !== undefined) {
      const pending = this.pending.get(message.id);
      if (!pending) return;
      this.pending.delete(message.id);
      if (message.error) pending.reject(new Error(`${pending.method}: ${JSON.stringify(message.error)}`));
      else pending.resolve(message.result);
    }
  }

  send(message) {
    if (this.failure) throw this.failure;
    const body = Buffer.from(JSON.stringify(message));
    this.child.stdin.write(Buffer.concat([Buffer.from(`Content-Length: ${body.length}\r\n\r\n`), body]));
  }

  notify(method, params) {
    this.send({ jsonrpc: '2.0', method, params });
  }

  async request(method, params) {
    const id = this.nextId++;
    const result = new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject, method });
    });
    try {
      this.send({ jsonrpc: '2.0', id, method, params });
      return await deadline(result, method);
    } finally {
      this.pending.delete(id);
    }
  }

  async waitForDiagnostics(uri, predicate) {
    if (this.failure) throw this.failure;
    let waiter;
    const result = new Promise((resolve, reject) => {
      waiter = { reject, check: () => {
        const diagnostics = this.diagnostics.get(fileKey(uri));
        if (diagnostics && predicate(diagnostics)) {
          this.waiters.delete(waiter);
          resolve(diagnostics);
        }
      } };
      this.waiters.add(waiter);
      waiter.check();
    });
    try {
      return await deadline(result, `diagnostics for ${uri}`);
    } finally {
      this.waiters.delete(waiter);
    }
  }
}

async function main() {
  const binary = process.argv[2];
  if (!binary || process.argv.length !== 3 || !path.isAbsolute(binary)) {
    throw new Error('Usage: node scripts/smoke-test.mjs /absolute/path/to/native/tsrs');
  }

  const root = await mkdtemp(path.join(tmpdir(), 'tsrs-smoke-'));
  let client;
  try {
    await writeFile(path.join(root, 'tsconfig.json'), JSON.stringify({
      compilerOptions: { allowJs: true, checkJs: true, jsx: 'preserve', noEmit: true, strict: true },
      include: ['*'],
    }));
    const documents = [
      { file: 'sample.ts', languageId: 'typescript', text: 'const smokeValue: number = 42;\nsmokeValue;\nconst smokeError: string = 123;\nsmokeVal\n' },
      { file: 'sample.tsx', languageId: 'typescriptreact', text: 'const tsxValue: number = 42;\ntsxValue;\n' },
      { file: 'sample.js', languageId: 'javascript', text: 'const jsValue = 42;\njsValue;\n' },
      { file: 'sample.jsx', languageId: 'javascriptreact', text: 'const jsxValue = 42;\njsxValue;\n' },
    ];
    for (const doc of documents) {
      await writeFile(path.join(root, doc.file), doc.text);
      doc.uri = pathToFileURL(path.join(root, doc.file)).href;
    }

    client = new Client(binary, root);
    const rootUri = pathToFileURL(root).href;
    const initialized = await client.request('initialize', {
      processId: process.pid,
      rootUri,
      workspaceFolders: [{ uri: rootUri, name: 'smoke' }],
      capabilities: { workspace: { configuration: true }, textDocument: { publishDiagnostics: {} } },
    });
    assert.ok(initialized?.capabilities, 'initialize must return server capabilities');

    client.notify('initialized', {});

    for (const doc of documents) {
      client.notify('textDocument/didOpen', { textDocument: { uri: doc.uri, languageId: doc.languageId, version: 1, text: doc.text } });
      const hover = await client.request('textDocument/hover', { textDocument: { uri: doc.uri }, position: { line: 1, character: 2 } });
      assert.ok(hover?.contents && JSON.stringify(hover.contents).includes(doc.file === 'sample.ts' ? 'smokeValue' : doc.file === 'sample.tsx' ? 'tsxValue' : doc.file === 'sample.js' ? 'jsValue' : 'jsxValue'), `hover failed for ${doc.languageId}`);
      console.log(`Opened and checked hover: ${doc.languageId}`);
    }

    const uri = documents[0].uri;
    const completion = await client.request('textDocument/completion', { textDocument: { uri }, position: { line: 3, character: 8 } });
    const items = Array.isArray(completion) ? completion : completion?.items;
    assert.ok(items?.some((item) => item.label === 'smokeValue'), 'completion must include smokeValue');
    const definition = await client.request('textDocument/definition', { textDocument: { uri }, position: { line: 1, character: 2 } });
    const locations = Array.isArray(definition) ? definition : [definition];
    assert.ok(locations.some((location) => {
      const target = location?.uri ?? location?.targetUri;
      const range = location?.range ?? location?.targetSelectionRange;
      return target && fileKey(target) === fileKey(uri) && range?.start.line === 0;
    }), `definition must point to smokeValue declaration: ${JSON.stringify(definition)}`);
    const isExpectedError = (diagnostics) => diagnostics.some((diagnostic) => diagnostic.range?.start.line === 2 && diagnostic.severity === 1);
    if (initialized.capabilities.diagnosticProvider) {
      const report = await client.request('textDocument/diagnostic', { textDocument: { uri } });
      assert.ok(report?.kind === 'full' && isExpectedError(report.items), `expected type error in diagnostics: ${JSON.stringify(report)}`);
    } else {
      await client.waitForDiagnostics(uri, isExpectedError);
    }
    console.log('Completion, definition, and error diagnostics passed');

    const shutdown = await client.request('shutdown');
    assert.equal(shutdown, null, 'shutdown must return null');
    client.notify('exit');
    const exit = await deadline(client.exited, 'server exit', EXIT_TIMEOUT);
    if (exit.error) throw exit.error;
    assert.ok(!exit.signal && (exit.code === 0 || (exit.code === 1 && /^context canceled\r?\n?$/.test(client.stderr))), `unexpected server exit: code ${exit.code}, signal ${exit.signal}, stderr ${JSON.stringify(client.stderr)}`);
    console.log(exit.code === 0 ? 'Shutdown and exit passed' : 'Shutdown acknowledged; tolerated known upstream exit 1: context canceled');
  } finally {
    if (client && client.child.exitCode === null && client.child.signalCode === null) {
      client.child.kill();
      await deadline(client.exited, 'server cleanup', EXIT_TIMEOUT);
    }
    await rm(root, { recursive: true, force: true });
  }
}

main().catch((error) => {
  console.error(`Smoke test failed: ${error.stack || error}`);
  process.exitCode = 1;
});
