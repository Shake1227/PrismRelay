import { useEffect, useId, useRef } from "react";
import "./CrystalCompletion.css";

const duration = 4200;
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
const clamp = (value: number) => Math.min(1, Math.max(0, value));
function valueAt(time: number, points: [number, number][]): number {
  for (let index = 1; index < points.length; index++) {
    const [end, to] = points[index];
    if (time <= end) {
      const [start, from] = points[index - 1];
      const progress = clamp((time - start) / (end - start));
      const eased = progress * progress * (3 - 2 * progress);
      return from + (to - from) * eased;
    }
  }
  return points[points.length - 1][1];
}
function transform(x = 0, y = 0, rotation = 0, scale = 1): string {
  return `translate(${x} ${y}) translate(32 32) rotate(${rotation}) scale(${scale}) translate(-32 -32)`;
}
function CrystalGeometry({ gradient }: { gradient: string }) {
  const paint = `url(#${gradient})`;
  return (
    <>
      <path
        d="M32 5 57 19v27L32 60 7 46V19L32 5Z"
        fill={paint}
        fillOpacity=".08"
        stroke={paint}
        strokeWidth="1.5"
      />
      <path
        d="m32 14 18 32H14l18-32Z"
        fill={paint}
        fillOpacity=".2"
        stroke={paint}
        strokeWidth="1.5"
      />
      <path
        d="m32 14 4.5 22L50 46H14l18-32Z"
        stroke={paint}
        strokeWidth="1.5"
      />
      <path
        d="m14 46 22.5-10L32 60M36.5 36 57 19M32 14V5"
        stroke={paint}
        strokeOpacity=".5"
      />
    </>
  );
}

