"use client";
import { useEffect, useState } from "react";
import { REPO } from "@/lib/site";

function copyFallback(text: string) {
  const textarea = document.createElement("textarea");
  textarea.value = text;
  textarea.setAttribute("readonly", "");
  textarea.style.position = "fixed";
  textarea.style.opacity = "0";
  document.body.appendChild(textarea);
  textarea.select();
  const copied = document.execCommand("copy");
  textarea.remove();
  return copied;
}

export function CopyButton({
  text,
  label = "Copy command",
}: {
  text: string;
  label?: string;
}) {
  const [state, setState] = useState("idle");
  useEffect(() => {
    if (state === "idle") return;
    const timer = setTimeout(() => setState("idle"), 2400);
    return () => clearTimeout(timer);
  }, [state]);
  return (
    <button
      className="copy-button"
      aria-label={state === "copied" ? "Copied" : label}
      onClick={async () => {
        try {
          if (!navigator.clipboard?.writeText)
            throw new Error("Clipboard unavailable");
          await navigator.clipboard.writeText(text);
          setState("copied");
        } catch {
          setState(copyFallback(text) ? "copied" : "failed");
        }
      }}
    >
      <span aria-hidden="true">{state === "copied" ? "✓" : "⧉"}</span>
      <span className="copy-status" role="status">
        {state === "copied"
          ? "Copied"
          : state === "failed"
            ? "Copy failed"
            : "Copy"}
      </span>
    </button>
  );
}
export function Navigation() {
  const [open, setOpen] = useState(false);
  return (
    <header className="site-header">
      <div className="nav-inner">
        <a className="brand" href="#top" aria-label="Vynk home">
          <span className="brand-icon" aria-hidden="true">
            <i />
            <i />
            <i />
          </span>
          vynk<span className="brand-bracket">/</span>
        </a>
        <button
          className="menu-button"
          aria-expanded={open}
          aria-controls="main-nav"
          onClick={() => setOpen(!open)}
        >
          {open ? "Close −" : "Menu +"}
        </button>
        <nav
          id="main-nav"
          className={open ? "main-nav is-open" : "main-nav"}
          aria-label="Main navigation"
        >
          <a href="#playground" onClick={() => setOpen(false)}>
            Playground
          </a>
          <a href="#under-the-hood" onClick={() => setOpen(false)}>
            Under the hood
          </a>
          <a href="#evidence" onClick={() => setOpen(false)}>
            Evidence
          </a>
          <a href={REPO} className="nav-github">
            GitHub <span aria-hidden="true">↗</span>
          </a>
        </nav>
      </div>
    </header>
  );
}
const examples = [
  {
    name: "Store & read",
    caption: "The familiar starting point.",
    lines: [
      ['SET product:42 "Mechanical keyboard"', "OK"],
      ["GET product:42", '\"Mechanical keyboard\"'],
      ["OBJECT ENCODING product:42", '\"embstr\"'],
    ],
  },
  {
    name: "Cache admission",
    caption: "Let useful data earn its place.",
    lines: [
      ["GET product:99", "(nil)"],
      ['CACHE.PUT product:99 "Studio headphones" EX 60', "STORED"],
      ["GET product:99", '\"Studio headphones\"'],
    ],
  },
  {
    name: "Expiration",
    caption: "Give temporary data a deadline.",
    lines: [
      ['SET session:42 "active" EX 60', "OK"],
      ["TTL session:42", "(integer) 60"],
      ["EXPIRE session:42 0", "(integer) 1"],
      ["GET session:42", "(nil)"],
    ],
  },
];
export function CommandExamples() {
  const [active, setActive] = useState(0);
  const example = examples[active];
  return (
    <div className="command-window">
      <div className="window-bar">
        <span>
          <i className="status-dot" />
          127.0.0.1:7379
        </span>
        <span>redis-cli</span>
      </div>
      <div
        className="command-tabs"
        role="tablist"
        aria-label="Command examples"
      >
        {examples.map((item, index) => (
          <button
            key={item.name}
            role="tab"
            id={`command-tab-${index}`}
            aria-selected={active === index}
            aria-controls="command-panel"
            tabIndex={active === index ? 0 : -1}
            onKeyDown={(e) => {
              if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
              e.preventDefault();
              const next =
                (active + (e.key === "ArrowRight" ? 1 : -1) + examples.length) %
                examples.length;
              setActive(next);
              document.getElementById(`command-tab-${next}`)?.focus();
            }}
            onClick={() => setActive(index)}
          >
            {item.name}
          </button>
        ))}
      </div>
      <div
        id="command-panel"
        className="command-panel"
        role="tabpanel"
        aria-labelledby={`command-tab-${active}`}
      >
        <p className="terminal-comment"># {example.caption}</p>
        {example.lines.map(([command, output], index) => (
          <div className="terminal-pair" key={`${active}-${index}`}>
            <code>
              <span className="prompt">❯ </span>
              {command}
            </code>
            <samp>{output}</samp>
          </div>
        ))}
      </div>
      <div className="terminal-foot">
        <span>
          Example session · {active === 1 ? "free capacity assumed" : "RESP2"}
        </span>
        <CopyButton
          text={example.lines.map((line) => line[0]).join("\n")}
          label="Copy example commands"
        />
      </div>
    </div>
  );
}
