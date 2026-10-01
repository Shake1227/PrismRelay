import { useId } from "react";
import type { CSSProperties } from "react";
import "./CrystalCompletion.css";

const facets = [
  { path: "M32 5 57 19 36.5 36 32 14Z", x: 22, y: -26, rotation: 32 },
  { path: "M7 19 32 5 32 14 14 46 7 46Z", x: -28, y: -18, rotation: -38 },
  { path: "M32 14 36.5 36 14 46Z", x: -17, y: 3, rotation: -24 },
  { path: "M36.5 36 50 46H14Z", x: 10, y: 26, rotation: 42 },
  { path: "M57 19V46L32 60 36.5 36Z", x: 30, y: 12, rotation: 36 },
  { path: "M7 46H14l22.5-10L32 60Z", x: -21, y: 25, rotation: -30 },
];
const filaments = [
  "M32 32C10 26 2 10-13 5S-8-10-27-15",
  "M32 32C54 25 62 5 77 8S78-7 94-10",
  "M32 32C48 35 59 18 75 24S82 13 102 17",
  "M32 32C13 42-4 29-15 42S-28 38-37 54",
  "M32 32C43 54 62 47 71 66S85 64 93 82",
  "M32 32C25 55 12 53 7 70S-4 72-12 90",
  "M32 32C22 14 40 4 32-12S39-20 33-31",
  "M32 32C50 44 67 31 84 44S89 37 106 48",
];
const particles = Array.from({ length: 22 }, (_, index) => {
  const angle = (index * Math.PI * 2) / 22;
  const distance = 33 + (index % 4) * 8;
  return {
    x: Math.cos(angle) * distance,
    y: Math.sin(angle) * distance,
    radius: 0.65 + (index % 3) * 0.35,
    delay: (index % 5) * 25,
  };
});

export function CrystalCompletion() {
  const id = useId().replace(/:/g, "");
  const gradient = `completion-gradient-${id}`;
  const plasma = `completion-plasma-${id}`;
  const glow = `completion-glow-${id}`;
  const crystal = `completion-crystal-${id}`;
  const url = (name: string) => `url(#${name})`;

  return (
    <div className="crystal-completion" aria-hidden="true">
      <svg viewBox="-50 -40 164 148" fill="none" focusable="false">
        <defs>
          <linearGradient
            id={gradient}
            x1="8"
            y1="10"
            x2="59"
            y2="54"
            gradientUnits="userSpaceOnUse"
          >
            <stop stopColor="#9a8fff" />
            <stop offset=".5" stopColor="#e9e5ff" />
            <stop offset="1" stopColor="#58ddcf" />
          </linearGradient>
          <radialGradient id={plasma}>
            <stop stopColor="#fff" stopOpacity=".95" />
            <stop offset=".22" stopColor="#e7deff" stopOpacity=".9" />
            <stop offset=".5" stopColor="#a087ff" stopOpacity=".48" />
            <stop offset=".75" stopColor="#64efdd" stopOpacity=".18" />
            <stop offset="1" stopColor="#8e79ff" stopOpacity="0" />
          </radialGradient>
          <filter
            id={glow}
            x="-100%"
            y="-100%"
            width="300%"
            height="300%"
            colorInterpolationFilters="sRGB"
          >
            <feGaussianBlur stdDeviation="2.1" />
            <feMerge>
              <feMergeNode />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
          <g id={crystal}>
            <path
              d="M32 5 57 19v27L32 60 7 46V19L32 5Z"
              fill={url(gradient)}
              fillOpacity=".08"
              stroke={url(gradient)}
              strokeWidth="1.5"
            />
            <path
              d="m32 14 18 32H14l18-32Z"
              fill={url(gradient)}
              fillOpacity=".2"
              stroke={url(gradient)}
              strokeWidth="1.5"
            />
            <path
              d="m32 14 4.5 22L50 46H14l18-32Z"
              stroke={url(gradient)}
              strokeWidth="1.5"
            />
            <path
              d="m14 46 22.5-10L32 60M36.5 36 57 19M32 14V5"
              stroke={url(gradient)}
              strokeOpacity=".5"
            />
          </g>
          {facets.map((facet, index) => (
            <clipPath id={`${crystal}-facet-${index}`} key={facet.path}>
              <path d={facet.path} />
            </clipPath>
          ))}
        </defs>
        <circle
          className="completion-afterglow"
          cx="32"
          cy="32"
          r="29"
          fill={url(plasma)}
        />
        <g className="completion-motion completion-field" filter={url(glow)}>
          <circle cx="32" cy="32" r="30" fill={url(plasma)} />
        </g>
        <g
          className="completion-motion completion-orbits"
          stroke={url(gradient)}
          strokeWidth=".7"
        >
          <ellipse
            cx="32"
            cy="32"
            rx="40"
            ry="14"
            transform="rotate(-24 32 32)"
          />
          <ellipse
            cx="32"
            cy="32"
            rx="35"
            ry="18"
            transform="rotate(34 32 32)"
          />
        </g>
        <circle
          className="completion-motion completion-ring"
          cx="32"
          cy="32"
          r="23"
          stroke={url(gradient)}
          strokeWidth=".8"
        />
        <g className="completion-motion" filter={url(glow)}>
          {filaments.map((path, index) => (
            <path
              className="completion-filament"
              key={path}
              d={path}
              stroke={index % 3 === 0 ? "#eefcff" : url(gradient)}
              strokeWidth={index % 3 === 0 ? ".55" : ".85"}
              style={{ animationDelay: `${(index % 3) * 35}ms` }}
            />
          ))}
        </g>
        <g className="completion-crystal" filter={url(glow)}>
          <use href={`#${crystal}`} />
        </g>
        <g className="completion-motion" filter={url(glow)}>
          {facets.map((facet, index) => (
            <g
              className="completion-shard"
              key={facet.path}
              style={
                {
                  "--shard-x": `${facet.x}px`,
                  "--shard-y": `${facet.y}px`,
                  "--shard-rotation": `${facet.rotation}deg`,
                  animationDelay: `${index * 12}ms`,
                } as CSSProperties
              }
            >
              <g clipPath={url(`${crystal}-facet-${index}`)}>
                <use href={`#${crystal}`} />
              </g>
            </g>
          ))}
          {particles.map((particle, index) => (
            <circle
              className="completion-particle"
              key={index}
              cx="32"
              cy="32"
              r={particle.radius}
              fill={
                index % 3 === 0
                  ? "#f5ffff"
                  : index % 3 === 1
                    ? "#bc9fff"
                    : "#7bf4df"
              }
              style={
                {
                  "--particle-x": `${particle.x}px`,
                  "--particle-y": `${particle.y}px`,
                  animationDelay: `${particle.delay}ms`,
                } as CSSProperties
              }
            />
          ))}
        </g>
        <path
          className="completion-motion completion-rest"
          d="m32 24 2 6 6 2-6 2-2 6-2-6-6-2 6-2Z"
          fill={url(gradient)}
          filter={url(glow)}
        />
      </svg>
    </div>
  );
}
