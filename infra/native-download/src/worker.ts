import { byteRange, matchesEtag, objectPolicy } from "./policy.ts";
import type { ObjectPolicy } from "./policy.ts";

type Archive = Pick<R2Bucket, "get" | "head">;

function failure(status: number, head: boolean, extra?: Record<string, string>): Response {
  return new Response(head ? null : `Repository request failed (${status}).\n`, {
    status, headers: { "Cache-Control": "no-store", "Content-Type": "text/plain; charset=utf-8",
      "X-Content-Type-Options": "nosniff", ...extra },
  });
}

function valid(object: R2Object, policy: ObjectPolicy): boolean {
  return Number.isSafeInteger(object.size) && object.size > 0 && object.size <= policy.maximum;
}

function headers(object: R2Object, policy: ObjectPolicy): Headers {
  const result = new Headers({
    "Content-Type": policy.type,
    "Content-Length": String(object.size),
    "Cache-Control": policy.mutable ? "no-store" : "public, max-age=31536000, immutable",
    "ETag": object.httpEtag,
    "X-Content-Type-Options": "nosniff",
  });
  if (!policy.mutable) result.set("Accept-Ranges", "bytes");
  return result;
}

/** The binding is the sole dependency. No fallback fetch, Cache API or writes. */
export async function serve(request: Request, archive: Archive): Promise<Response> {
  const head = request.method === "HEAD";
  const policy = objectPolicy(request.url);
  if (!policy) return failure(404, head);
  if (request.method !== "GET" && !head) return failure(405, false, { Allow: "GET, HEAD" });
  try {
    // Mutable authority is always retrieved afresh; conditionals/ranges cannot hide an update.
    if (policy.mutable && !head) {
      const object = await archive.get(policy.key);
      if (!object) return failure(404, false);
      if (!valid(object, policy)) {
        await object.body.cancel();
        return failure(503, false);
      }
      return new Response(object.body, { headers: headers(object, policy) });
    }
    const metadata = await archive.head(policy.key);
    if (!metadata) return failure(404, head);
    if (!valid(metadata, policy)) return failure(503, head);
    const responseHeaders = headers(metadata, policy);
    if (!policy.mutable && matchesEtag(request.headers.get("If-None-Match"), metadata.httpEtag)) {
      responseHeaders.delete("Content-Length");
      return new Response(null, { status: 304, headers: responseHeaders });
    }
    // RFC 9110: Range applies to GET; HEAD describes the complete representation.
    if (head) return new Response(null, { headers: responseHeaders });
    const rawRange = request.headers.get("Range");
    const ifRange = request.headers.get("If-Range");
    const useRange = rawRange !== null && (ifRange === null || ifRange === metadata.httpEtag);
    const range = useRange ? byteRange(rawRange, metadata.size) : null;
    if (useRange && !range) return failure(416, false, { "Content-Range": `bytes */${metadata.size}` });
    const object = await archive.get(policy.key, {
      onlyIf: { etagMatches: metadata.etag },
      ...(range ? { range } : {}),
    });
    if (!object) return failure(503, false);
    if (!("body" in object)) return failure(503, false);
    if (object.etag !== metadata.etag || object.size !== metadata.size ||
        (range && (!object.range || !("offset" in object.range) ||
          object.range.offset !== range.offset || object.range.length !== range.length))) {
      await object.body.cancel();
      return failure(503, false);
    }
    if (range) {
      responseHeaders.set("Content-Length", String(range.length));
      responseHeaders.set("Content-Range", `bytes ${range.offset}-${range.offset + range.length - 1}/${metadata.size}`);
    }
    return new Response(object.body, { status: range ? 206 : 200, headers: responseHeaders });
  } catch {
    console.error(JSON.stringify({ event: "native_archive_read_failed" }));
    return failure(503, head);
  }
}

export default {
  fetch(request, env) { return serve(request, env.ARCHIVE); },
} satisfies ExportedHandler<Env>;
