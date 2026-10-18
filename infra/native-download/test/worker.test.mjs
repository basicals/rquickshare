import assert from "node:assert/strict";
import test from "node:test";
import { serve } from "../src/worker.ts";
import { BASE_URI, SUITE } from "../src/policy.ts";

const active = `dists/${SUITE}/InRelease`;
const pool = "pool/main/r/rquickshare-native/rquickshare-native_1.0.0-2_amd64.deb";
const metadata = { size: 10, etag: "one", httpEtag: '"one"' };
function request(key = pool, method = "GET", headers = {}) {
  return new Request(BASE_URI + key, { method, headers });
}
function archive(overrides = {}) {
  const calls = [];
  return { calls, async head(key) { calls.push(["head", key]); return metadata; },
    async get(key, options) {
      calls.push(["get", key, options]);
      const range = options?.range;
      const body = new Response(range ? "0123456789".slice(range.offset, range.offset + range.length) : "0123456789").body;
      return { ...metadata, body, ...(range ? { range } : {}) };
    }, ...overrides };
}

test("Given_invalid_paths_and_methods_When_requested_Then_storage_is_never_accessed", async () => {
  const storage = archive();
  for (const [key, method, status] of [["snapshots/private", "GET", 404],
    ["../Picturix/latest.json", "GET", 404], [active, "POST", 405],
    [pool, "DELETE", 405], ["unknown", "HEAD", 404]]) {
    const response = await serve(request(key, method), storage);
    assert.equal(response.status, status);
    assert.equal(response.headers.get("Cache-Control"), "no-store");
    if (method === "HEAD") assert.equal(await response.text(), "");
    if (status === 405) assert.equal(response.headers.get("Allow"), "GET, HEAD");
  }
  assert.deepEqual(storage.calls, []);
});

test("Given_current_authority_When_conditionals_ranges_or_HEAD_arrive_Then_mutable_bytes_are_fresh_and_uncacheable", async () => {
  for (const method of ["GET", "HEAD"]) {
    const storage = archive();
    const response = await serve(request(active, method, { Range: "bytes=0-0", "If-None-Match": "*" }), storage);
    assert.equal(response.status, 200);
    assert.equal(response.headers.get("Cache-Control"), "no-store");
    assert.equal(response.headers.get("Content-Length"), "10");
    assert.equal(await response.text(), method === "HEAD" ? "" : "0123456789");
    assert.deepEqual(storage.calls, method === "HEAD" ? [["head", active]] : [["get", active, undefined]]);
  }
});

test("Given_immutable_files_When_GET_HEAD_ETag_or_Range_arrive_Then_stream_and_HTTP_metadata_agree", async () => {
  for (const [method, headers, status, content] of [
    ["GET", {}, 200, "0123456789"], ["HEAD", { Range: "bytes=2-4" }, 200, ""],
    ["GET", { "If-None-Match": 'W/"one"' }, 304, ""],
    ["HEAD", { "If-None-Match": "*" }, 304, ""],
    ["GET", { Range: "bytes=2-4" }, 206, "234"],
    ["GET", { Range: "bytes=-2", "If-Range": '"one"' }, 206, "89"],
    ["GET", { Range: "bytes=2-", "If-Range": '"old"' }, 200, "0123456789"],
    ["GET", { Range: "bytes=2-", "If-Range": 'W/"one"' }, 200, "0123456789"],
    ["GET", { Range: "bytes=2-", "If-Range": "Wed, 14 Oct 2026 00:00:00 GMT" }, 200, "0123456789"],
    ["GET", { Range: "bytes=10-" }, 416, "Repository request failed (416).\n"]]) {
    const storage = archive();
    const response = await serve(request(pool, method, headers), storage);
    assert.equal(response.status, status);
    assert.equal(await response.text(), content);
    assert.equal(response.headers.get("X-Content-Type-Options"), "nosniff");
    if (status === 416) {
      assert.equal(response.headers.get("Content-Range"), "bytes */10");
      assert.equal(response.headers.get("Cache-Control"), "no-store");
    } else {
      assert.equal(response.headers.get("Cache-Control"), "public, max-age=31536000, immutable");
      assert.equal(response.headers.get("ETag"), '"one"');
      if (status === 304) assert.equal(response.headers.get("Content-Length"), null);
      else assert.equal(response.headers.get("Content-Length"), method === "HEAD" ? "10" : String(content.length));
    }
    if (method === "HEAD" || [304, 416].includes(status)) assert.equal(storage.calls.length, 1);
    else assert.deepEqual(storage.calls[1][2].onlyIf, { etagMatches: "one" });
  }
});

test("Given_missing_corrupt_changed_or_failing_storage_When_read_Then_fail_closed_without_sensitive_errors", async () => {
  const events = [];
  const log = console.error;
  console.error = event => events.push(event);
  try {
    for (const [key, method, override, status] of [
      [pool, "GET", { head: async () => null }, 404],
      [active, "GET", { get: async () => null }, 404],
      [pool, "HEAD", { head: async () => null }, 404],
      [pool, "GET", { get: async () => null }, 503],
      [pool, "GET", { get: async () => metadata }, 503],
      [pool, "GET", { head: async () => { throw new Error("secret/path"); } }, 503],
      [active, "GET", { get: async () => { throw new Error("secret/path"); } }, 503],
      [pool, "HEAD", { head: async () => { throw new Error("secret/path"); } }, 503],
    ]) {
      const response = await serve(request(key, method), archive(override));
      assert.equal(response.status, status);
      assert.equal(response.headers.get("Cache-Control"), "no-store");
      assert.ok(!(await response.text()).includes("secret"));
    }
    for (const [key, maximum] of [[active, 1048576], [pool, 1073741824], [pool + ".sha256", 1024]]) {
      for (const size of [0, maximum + 1, -1, NaN, Infinity]) {
        let cancelled = false;
        const body = new ReadableStream({ cancel() { cancelled = true; } });
        const storage = archive({ head: async () => ({ ...metadata, size }),
          get: async () => ({ ...metadata, size, body }) });
        assert.equal((await serve(request(key), storage)).status, 503);
        if (key === active) assert.equal(cancelled, true);
      }
    }
    for (const changed of [{ etag: "two" }, { size: 11 }, { range: { offset: 1, length: 1 } }]) {
      let cancelled = false;
      const body = new ReadableStream({ cancel() { cancelled = true; } });
      const storage = archive({ get: async () => ({ ...metadata, ...changed, body }) });
      const response = await serve(request(pool, "GET", { Range: "bytes=2-4" }), storage);
      assert.equal(response.status, 503);
      assert.equal(cancelled, true);
    }
    assert.deepEqual(events, Array(3).fill('{"event":"native_archive_read_failed"}'));
  } finally { console.error = log; }
});

test("Given_a_lazy_package_stream_When_response_is_created_Then_package_is_not_buffered", async () => {
  let consumed = false;
  const body = new ReadableStream({ pull(controller) {
    consumed = true; controller.enqueue(new TextEncoder().encode("0123456789")); controller.close();
  } }, { highWaterMark: 0 });
  const storage = archive({ get: async () => ({ ...metadata, body }) });
  const response = await serve(request(), storage);
  assert.equal(consumed, false);
  assert.equal(await response.text(), "0123456789");
  assert.equal(consumed, true);
});
