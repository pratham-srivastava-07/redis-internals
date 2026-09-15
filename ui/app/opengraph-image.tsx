import { ImageResponse } from "next/og";

export const alt = "Vynk — The Redis idea, rewritten";
export const size = { width: 1200, height: 630 };
export const contentType = "image/png";

export default function OpenGraphImage() {
  return new ImageResponse(
    <div
      style={{
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        background: "#e9e5db",
        color: "#172125",
        padding: "72px 78px",
        fontFamily: "Arial, sans-serif",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          fontSize: 34,
          fontWeight: 700,
        }}
      >
        vynk<span style={{ color: "#245be7" }}>/</span>
      </div>
      <div style={{ display: "flex", flexDirection: "column", gap: 28 }}>
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            fontSize: 92,
            lineHeight: 0.95,
            letterSpacing: "-5px",
            fontWeight: 700,
          }}
        >
          <span>The Redis idea,</span>
          <span>rewritten.</span>
        </div>
        <div style={{ color: "#4f5b62", fontSize: 28 }}>
          From scratch in Rust, with smarter cache admission.
        </div>
      </div>
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          color: "#4f5b62",
          fontSize: 18,
        }}
      >
        <span>RESP2 · Approximate LRU · TinyLFU-style admission</span>
        <span>github.com/pratham-srivastava-07/vynk</span>
      </div>
    </div>,
    size,
  );
}
