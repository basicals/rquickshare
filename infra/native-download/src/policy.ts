export const BASE_URI = "https://download.morozhenko.com/dist/rQuickShare/";
export const SUITE = "rquickshare-native-ubuntu26.04";

export interface ObjectPolicy {
  key: string;
  mutable: boolean;
  type: string;
  maximum: number;
}

/** Project only public repository artifacts; never project audit snapshot keys. */
export function objectPolicy(raw: string): ObjectPolicy | null {
  if (raw.length > 2048 || /[\x00-\x20\\]/.test(raw) || /\/\.{1,2}(?:\/|$)/.test(raw)) return null;
  let url: URL;
  try { url = new URL(raw); } catch { return null; }
  if (url.origin !== "https://download.morozhenko.com" || url.username || url.password ||
      url.search || url.hash || raw.includes("?") || raw.includes("#") ||
      !url.pathname.startsWith("/dist/rQuickShare/")) return null;
  const encoded = url.pathname.slice("/dist/rQuickShare/".length);
  // Only '+' has a legitimate encoded spelling in the fixed repository grammar.
  if (/%(?!2[bB])/u.test(encoded)) return null;
  const key = encoded.replace(/%2b/gi, "+");
  if (key === `dists/${SUITE}/InRelease`)
    return { key, mutable: true, type: "text/plain; charset=utf-8", maximum: 1024 * 1024 };
  if (new RegExp(`^dists/${SUITE}/main/binary-(amd64|arm64)/by-hash/SHA256/[a-f0-9]{64}$`).test(key))
    return { key, mutable: false, type: "application/octet-stream", maximum: 1024 * 1024 };
  const match = /^pool\/main\/r\/rquickshare\/rquickshare_[0-9]+\.[0-9]+\.[0-9]+\+native([1-9][0-9]{0,9})_(amd64|arm64)\.deb(\.sha256)?$/.exec(key);
  if (!match || Number(match[1]) > 2147483647) return null;
  return { key, mutable: false,
    type: match[3] ? "text/plain; charset=utf-8" : "application/vnd.debian.binary-package",
    maximum: match[3] ? 1024 : 1024 * 1024 * 1024 };
}

export interface ByteRange { offset: number; length: number }

/** One byte interval only; numeric overflow and empty/unsatisfiable ranges fail. */
export function byteRange(raw: string, size: number): ByteRange | null {
  const match = /^bytes=([0-9]*)-([0-9]*)$/.exec(raw);
  if (!match || !Number.isSafeInteger(size) || size <= 0) return null;
  const first = match[1] ?? "", last = match[2] ?? "";
  if (!first && !last) return null;
  const a = Number(first), b = Number(last);
  if (!Number.isSafeInteger(a) || !Number.isSafeInteger(b)) return null;
  if (!first) {
    if (b === 0) return null;
    const length = Math.min(b, size);
    return { offset: size - length, length };
  }
  if (a >= size || (last && b < a)) return null;
  return { offset: a, length: (last ? Math.min(b, size - 1) : size - 1) - a + 1 };
}

/** Weak comparison is correct for GET/HEAD If-None-Match. Bound parsing work. */
export function matchesEtag(raw: string | null, etag: string): boolean {
  return raw !== null && raw.length <= 4096 &&
    raw.split(",").some(part => {
      const value = part.trim();
      return value === "*" || value.replace(/^W\//, "") === etag;
    });
}