export function CrystalCompletion() {
  const root = useRef<HTMLDivElement>(null);
  const id = useId().replace(/:/g, "");
  const gradient = `completion-gradient-${id}`;
  const plasma = `completion-plasma-${id}`;
  const glow = `completion-glow-${id}`;
  const crystal = `completion-crystal-${id}`;
  const url = (name: string) => `url(#${name})`;

  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const query = window.matchMedia("(prefers-reduced-motion: reduce)");
    const find = (name: string) =>
      element.querySelector<SVGElement>(`.completion-${name}`)!;
    const core = find("crystal");
    const field = find("field");
    const ring = find("ring");
    const orbits = find("orbits");
    const afterglow = find("afterglow");
    const rest = find("rest");
    const lightPaths = element.querySelectorAll<SVGElement>(
      ".completion-filament",
    );
    const shards = element.querySelectorAll<SVGElement>(".completion-shard");
    const sparks = element.querySelectorAll<SVGElement>(".completion-particle");
    let frame = 0;
    let started = false;
    let start: number | undefined;
    const opacity = (node: SVGElement, value: number) =>
      node.setAttribute("opacity", String(value));
    const reset = () => {
      window.cancelAnimationFrame(frame);
      element.dataset.motion = "reduced";
      opacity(core, 1);
      core.removeAttribute("transform");
      opacity(afterglow, 0.18);
    };
    const animate = (timestamp: number) => {
      start ??= timestamp;
      const elapsed = timestamp - start;
      const time = clamp(elapsed / duration);
      const progress = clamp((time - 0.08) / 0.5);
      const wave = Math.PI * 2 * (2 * progress + 7 * progress * progress);
      const amplitude =
        time < 0.08 || time >= 0.61 ? 0 : 0.2 + 3 * progress * progress;
      opacity(
        core,
        valueAt(time, [
          [0, 1],
          [0.61, 1],
          [0.64, 0],
          [1, 0],
        ]),
      );
      core.setAttribute(
        "transform",
        transform(
          Math.sin(wave) * amplitude,
          Math.sin(wave * 1.3) * amplitude * 0.25,
          Math.sin(wave) * amplitude * 1.4,
          valueAt(time, [
            [0, 1],
            [0.57, 1],
            [0.61, 1.12],
            [0.64, 0.94],
            [1, 0.94],
          ]),
        ),
      );
      opacity(
        field,
        valueAt(time, [
          [0, 0],
          [0.18, 0],
          [0.38, 0.2],
          [0.55, 0.7],
          [0.63, 0.95],
          [0.78, 0.45],
          [1, 0],
        ]),
      );
      field.setAttribute(
        "transform",
        transform(
          0,
          0,
          0,
          valueAt(time, [
            [0, 0.4],
            [0.18, 0.4],
            [0.38, 0.75],
            [0.55, 1.1],
            [0.63, 1.8],
            [0.78, 2.3],
            [1, 2.7],
          ]),
        ),
      );
      opacity(
        orbits,
        valueAt(time, [
          [0, 0],
          [0.38, 0],
          [0.52, 0.2],
          [0.63, 0.55],
          [0.76, 0],
          [1, 0],
        ]),
      );
      opacity(
        ring,
        valueAt(time, [
          [0, 0],
          [0.56, 0],
          [0.64, 0.85],
          [0.82, 0.15],
          [1, 0],
        ]),
      );
      ring.setAttribute(
        "transform",
        transform(
          0,
          0,
          0,
          valueAt(time, [
            [0, 0.55],
            [0.56, 0.55],
            [0.64, 1.15],
            [0.82, 2.7],
            [1, 3.1],
          ]),
        ),
      );
      lightPaths.forEach((node, index) => {
        const local = clamp((elapsed - (index % 3) * 35) / duration);
        opacity(
          node,
          valueAt(local, [
            [0, 0],
            [0.43, 0],
            [0.58, 0.35],
            [0.65, 0.95],
            [0.77, 0.35],
            [0.94, 0],
            [1, 0],
          ]),
        );
        node.setAttribute(
          "stroke-dashoffset",
          String(
            valueAt(local, [
              [0, 150],
              [0.43, 150],
              [0.58, 115],
              [0.65, 38],
              [0.77, 0],
              [0.94, -70],
              [1, -70],
            ]),
          ),
        );
      });
      shards.forEach((node, index) => {
        const local = clamp((elapsed - index * 12) / duration);
        const travel = valueAt(local, [
          [0, 0],
          [0.63, 0],
          [0.78, 1],
          [1, 1.6],
        ]);
        const facet = facets[index];
        opacity(
          node,
          valueAt(local, [
            [0, 0],
            [0.6, 0],
            [0.63, 1],
            [0.78, 0.7],
            [1, 0],
          ]),
        );
        node.setAttribute(
          "transform",
          transform(
            facet.x * travel,
            facet.y * travel,
            facet.rotation * Math.min(1, travel),
            valueAt(local, [
              [0, 1],
              [0.6, 1],
              [0.63, 1.08],
              [0.78, 0.78],
              [1, 0.2],
            ]),
          ),
        );
      });
      sparks.forEach((node, index) => {
        const particle = particles[index];
        const local = clamp((elapsed - particle.delay) / duration);
        const travel = valueAt(local, [
          [0, 0],
          [0.58, 0],
          [0.64, 0.18],
          [0.84, 1],
          [1, 1.25],
        ]);
        opacity(
          node,
          valueAt(local, [
            [0, 0],
            [0.58, 0],
            [0.64, 1],
            [0.84, 0.6],
            [1, 0],
          ]),
        );
        node.setAttribute(
          "transform",
          transform(
            particle.x * travel,
            particle.y * travel,
            0,
            valueAt(local, [
              [0, 0.3],
              [0.58, 0.3],
              [0.64, 1.3],
              [0.84, 0.8],
              [1, 0.1],
            ]),
          ),
        );
      });
      opacity(
        afterglow,
        valueAt(time, [
          [0, 0],
          [0.7, 0],
          [0.87, 0.2],
          [1, 0.16],
        ]),
      );
      opacity(
        rest,
        valueAt(time, [
          [0, 0],
          [0.8, 0],
          [1, 0.55],
        ]),
      );
      rest.setAttribute(
        "transform",
        transform(
          0,
          0,
          0,
          valueAt(time, [
            [0, 0.4],
            [0.8, 0.4],
            [1, 0.8],
          ]),
        ),
      );
      if (elapsed < duration + 100)
        frame = window.requestAnimationFrame(animate);
      else element.dataset.motion = "finished";
    };
    const updateMotion = () => {
      if (
        query.matches ||
        document.documentElement.dataset.reduceMotion === "true"
      )
        reset();
      else if (!started) {
        started = true;
        element.dataset.motion = "running";
        frame = window.requestAnimationFrame(animate);
      }
    };
    updateMotion();
    query.addEventListener?.("change", updateMotion);
    const observer = new window.MutationObserver(updateMotion);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-reduce-motion"],
    });
    return () => {
      window.cancelAnimationFrame(frame);
      query.removeEventListener?.("change", updateMotion);
      observer.disconnect();
    };
  }, []);

  return (
    <div
      ref={root}
      className="crystal-completion"
      aria-hidden="true"
      data-motion="static"
    >
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
          opacity="0"
        />
        <g
          className="completion-motion completion-field"
          filter={url(glow)}
          opacity="0"
        >
          <circle cx="32" cy="32" r="30" fill={url(plasma)} />
        </g>
        <g
          className="completion-motion completion-orbits"
          stroke={url(gradient)}
          strokeWidth=".7"
          opacity="0"
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
          opacity="0"
        />
        <g className="completion-motion" filter={url(glow)}>
          {filaments.map((path, index) => (
            <path
              className="completion-filament"
              key={path}
              d={path}
              stroke={index % 3 === 0 ? "#eefcff" : url(gradient)}
              strokeWidth={index % 3 === 0 ? ".55" : ".85"}
              strokeDasharray="150"
              strokeDashoffset="150"
              opacity="0"
            />
          ))}
        </g>
        <g className="completion-crystal" filter={url(glow)}>
          <CrystalGeometry gradient={gradient} />
        </g>
        <g className="completion-motion" filter={url(glow)}>
          {facets.map((facet, index) => (
            <g className="completion-shard" key={facet.path} opacity="0">
              <g clipPath={url(`${crystal}-facet-${index}`)}>
                <CrystalGeometry gradient={gradient} />
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
              opacity="0"
            />
          ))}
        </g>
        <path
          className="completion-motion completion-rest"
          d="m32 24 2 6 6 2-6 2-2 6-2-6-6-2 6-2Z"
          fill={url(gradient)}
          filter={url(glow)}
          opacity="0"
        />
      </svg>
    </div>
  );
}
