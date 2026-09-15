import CacheArt from "@/components/cache-art";
import CachePlayground from "@/components/cache-playground";
import { CommandExamples, CopyButton, Navigation } from "@/components/controls";
import { INSTALL, REPO, SOURCE } from "@/lib/site";

const faqs = [
  [
    "Is this a drop-in replacement for Redis?",
    "No. Vynk is a Redis implementation with extra features, written from scratch in Rust. It speaks RESP2 and works with redis-cli, but does not implement the full Redis command set, replication, clustering, authentication, or transactions.",
  ],
  [
    "Does admission control change SET?",
    "Normal SET keeps its existing behavior. Opt in with CACHE.PUT: a valid fill returns STORED or REJECTED. On rejection, your application still returns the value it fetched from its backend; it simply does not cache that value.",
  ],
  [
    "What happens when the server restarts?",
    "Accepted values, expiration deadlines, and eviction deletions are restored from the append-only file. Frequency history and metrics start fresh. AOF synchronization is periodic, so an acknowledged write is not a guarantee against power-loss data loss.",
  ],
  [
    "Does TinyLFU always perform better?",
    "No policy wins every workload. Admission can protect popular data from one-time traffic, but can also reject useful newcomers. The implementation has a frequency sketch and aging, without a W-TinyLFU recency window. Comparative Redis benchmarks are still pending.",
  ],
];

