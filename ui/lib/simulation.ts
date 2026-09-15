export type Workload = "scan" | "repeat" | "shift";
export type Policy = "lru" | "admission";
export type Entry = { key: string; last: number };
export type Result = {
  entries: Entry[];
  counts: Record<string, number>;
  hits: number;
  misses: number;
  rejected: number;
  decision: string;
};
export const CAPACITY = 12;
export const hotKeys = Array.from(
  { length: CAPACITY },
  (_, i) => `p${String(i + 1).padStart(2, "0")}`,
);
export function traceFor(workload: Workload): string[] {
  if (workload === "scan")
    return [
      ...Array.from({ length: 18 }, (_, i) => `s${i + 1}`),
      ...hotKeys,
      ...hotKeys,
    ];
  if (workload === "repeat")
    return Array.from({ length: 42 }, (_, i) => hotKeys[i % CAPACITY]);
  return Array.from({ length: 72 }, (_, i) => `n${(i % 6) + 1}`);
}
export function simulate(
  workload: Workload,
  policy: Policy,
  steps: number,
): Result {
  const state: Result = {
    entries: hotKeys.map((key, last) => ({ key, last })),
    counts: Object.fromEntries(hotKeys.map((key) => [key, 5])),
    hits: 0,
    misses: 0,
    rejected: 0,
    decision: "12 popular keys warmed. Ready for traffic.",
  };
  traceFor(workload)
    .slice(0, steps)
    .forEach((key, tick) => {
      // Explanatory exact-counter model; the server uses a sketch and sampled LRU.
      if (tick > 0 && tick % 24 === 0)
        for (const name in state.counts)
          state.counts[name] = Math.floor(state.counts[name] / 2);
      state.counts[key] = (state.counts[key] ?? 0) + 1;
      const resident = state.entries.find((entry) => entry.key === key);
      if (resident) {
        state.hits++;
        resident.last = tick + CAPACITY;
        state.decision = `${key} → HIT. Served from cache.`;
        return;
      }
      state.misses++;
      const victim = state.entries.reduce((oldest, entry) =>
        entry.last < oldest.last ? entry : oldest,
      );
      if (
        policy === "admission" &&
        state.counts[key] <= (state.counts[victim.key] ?? 0)
      ) {
        state.rejected++;
        state.decision = `${key} → REJECTED. Frequency ${state.counts[key]} ≤ ${state.counts[victim.key] ?? 0} for ${victim.key}.`;
        return;
      }
      state.entries = state.entries.map((entry) =>
        entry === victim ? { key, last: tick + CAPACITY } : entry,
      );
      state.decision = `${key} → STORED. Replaces ${victim.key}.`;
    });
  return state;
}
