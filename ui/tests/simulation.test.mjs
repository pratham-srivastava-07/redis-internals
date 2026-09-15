import test from "node:test";
import assert from "node:assert/strict";
import { simulate, traceFor, CAPACITY } from "../lib/simulation.ts";

test("scan admission protects popular keys and saves modeled fetches", () => {
  const steps = traceFor("scan").length;
  const lru = simulate("scan", "lru", steps);
  const admission = simulate("scan", "admission", steps);
  assert.equal(admission.hits, 24);
  assert.equal(lru.hits, 12);
  assert.equal(admission.rejected, 18);
  assert.ok(admission.misses < lru.misses);
});
test("repeated popular reads hit equally", () => {
  for (const policy of ["lru", "admission"]) {
    const result = simulate("repeat", policy, 42);
    assert.equal(result.hits, 42);
    assert.equal(result.misses, 0);
  }
});
test("changing demand eventually admits new keys and exposes initial admission cost", () => {
  const admission = simulate("shift", "admission", 72);
  assert.ok(admission.entries.some((entry) => entry.key.startsWith("n")));
  assert.ok(admission.hits > 0);
  assert.ok(admission.misses > simulate("shift", "lru", 72).misses);
});
test("each step preserves capacity, key uniqueness, and request accounting", () => {
  for (const workload of ["scan", "repeat", "shift"])
    for (const policy of ["lru", "admission"]) {
      for (let step = 0; step <= traceFor(workload).length; step++) {
        const result = simulate(workload, policy, step);
        assert.equal(result.entries.length, CAPACITY);
        assert.equal(
          new Set(result.entries.map((entry) => entry.key)).size,
          CAPACITY,
        );
        assert.equal(result.hits + result.misses, step);
      }
    }
});
