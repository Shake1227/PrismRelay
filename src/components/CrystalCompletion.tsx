import { useEffect, useId, useRef } from "react";
import "./CrystalCompletion.css";

const duration = 6200;
const surfaces = [
  { path: "M32 5 7 19 32 14Z", x: -24, y: -32, rotation: -38 },
  { path: "M32 5 32 14 57 19Z", x: 24, y: -30, rotation: 34 },
  { path: "M7 19 7 46 14 46Z", x: -38, y: 2, rotation: -42 },
  { path: "M7 19 32 14 14 46Z", x: -28, y: -10, rotation: -24 },
  { path: "M32 14 36.5 36 14 46Z", x: -14, y: 12, rotation: -32 },
  { path: "M32 14 57 19 36.5 36Z", x: 21, y: -12, rotation: 28 },
  { path: "M57 19 57 46 36.5 36Z", x: 39, y: 8, rotation: 42 },
  { path: "M57 46 32 60 36.5 36Z", x: 22, y: 32, rotation: 38 },
  { path: "M14 46 36.5 36 32 60Z", x: -5, y: 36, rotation: -28 },
  { path: "M7 46 14 46 32 60Z", x: -30, y: 24, rotation: -46 },
];
const facets = surfaces.flatMap((surface, index) => {
  if (index >= 6) return [surface];
  const points = surface.path.match(/-?\d+(?:\.\d+)?/g)!.map(Number);
  const midpoint = [(points[2] + points[4]) / 2, (points[3] + points[5]) / 2];
  return [
    {
      ...surface,
      path: `M${points[0]} ${points[1]} ${points[2]} ${points[3]} ${midpoint[0]} ${midpoint[1]}Z`,
      x: surface.x * 1.1 - 4,
      y: surface.y * 1.1 - 3,
      rotation: surface.rotation - 19,
    },
    {
      ...surface,
      path: `M${points[0]} ${points[1]} ${midpoint[0]} ${midpoint[1]} ${points[4]} ${points[5]}Z`,
      x: surface.x * 1.18 + 5,
      y: surface.y * 1.18 + 4,
      rotation: surface.rotation + 23,
    },
  ];
});

const filaments = [
  "M-27-15C-6-21-2 9 12 13S24 25 32 32",
  "M94-10C73-18 66 7 50 15S37 25 32 32",
  "M102 17C87 34 59 15 48 25S38 29 32 32",
  "M-37 54C-23 34-2 56 13 45S25 36 32 32",
  "M93 82C66 86 73 48 54 45S38 35 32 32",
  "M-12 90C14 81-1 56 17 48S28 37 32 32",
  "M33-31C53-13 24 1 33 14S33 27 32 32",
  "M106 48C89 63 64 36 53 39S39 34 32 32",
];
const particles = Array.from({ length: 48 }, (_, index) => ({
  angle: index * 2.399963229728653,
  distance: 54 + ((index * 17) % 31),
  radius: 0.65 + (index % 5) * 0.28,
  delay: (index % 7) * 0.011,
}));
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

export type CompletionMotion = "reduced" | "running" | "finished";

