import assert from "node:assert/strict";
import test from "node:test";
import { BASE_URI, SUITE, objectPolicy, byteRange, matchesEtag } from "../src/policy.ts";

export const active = `dists/${SUITE}/InRelease`;
export const pool = "pool/main/r/rquickshare-native/rquickshare-native_1.0.0-2_amd64.deb";

test("Given_public_artifacts_When_projected_Then_only_exact_repository_keys_are_exposed", () => {
  // Arrange / Act / Assert: grammar matches native_apt/native_deb production contracts.
  for (const architecture of ["amd64", "arm64"]) {
    for (const suffix of ["", ".sha256"]) {
      const key = pool.replace("amd64", architecture) + suffix;
      for (const spelling of [key])
        assert.equal(objectPolicy(BASE_URI + spelling).key, key);
    }
    const key = `dists/${SUITE}/main/binary-${architecture}/by-hash/SHA256/${"a".repeat(64)}`;
    assert.equal(objectPolicy(BASE_URI + key).maximum, 1048576);
  }
  assert.equal(objectPolicy(BASE_URI + active).mutable, true);
  assert.equal(objectPolicy(BASE_URI + pool).maximum, 1073741824);
  assert.equal(objectPolicy(BASE_URI + pool + ".sha256").maximum, 1024);
  assert.ok(objectPolicy(BASE_URI + pool.replace("-2_", "-2147483647_")));
});

test("Given_untrusted_URLs_When_projected_Then_private_sibling_and_ambiguous_paths_are_rejected", () => {
  const urls = ["invalid", BASE_URI, BASE_URI + "snapshots/x/" + active,
    BASE_URI + active.replace("InRelease", "Release"),
    BASE_URI + `dists/${SUITE}/main/binary-amd64/Packages.gz`,
    BASE_URI.replace("rQuickShare", "Picturix") + "latest.json",
    BASE_URI.replace("rQuickShare", "rquickshare") + active,
    BASE_URI.replace("https:", "http:") + active,
    BASE_URI.replace("download.", "evil.") + active,
    BASE_URI.replace("https://", "https://user:password@") + active,
    BASE_URI.replace(".com/", ".com:444/") + active,
    BASE_URI + active + "?", BASE_URI + active + "#", BASE_URI + active + "?x=y",
    BASE_URI + active + "/", BASE_URI + "../" + active,
    BASE_URI + "%2e%2e/" + active, BASE_URI + active.replace("/", "%2f"),
    BASE_URI + active.replace("/", "%5c"), BASE_URI + "%00" + active,
    BASE_URI + "%252e" + active, BASE_URI + "%" + active,
    BASE_URI + active + "\n", BASE_URI + active.replace("/", "\\"),
    BASE_URI + "x".repeat(2049), BASE_URI + pool.replace("-2_", "-0_"),
    BASE_URI + pool.replace("-2_", "-2147483648_"),
    BASE_URI + pool.replace("-2_", "-99999999999_"),
    BASE_URI + pool.replace("-2_", "-2.debug_"),
    BASE_URI + pool.replace("amd64", "i386"),
    BASE_URI + pool.replace("1.0.0-2", "01.0.0-2"),
    BASE_URI + pool.replace("1.0.0-2", "1.0.0%2B2"),
    BASE_URI + pool.replace("1.0.0-2", "1.0.0-02"),
    BASE_URI + `dists/${SUITE}/main/binary-amd64/by-hash/SHA256/${"A".repeat(64)}`];
  for (const url of urls) assert.equal(objectPolicy(url), null, url);
});

test("Given_byte_ranges_When_parsed_Then_boundaries_suffixes_and_overflow_are_exact", () => {
  for (const [raw, expected] of [["bytes=0-0", [0, 1]], ["bytes=9-9", [9, 1]],
    ["bytes=0-", [0, 10]], ["bytes=4-", [4, 6]], ["bytes=3-100", [3, 7]],
    ["bytes=-1", [9, 1]], ["bytes=-10", [0, 10]], ["bytes=-11", [0, 10]]])
    assert.deepEqual(byteRange(raw, 10), { offset: expected[0], length: expected[1] });
  for (const raw of ["", "bytes=-", "bytes=-0", "bytes=10-", "bytes=9-8", "bytes=0-1,3-4",
    "items=0-1", "bytes= 0-1", "bytes=0-NaN", "bytes=0-Infinity",
    "bytes=9007199254740992-", "bytes=-9007199254740992"])
    assert.equal(byteRange(raw, 10), null, raw);
  for (const size of [0, -1, NaN, Infinity, 9007199254740992]) assert.equal(byteRange("bytes=0-", size), null);
});

test("Given_conditional_ETags_When_compared_Then_weak_lists_and_wildcards_match_without_unbounded_work", () => {
  for (const value of ['"one"', 'W/"one"', '*', '"other", W/"one"'])
    assert.equal(matchesEtag(value, '"one"'), true);
  for (const value of [null, "", '"other"', '"one-more"', "x".repeat(4097)])
    assert.equal(matchesEtag(value, '"one"'), false);
});
