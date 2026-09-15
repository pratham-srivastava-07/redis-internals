"use client";
import { useEffect, useState } from "react";
import { CAPACITY, simulate, traceFor, type Workload } from "@/lib/simulation";
const workloads: { id: Workload; title: string; note: string }[] = [
  {
    id: "scan",
    title: "One-time traffic",
    note: "18 unique requests arrive. Then the popular keys are requested again.",
  },
  {
    id: "repeat",
    title: "Popular keys",
    note: "The same 12 keys are requested repeatedly. Both policies should serve hits.",
  },
  {
    id: "shift",
    title: "Changing demand",
    note: "Six new keys become popular. Watch aging give the newcomers a chance.",
  },
];
export default function CachePlayground() {
  const [workload, setWorkload] = useState<Workload>("scan");
  const [step, setStep] = useState(0);
  const [playing, setPlaying] = useState(false);
  const trace = traceFor(workload);
  const done = step >= trace.length;
  useEffect(() => {
    if (!playing || done) return;
    const timer = setInterval(
      () => setStep((value) => Math.min(value + 1, trace.length)),
      230,
    );
    return () => clearInterval(timer);
  }, [playing, done, trace.length]);
  return (
    <div className="playground">
      <div className="playground-toolbar">
        <div
          className="workload-buttons"
          role="group"
          aria-label="Traffic scenario"
        >
          {workloads.map((item) => (
            <button
              key={item.id}
              aria-pressed={workload === item.id}
              onClick={() => {
                setWorkload(item.id);
                setStep(0);
                setPlaying(false);
              }}
            >
              {item.title}
            </button>
          ))}
        </div>
        <span className="simulation-badge">Interactive simulation</span>
      </div>
      <div className="traffic-strip">
        <span className="traffic-label">
          Incoming
          <br />
          <strong>{step ? trace[step - 1] : "ready"}</strong>
        </span>
        <div className="traffic-track" aria-hidden="true">
          {trace
            .slice(Math.max(0, step - 3), Math.max(0, step - 3) + 13)
            .map((key, index) => (
              <span
                key={`${key}-${index}`}
                className={`${key.startsWith("p") ? "popular" : "one-time"} ${index === (step > 3 ? 2 : step - 1) ? "current" : ""}`}
              >
                {key}
              </span>
            ))}
        </div>
        <span className="traffic-count">
          {String(step).padStart(2, "0")} / {trace.length}
        </span>
      </div>
      <div className="policy-comparison">
        {(["lru", "admission"] as const).map((policy) => {
          const result = simulate(workload, policy, step);
          return (
            <div className={`policy-panel ${policy}`} key={policy}>
              <div className="policy-title">
                <h3>{policy === "lru" ? "LRU only" : "LRU + admission"}</h3>
                <span>
                  {policy === "lru"
                    ? "Store every miss"
                    : "Keep what gets reused"}
                </span>
              </div>
              <div
                className="cache-slots"
                aria-label={`${policy === "lru" ? "LRU" : "Admission"} cache contents`}
              >
                {result.entries.map((entry, index) => (
                  <div
                    key={index}
                    className={`cache-slot ${entry.key.startsWith("p") ? "popular" : "one-time"}`}
                  >
                    <span>{entry.key}</span>
                    <span className="slot-mark" aria-hidden="true">
                      {entry.key.startsWith("p") ? "●" : "○"}
                    </span>
                  </div>
                ))}
              </div>
              <div className="policy-metrics">
                <div>
                  <strong>{result.hits}</strong>
                  <span>cache hits</span>
                </div>
                <div>
                  <strong>{result.misses}</strong>
                  <span>backend fetches*</span>
                </div>
                <div>
                  <strong>{result.rejected}</strong>
                  <span>fills rejected</span>
                </div>
              </div>
              <p className="decision-line">{result.decision}</p>
            </div>
          );
        })}
      </div>
      <div className="playground-controls">
        <div className="run-controls">
          <button
            className="button primary"
            onClick={() => {
              if (done) setStep(0);
              setPlaying(done ? true : !playing);
            }}
          >
            <span aria-hidden="true">{playing && !done ? "Ⅱ" : "▶"}</span>
            {done
              ? "Replay traffic"
              : playing
                ? "Pause"
                : step
                  ? "Continue"
                  : "Run traffic"}
          </button>
          <button
            className="reset-button"
            disabled={step === 0 && !playing}
            onClick={() => {
              setPlaying(false);
              setStep(0);
            }}
          >
            ↺ Reset
          </button>
          <button
            className="finish-button"
            disabled={done}
            onClick={() => {
              setPlaying(false);
              setStep(trace.length);
            }}
          >
            See result <span aria-hidden="true">↗</span>
          </button>
        </div>
        <div className="cache-legend">
          <span>
            <i className="legend-popular" />
            Popular key
          </span>
          <span>
            <i className="legend-once" />
            New key
          </span>
        </div>
      </div>
      <p className="scenario-note" aria-live="polite">
        {done
          ? "Trace complete. Switch the workload to explore another trade-off."
          : workloads.find((item) => item.id === workload)?.note}
      </p>
      <details className="model-note">
        <summary>
          How this demo works <span>+</span>
        </summary>
        <p>
          A {CAPACITY}-slot browser model, prefilled with popular keys. Both
          sides use exact LRU and exact frequency counters with aging every 24
          requests. The Rust server uses a frequency sketch, sampled LRU, a
          100-key limit, and a different aging interval. This is an explanation,
          not a benchmark. *Each simulated miss assumes one backend fetch. No
          requests are sent to a Redis server.
        </p>
      </details>
    </div>
  );
}