export function CrystalCompletion({
  onMotionChange,
}: {
  onMotionChange?: (state: CompletionMotion) => void;
}) {
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
    const charge = find("charge");
    const seams = find("seams");
    const afterglow = find("afterglow");
    const lightPaths = element.querySelectorAll<SVGElement>(
      ".completion-filament",
    );
    const shards = element.querySelectorAll<SVGElement>(".completion-shard");
    const sparks = element.querySelectorAll<SVGElement>(".completion-particle");
    const trails = element.querySelectorAll<SVGElement>(
      ".completion-particle-trail",
    );
    let frame = 0;
    let started = false;
    let elapsed = 0;
    let lastTimestamp: number | undefined;
    let motion: CompletionMotion = "reduced";
    const setMotion = (value: CompletionMotion) => {
      motion = value;
      element.dataset.motion = value;
      onMotionChange?.(value);
    };
    const resetGeometry = () => {
      for (const node of [
        field,
        charge,
        seams,
        afterglow,
        ...lightPaths,
        ...shards,
        ...sparks,
        ...trails,
      ]) {
        node.setAttribute("opacity", "0");
        node.removeAttribute("transform");
      }
      core.setAttribute("opacity", "1");
      core.removeAttribute("transform");
    };
    const opacity = (node: SVGElement, value: number) =>
      node.setAttribute("opacity", String(value));
    const reset = () => {
      window.cancelAnimationFrame(frame);
      frame = 0;
      started = false;
      elapsed = 0;
      lastTimestamp = undefined;
      resetGeometry();
      setMotion("reduced");
      opacity(afterglow, 0.18);
    };
    const animate = (timestamp: number) => {
      frame = 0;
      if (document.visibilityState === "hidden") {
        lastTimestamp = undefined;
        return;
      }
      if (lastTimestamp !== undefined)
        elapsed += Math.max(0, timestamp - lastTimestamp);
      lastTimestamp = timestamp;
      const time = clamp(elapsed / duration);
      if (time >= 1) {
        for (const node of [
          core,
          field,
          charge,
          seams,
          afterglow,
          ...lightPaths,
          ...shards,
          ...sparks,
          ...trails,
        ])
          opacity(node, 0);
        setMotion("finished");
        return;
      }
      const progress = clamp((time - 0.08) / 0.62);
      const wave = Math.PI * 2 * (1.6 * progress + 10 * progress * progress);
      const amplitude =
        time < 0.08 || time >= 0.71 ? 0 : 0.1 + 4.2 * progress * progress;
      opacity(
        core,
        valueAt(time, [
          [0, 1],
          [0.7, 1],
          [0.73, 0],
          [1, 0],
        ]),
      );
      core.setAttribute(
        "transform",
        transform(
          Math.sin(wave) * amplitude,
          Math.sin(wave * 1.3) * amplitude * 0.3,
          Math.sin(wave) * amplitude * 1.2,
          valueAt(time, [
            [0, 1],
            [0.62, 1],
            [0.7, 1.06],
            [0.73, 0.96],
            [1, 0.96],
          ]),
        ),
      );
      opacity(
        charge,
        valueAt(time, [
          [0, 0],
          [0.12, 0.03],
          [0.38, 0.36],
          [0.58, 0.8],
          [0.7, 0.98],
          [0.73, 0],
          [1, 0],
        ]),
      );
      opacity(
        seams,
        valueAt(time, [
          [0, 0],
          [0.25, 0],
          [0.48, 0.35],
          [0.66, 0.85],
          [0.71, 1],
          [0.74, 0],
          [1, 0],
        ]),
      );
      opacity(
        field,
        valueAt(time, [
          [0, 0],
          [0.3, 0],
          [0.58, 0.18],
          [0.69, 0.95],
          [0.74, 0.75],
          [0.85, 0],
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
            [0, 0.7],
            [0.58, 0.7],
            [0.69, 1.2],
            [0.74, 1.9],
            [0.85, 2.1],
            [1, 2.1],
          ]),
        ),
      );
      lightPaths.forEach((node, index) => {
        const delay = (index % 3) * 0.012;
        const local = clamp((time - delay) / (1 - delay));
        opacity(
          node,
          valueAt(local, [
            [0, 0],
            [0.12, 0.06],
            [0.3, 0.2],
            [0.52, 0.35],
            [0.66, 0],
            [1, 0],
          ]),
        );
        node.setAttribute("stroke-dashoffset", String(-elapsed / (32 - index)));
      });
      shards.forEach((node, index) => {
        const delay = (index % 4) * 0.006;
        const local = clamp((time - delay) / (1 - delay));
        const travel = valueAt(local, [
          [0, 0],
          [0.7, 0],
          [0.84, 1],
          [1, 1.45],
        ]);
        const facet = facets[index];
        opacity(
          node,
          valueAt(local, [
            [0, 0],
            [0.69, 0],
            [0.72, 1],
            [0.84, 0.85],
            [0.95, 0.2],
            [1, 0],
          ]),
        );
        node.setAttribute(
          "transform",
          transform(
            facet.x * travel * 1.2,
            facet.y * travel * 1.2 + 12 * travel * travel,
            facet.rotation * travel,
            valueAt(local, [
              [0, 1],
              [0.7, 1],
              [0.84, 0.8],
              [1, 0.4],
            ]),
          ),
        );
      });
      const positionAt = (
        particle: (typeof particles)[number],
        clock: number,
      ) => {
        if (clock >= 0.7) {
          const travel = clamp((clock - 0.7) / 0.3);
          const distance =
            particle.distance *
            (0.12 + 1.12 * (1 - (1 - travel) * (1 - travel)));
          const angle = particle.angle + 1.5 + travel * 0.35;
          return {
            x: Math.cos(angle) * distance,
            y: Math.sin(angle) * distance + 9 * travel * travel,
          };
        }
        const travel = clamp((clock - particle.delay) / 0.62);
        const distance = particle.distance * Math.pow(1 - travel, 1.35);
        const angle =
          particle.angle + 1.5 * travel + 0.16 * Math.sin(travel * Math.PI);
        return { x: Math.cos(angle) * distance, y: Math.sin(angle) * distance };
      };
      sparks.forEach((node, index) => {
        const particle = particles[index];
        const local = clamp((time - particle.delay) / (1 - particle.delay));
        const light = valueAt(local, [
          [0, 0],
          [0.08, 0.25],
          [0.28, 0.7],
          [0.52, 1],
          [0.64, 0],
          [0.7, 0],
          [0.73, 1],
          [0.88, 0.6],
          [1, 0],
        ]);
        const position = positionAt(particle, time);
        const previous = positionAt(
          particle,
          Math.max(time >= 0.7 ? 0.7 : 0, time - 0.03),
        );
        opacity(node, light);
        node.setAttribute(
          "transform",
          transform(
            position.x,
            position.y,
            0,
            valueAt(local, [
              [0, 0.7],
              [0.52, 1.05],
              [0.64, 0.4],
              [0.7, 0.4],
              [0.73, 1.3],
              [1, 0.2],
            ]),
          ),
        );
        const trail = trails[index];
        opacity(trail, light * 0.65);
        trail.setAttribute(
          "d",
          `M${32 + previous.x} ${32 + previous.y}L${32 + position.x} ${32 + position.y}`,
        );
      });
      opacity(
        afterglow,
        valueAt(time, [
          [0, 0],
          [0.67, 0],
          [0.72, 0.35],
          [0.88, 0.12],
          [1, 0],
        ]),
      );
      frame = window.requestAnimationFrame(animate);
    };
    const updateMotion = () => {
      if (
        query.matches ||
        document.documentElement.dataset.reduceMotion === "true"
      )
        reset();
      else if (!started) {
        started = true;
        resetGeometry();
        setMotion("running");
        if (document.visibilityState !== "hidden")
          frame = window.requestAnimationFrame(animate);
      }
    };
    const updateVisibility = () => {
      if (motion !== "running") return;
      window.cancelAnimationFrame(frame);
      frame = 0;
      lastTimestamp = undefined;
      if (document.visibilityState !== "hidden")
        frame = window.requestAnimationFrame(animate);
    };
    updateMotion();
    document.addEventListener("visibilitychange", updateVisibility);
    query.addEventListener?.("change", updateMotion);
    const observer = new window.MutationObserver(updateMotion);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-reduce-motion"],
    });
    return () => {
      window.cancelAnimationFrame(frame);
      query.removeEventListener?.("change", updateMotion);
      document.removeEventListener("visibilitychange", updateVisibility);
      observer.disconnect();
    };
  }, [onMotionChange]);

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
            <stop stopColor="#b68dff" />
            <stop offset=".5" stopColor="#ffffff" />
            <stop offset="1" stopColor="#63ffee" />
          </linearGradient>
          <radialGradient id={plasma}>
            <stop stopColor="#fff" stopOpacity=".95" />
            <stop offset=".22" stopColor="#efffff" stopOpacity="1" />
            <stop offset=".5" stopColor="#cc8aff" stopOpacity=".75" />
            <stop offset=".75" stopColor="#64ffec" stopOpacity=".32" />
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
            <feGaussianBlur stdDeviation="2.7" />
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
          <path d="M32 5 57 19v27L32 60 7 46V19Z" fill={url(plasma)} />
        </g>
        <g className="completion-motion" filter={url(glow)}>
          {filaments.map((path, index) => (
            <path
              className="completion-filament"
              key={path}
              d={path}
              stroke={index % 3 === 0 ? "#eefcff" : url(gradient)}
              strokeWidth={index % 3 === 0 ? ".55" : ".85"}
              strokeDasharray="1 12"
              strokeDashoffset="0"
              opacity="0"
            />
          ))}
        </g>
        <g className="completion-crystal" filter={url(glow)}>
          <CrystalGeometry gradient={gradient} />
          <path
            className="completion-motion completion-charge"
            d="M32 5 57 19v27L32 60 7 46V19Z"
            fill={url(plasma)}
            opacity="0"
          />
          <path
            className="completion-motion completion-seams"
            d="m32 5 4.5 31L7 46m29.5-10L57 19M32 14l18 32H14Zm4.5 22L32 60"
            stroke="#f1fbff"
            strokeWidth=".75"
            opacity="0"
          />
        </g>
        <g className="completion-motion" filter={url(glow)}>
          {facets.map((facet, index) => (
            <g className="completion-shard" key={facet.path} opacity="0">
              <path
                d={facet.path}
                fill={url(gradient)}
                fillOpacity=".76"
                stroke={url(gradient)}
                strokeWidth=".7"
              />
              <g clipPath={url(`${crystal}-facet-${index}`)}>
                <CrystalGeometry gradient={gradient} />
              </g>
            </g>
          ))}
          {particles.map((_, index) => (
            <path
              className="completion-particle-trail"
              key={`trail-${index}`}
              stroke={
                index % 3 === 0
                  ? "#f5ffff"
                  : index % 3 === 1
                    ? "#bc9fff"
                    : "#7bf4df"
              }
              strokeWidth=".85"
              strokeLinecap="round"
              opacity="0"
            />
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
      </svg>
    </div>
  );
}