export default function Home() {
  return (
    <>
      <a className="skip-link" href="#main">
        Skip to content
      </a>
      <Navigation />
      <main id="main">
        <section className="hero shell" id="top">
          <div className="hero-copy">
            <div className="release-label">
              <span className="status-dot" />
              Vynk is live on crates.io{" "}
              <span className="release-version">v0.1.0</span>
            </div>
            <h1>
              Make room for
              <br />
              what comes
              <br />
              <span className="outline-word">back.</span>
            </h1>
            <p className="hero-description">
              The Redis idea, rewritten. From scratch in Rust,
              <br className="desktop-break" /> with a little more judgment about
              what gets to stay.
            </p>
            <div className="hero-actions">
              <a href="#playground" className="button primary">
                See admission in action <span aria-hidden="true">↗</span>
              </a>
              <a href="#start" className="text-link">
                Install Vynk <span aria-hidden="true">↓</span>
              </a>
            </div>
            <div className="hero-footnote">
              RESP2 compatible <span>/</span> Approximate LRU <span>/</span>{" "}
              TinyLFU-style admission
            </div>
          </div>
          <CacheArt />
        </section>
        <div className="spec-strip">
          <div className="shell spec-inner">
            <span className="spec-intro">
              Small surface.
              <br />
              <strong>Interesting internals.</strong>
            </span>
            <div>
              <strong>Rust</strong>
              <span>from the socket up</span>
            </div>
            <div>
              <strong>RESP2</strong>
              <span>bring your redis-cli</span>
            </div>
            <div>
              <strong>9 KiB</strong>
              <span>sketch + doorkeeper</span>
            </div>
            <div>
              <strong>AOF</strong>
              <span>state across restarts</span>
            </div>
          </div>
        </div>
        <section className="section shell" id="playground">
          <div className="section-heading">
            <div>
              <p className="eyebrow">A little selectivity goes a long way</p>
              <h2>
                Not every request
                <br />
                deserves a room.
              </h2>
            </div>
            <p>
              A scan can push useful data out of a cache.
              <br />
              Play the same traffic through two policies.
              <br />
              Watch what stays, and what gets replaced.
            </p>
          </div>
          <CachePlayground />
        </section>
        <section className="principle-section">
          <div className="shell principle-inner">
            <span className="section-index">The extra decision</span>
            <h2>
              Before asking what to evict,
              <br />
              ask what to let in.
            </h2>
            <div className="principle-process">
              <div>
                <span className="process-number">01</span>
                <h3>Observe demand.</h3>
                <p>
                  Every GET contributes to recent history. Even the requests
                  that miss.
                </p>
              </div>
              <div>
                <span className="process-number">02</span>
                <h3>Compare the candidates.</h3>
                <p>
                  Would the incoming key be reused more than the LRU eviction
                  candidate?
                </p>
              </div>
              <div>
                <span className="process-number">03</span>
                <h3>Make the trade.</h3>
                <p>
                  Admit the stronger candidate. Preserve the resident on a tie.
                  Age the history.
                </p>
              </div>
            </div>
            <a className="text-link" href={`${SOURCE}/src/admission.rs`}>
              Read the admission code <span aria-hidden="true">↗</span>
            </a>
          </div>
        </section>
        <section className="section shell commands-section" id="commands">
          <div className="commands-copy">
            <p className="eyebrow">Familiar commands. One new choice.</p>
            <h2>
              Same conversation.
              <br />
              Smarter fills.
            </h2>
            <p>
              Read with GET. Fetch on a miss. Use CACHE.PUT when a fill can be
              declined. Your application stays in control.
            </p>
            <a href={`${REPO}#supported-commands`} className="text-link">
              Explore the command set <span aria-hidden="true">↗</span>
            </a>
            <div className="command-note">
              <span>+STORED</span>
              <span>+REJECTED</span>
              <p>
                Both are valid admission outcomes.
                <br />A rejected fill is not a failed user request.
              </p>
            </div>
          </div>
          <CommandExamples />
        </section>
        <section className="section shell engineering" id="under-the-hood">
          <div className="section-heading">
            <div>
              <p className="eyebrow">Built to be understood</p>
              <h2>
                Open the hood.
                <br />
                Follow the bytes.
              </h2>
            </div>
            <p>
              One event loop. Explicit data structures.
              <br />A small server you can read end to end.
            </p>
          </div>
          <div className="architecture-flow">
            <span>TCP / RESP2</span>
            <b aria-hidden="true">→</b>
            <span>Command</span>
            <b aria-hidden="true">→</b>
            <span className="flow-highlight">
              Admission check<small>CACHE.PUT</small>
            </span>
            <b aria-hidden="true">→</b>
            <span>Memory + AOF</span>
          </div>
          <div className="engineering-grid">
            <article>
              <span className="detail-marker">[ network ]</span>
              <h3>One loop, many clients.</h3>
              <p>
                Mio readiness events, buffered replies, and pipelined commands.
                Partial reads and slow readers are handled without a thread per
                connection.
              </p>
            </article>
            <article>
              <span className="detail-marker">[ memory ]</span>
              <h3>A small pool of possibilities.</h3>
              <p>
                Sample five keys. Keep sixteen candidates. Estimate idle time
                with a wrapping 24-bit clock stored in a u32.
              </p>
            </article>
            <article>
              <span className="detail-marker">[ persistence ]</span>
              <h3>Write it down. Read it back.</h3>
              <p>
                Record accepted state and absolute expiry deadlines. Replay the
                append-only file on restart, recovering an incomplete final
                frame.
              </p>
            </article>
          </div>
        </section>
        <section className="evidence-section" id="evidence">
          <div className="shell evidence-inner">
            <div className="evidence-copy">
              <p className="eyebrow">Evidence, with the boundaries included</p>
              <h2>
                Test the behavior.
                <br />
                Then make the claim.
              </h2>
              <p>
                Commands, malformed frames, expiration, admission, and restart
                recovery. Verified in debug and release on Windows.
              </p>
              <a
                href={`${SOURCE}/tests/feature_audit.rs`}
                className="text-link"
              >
                Inspect the test suite <span aria-hidden="true">↗</span>
              </a>
            </div>
            <div className="test-report">
              <div className="test-report-head">
                <span>
                  <i className="status-dot" />
                  Latest recorded verification
                </span>
                <span>2026-09-15</span>
              </div>
              <div className="test-totals">
                <div>
                  <strong>23</strong>
                  <span>unit tests passed</span>
                </div>
                <div>
                  <strong>54</strong>
                  <span>integration tests passed</span>
                </div>
              </div>
              <div className="test-result">
                <span>Debug + release</span>
                <span>
                  0 failed · 0 ignored <b>✓</b>
                </span>
              </div>
              <div className="test-command">
                <code>cargo test -- --test-threads=1</code>
                <CopyButton text="cargo test -- --test-threads=1" />
              </div>
              <p className="evidence-caveat">
                These are correctness checks, not speed benchmarks. Throughput,
                p99 latency, and comparisons with Redis have not been measured.
              </p>
            </div>
          </div>
        </section>
        <section className="section shell faq-section" id="questions">
          <div>
            <p className="eyebrow">A few useful boundaries</p>
            <h2>
              Before you
              <br />
              build on it.
            </h2>
            <p>
              An experiment you can inspect,
              <br />
              with room to grow.
            </p>
          </div>
          <div className="faq-list">
            {faqs.map(([question, answer]) => (
              <details key={question}>
                <summary>
                  {question}
                  <span aria-hidden="true">+</span>
                </summary>
                <p>{answer}</p>
              </details>
            ))}
          </div>
        </section>
        <section className="start-section shell" id="start">
          <div>
            <p className="eyebrow">Your terminal is the starting line</p>
            <h2>
              Go on.
              <br />
              Look inside.
            </h2>
            <p>
              Install Vynk, open a second terminal,
              <br />
              and bring your first key to life.
            </p>
            <a href={REPO} className="button primary">
              Explore on GitHub <span aria-hidden="true">↗</span>
            </a>
          </div>
          <div className="install-block">
            <div className="install-title">
              <span>Install Vynk</span>
              <span>Rust · edition 2026</span>
            </div>
            <div className="install-command">
              <code>{INSTALL}</code>
              <CopyButton text={INSTALL} />
            </div>
            <div className="install-command">
              <code>
                mkdir vynk-data
                <br />
                cd vynk-data
                <br />
                vynk
              </code>
              <CopyButton text={"mkdir vynk-data\ncd vynk-data\nvynk"} />
            </div>
            <div className="install-title second-terminal">
              <span>In another terminal</span>
            </div>
            <div className="install-command">
              <code>redis-cli -p 7379</code>
              <CopyButton text="redis-cli -p 7379" />
            </div>
            <p>
              Requires Rust and redis-cli. Server binds to localhost.
              <br />
              Writes appendonly.aof in your current directory.
            </p>
          </div>
        </section>
      </main>
      <footer className="site-footer">
        <div className="shell">
          <div className="footer-top">
            <div className="footer-intro">
              <a className="brand" href="#top">
                <span className="brand-icon" aria-hidden="true">
                  <i />
                  <i />
                  <i />
                </span>
                vynk<span className="brand-bracket">/</span>
              </a>
              <p>
                A small Rust server exploring
                <br />
                what deserves to stay in memory.
              </p>
              <div className="footer-command">
                <code>$ cargo install vynk --locked</code>
                <CopyButton text="cargo install vynk --locked" />
              </div>
              <span className="footer-version">
                <i className="status-dot" />
                v0.1.0 · MIT licensed
              </span>
            </div>
            <div className="footer-column">
              <h3>On this page</h3>
              <a href="#playground">Playground</a>
              <a href="#commands">Commands</a>
              <a href="#under-the-hood">Architecture</a>
              <a href="#evidence">Evidence</a>
              <a href="#start">Get started</a>
            </div>
            <div className="footer-column">
              <h3>Read the source</h3>
              <a href={`${REPO}#readme`}>README</a>
              <a href={`${SOURCE}/src/admission.rs`}>Admission</a>
              <a href={`${SOURCE}/src/eviction.rs`}>Eviction</a>
              <a href={`${SOURCE}/tests/feature_audit.rs`}>Tests</a>
            </div>
            <div className="footer-column">
              <h3>Project</h3>
              <a href="https://crates.io/crates/vynk">crates.io ↗</a>
              <a href={REPO}>GitHub ↗</a>
              <a href={`${REPO}/issues`}>Issues ↗</a>
              <a href="https://github.com/pratham-srivastava-07">
                Built by Pratham ↗
              </a>
            </div>
          </div>
          <div className="footer-bottom">
            <span>© 2026 Vynk</span>
            <span>Independent project. Not affiliated with Redis.</span>
            <a href="#top">Back to top ↑</a>
          </div>
          <div className="footer-wordmark" aria-hidden="true">
            vynk<span>_</span>
          </div>
        </div>
      </footer>
    </>
  );
}
