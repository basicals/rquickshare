import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { createServer } from "node:http";
import { createInterface } from "node:readline";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import { readFile, readdir } from "node:fs/promises";
import test from "node:test";
import { build } from "esbuild";
import { Miniflare } from "miniflare";
import { BASE_URI, SUITE, objectPolicy } from "../src/policy.ts";

test("Given_actual_workerd_R2_and_APT_When_acquiring_selected_prefix_Then_signatures_indexes_packages_and_HTTP_semantics_work", async () => {
  // Arrange: real local binding and ephemeral test-only key; no production objects or credentials.
  const bundled = await build({ entryPoints: ["src/worker.ts"], bundle: true, write: false, format: "esm", platform: "browser" });
  const mf = new Miniflare({ logRequests: false, workers: [{ config: { name: "native-download",
    manifest: { mainModule: "worker.mjs", modules: { "worker.mjs": { type: "esm", contents: bundled.outputFiles[0].text } } },
    compatibilityDate: "2026-10-14", compatibilityFlags: ["nodejs_compat"],
    env: { ARCHIVE: { type: "r2", name: "local-test-only" } } } }] });
  const fixture = spawn("python3", ["test/apt_fixture.py"], { stdio: ["pipe", "pipe", "pipe"] });
  const completion = once(fixture, "exit");
  let errors = "";
  fixture.stderr.setEncoding("utf8").on("data", chunk => { errors += chunk; });
  const lines = createInterface({ input: fixture.stdout });
  let server;
  const paths = [];
  try {
    const iterator = lines[Symbol.asyncIterator]();
    const first = await iterator.next();
    assert.equal(first.done, false, errors);
    const { repository, production_uri } = JSON.parse(first.value);
    assert.equal(production_uri, BASE_URI);
    const storage = await mf.getR2Bucket("ARCHIVE");
    async function load(directory, prefix = "") {
      for (const file of await readdir(directory, { withFileTypes: true })) {
        const key = prefix + file.name;
        if (file.isDirectory()) await load(directory + "/" + file.name, key + "/");
        else if (objectPolicy(BASE_URI + key)) await storage.put(key, await readFile(directory + "/" + file.name));
      }
    }
    await load(repository);
    // Even accidentally present private bytes must never become HTTP-readable.
    await storage.put("snapshots/private", "private authority");
    const pool = "pool/main/r/rquickshare-native/rquickshare-native_1.0.0-1_amd64.deb";
    const full = await mf.dispatchFetch(BASE_URI + pool);
    assert.equal(full.status, 200);
    const expected = await readFile(repository + "/" + pool);
    assert.deepEqual(Buffer.from(await full.arrayBuffer()), expected);
    const etag = full.headers.get("ETag");
    for (const [headers, status, start, end] of [
      [{ Range: "bytes=2-4" }, 206, 2, 5], [{ Range: "bytes=-10" }, 206, expected.length - 10, expected.length],
      [{ Range: "bytes=2-", "If-Range": '"old"' }, 200, 0, expected.length],
    ]) {
      const response = await mf.dispatchFetch(BASE_URI + pool, { headers });
      assert.equal(response.status, status);
      assert.deepEqual(Buffer.from(await response.arrayBuffer()), expected.subarray(start, end));
    }
    const conditional = await mf.dispatchFetch(BASE_URI + pool, { headers: { "If-None-Match": etag } });
    assert.equal(conditional.status, 304);
    const head = await mf.dispatchFetch(BASE_URI + pool, { method: "HEAD" });
    assert.equal(head.status, 200);
    assert.equal(await head.text(), "");
    assert.equal(head.headers.get("Content-Length"), String(expected.length));
    for (const key of ["snapshots/private", "../Picturix/latest.json", `dists/${SUITE}/Release`, "unknown"]) {
      const response = await mf.dispatchFetch(BASE_URI + key);
      assert.equal(response.status, 404);
      assert.equal(response.headers.get("Cache-Control"), "no-store");
    }
    // Act: bridge owned loopback HTTP to the exact production host/path policy in workerd.
    server = createServer(async (request, response) => {
      paths.push(request.url);
      try {
        const result = await mf.dispatchFetch("https://download.morozhenko.com" + request.url,
          { method: request.method, headers: request.headers });
        response.writeHead(result.status, Object.fromEntries(result.headers));
        if (result.body) await pipeline(Readable.fromWeb(result.body), response);
        else response.end();
      } catch { response.writeHead(500); response.end(); }
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    fixture.stdin.end(JSON.stringify({ uri: `http://127.0.0.1:${server.address().port}/dist/rQuickShare/` }) + "\n");
    const [code] = await completion;
    // Assert: actual apt validates correct authority, downloads exact .deb, rejects wrong key.
    assert.equal(code, 0, errors);
    assert.ok(paths.some(path => path.endsWith("/InRelease")));
    assert.ok(paths.some(path => path.includes("/by-hash/SHA256/")));
    assert.ok(paths.some(path => path.endsWith(".deb")));
    assert.equal(paths.some(path => /\/(Release|Release.gpg|Packages|Packages.gz)$/.test(path)), false);
    await storage.delete(`dists/${SUITE}/InRelease`);
    const absent = await mf.dispatchFetch(BASE_URI + `dists/${SUITE}/InRelease`);
    assert.equal(absent.status, 404);
    assert.equal(absent.headers.get("Cache-Control"), "no-store");
  } finally {
    fixture.stdin.end();
    await completion;
    lines.close();
    if (server) await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    await mf.dispose();
  }
});
