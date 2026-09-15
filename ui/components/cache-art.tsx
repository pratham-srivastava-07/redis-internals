export default function CacheArt() {
  const tiles = Array.from({ length: 24 }, (_, i) => ({
    x: 132 + (i % 4) * 77,
    y: 120 + Math.floor(i / 4) * 47,
    hot: [0, 1, 4, 5, 8, 9, 10, 12, 13, 16, 20].includes(i),
  }));
  return (
    <div
      className="cache-art"
      role="img"
      aria-label="Isometric cache illustration: frequently requested blue keys stay inside an admission layer while one-time requests are filtered out."
    >
      <div className="art-coordinate top">FIG. 01 / SELECTIVE MEMORY</div>
      <svg viewBox="0 0 620 530" aria-hidden="true">
        <defs>
          <linearGradient id="chip-side" x1="0" y1="0" x2="1" y2="1">
            <stop stopColor="#0d34a4" />
            <stop offset="1" stopColor="#041745" />
          </linearGradient>
          <linearGradient id="chip-top" x1="0" y1="0" x2="1" y2="1">
            <stop stopColor="#6897ff" />
            <stop offset=".48" stopColor="#2863ec" />
            <stop offset="1" stopColor="#184ac6" />
          </linearGradient>
          <linearGradient id="silver" x1="0" y1="0" x2="0" y2="1">
            <stop stopColor="#f8fbff" />
            <stop offset="1" stopColor="#a6bada" />
          </linearGradient>
          <filter id="chip-shadow" x="-50%" y="-50%" width="200%" height="240%">
            <feDropShadow
              dx="0"
              dy="18"
              stdDeviation="18"
              floodColor="#0c2860"
              floodOpacity=".18"
            />
          </filter>
        </defs>
        <g stroke="#9bb4da" strokeWidth=".65" opacity=".25">
          {Array.from({ length: 13 }, (_, i) => (
            <path key={i} d={`M${i * 55 - 50} 0V530 M0 ${i * 48}H620`} />
          ))}
        </g>
        <g
          transform="translate(30 65) matrix(.87 -.4 .64 .38 -25 125)"
          filter="url(#chip-shadow)"
        >
          <path
            d="M99 88H455V417H99Z"
            fill="#345690"
            transform="translate(0 42)"
          />
          <path
            d="M99 88H455V417H99Z"
            fill="url(#silver)"
            stroke="#f8fbff"
            strokeWidth="2"
            transform="translate(0 30)"
          />
          <path
            d="M99 88H455V417H99Z"
            fill="#b7cae8"
            stroke="white"
            strokeWidth="2"
            transform="translate(0 13)"
          />
          <path
            d="M99 88H455V417H99Z"
            fill="#edf4ff"
            stroke="white"
            strokeWidth="2"
          />
          {tiles.map((tile, i) => (
            <g
              key={i}
              className="art-tile"
              style={{ animationDelay: `${i * 35}ms` }}
            >
              <path
                d={`M${tile.x} ${tile.y}h61v31h-61Z`}
                fill={tile.hot ? "url(#chip-side)" : "#a5b7ce"}
              />
              <path
                d={`M${tile.x} ${tile.y - 10}h61v27h-61Z`}
                fill={tile.hot ? "url(#chip-top)" : "url(#silver)"}
                stroke={tile.hot ? "#86adff" : "#fff"}
                strokeWidth="1.2"
              />
              <path
                d={`M${tile.x + 8} ${tile.y - 1}h16 m8 0h16`}
                stroke={tile.hot ? "#a8c8ff" : "#8196b4"}
                strokeWidth="2"
              />
            </g>
          ))}
          <path
            d="M87 66H467V436H87Z"
            fill="#4081ff"
            fillOpacity=".035"
            stroke="#2966e9"
            strokeWidth="1.5"
            strokeDasharray="6 5"
          />
        </g>
        <path
          d="M80 150H165L233 186"
          stroke="#2360df"
          fill="none"
          strokeWidth="1.5"
        />
        <circle cx="80" cy="150" r="4" fill="#2360df" />
        <path d="M405 342L460 373H535" stroke="#8095b7" fill="none" />
        <circle cx="535" cy="373" r="3" fill="#8095b7" />
        <text x="53" y="129" className="art-text">
          reused → retained
        </text>
        <text x="434" y="397" className="art-text muted">
          one-off → bypass
        </text>
        <g className="floating-key">
          <rect
            x="321"
            y="83"
            width="125"
            height="34"
            rx="3"
            fill="#f8fbff"
            stroke="#a7bfeb"
          />
          <circle cx="337" cy="100" r="3" fill="#2e67ea" />
          <text x="349" y="104" className="art-text">
            product:42
          </text>
          <path d="M382 117V156" stroke="#5984d0" strokeDasharray="3 4" />
        </g>
      </svg>
      <div className="art-coordinate bottom">
        <span>
          <i /> Admission layer active
        </span>
        <span>Rust / in-memory</span>
      </div>
    </div>
  );
}
