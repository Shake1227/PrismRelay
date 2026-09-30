export function PrismMark({ className = "" }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 64 64"
      fill="none"
      aria-hidden="true"
    >
      <defs>
        <linearGradient
          id="prism-gradient"
          x1="8"
          y1="10"
          x2="59"
          y2="54"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#9a8fff" />
          <stop offset="0.5" stopColor="#c4bfff" />
          <stop offset="1" stopColor="#58ddcf" />
        </linearGradient>
      </defs>
      <path
        d="M32 5 57 19v27L32 60 7 46V19L32 5Z"
        fill="url(#prism-gradient)"
        fillOpacity=".08"
        stroke="url(#prism-gradient)"
        strokeWidth="1.5"
      />
      <path
        d="m32 14 18 32H14l18-32Z"
        fill="url(#prism-gradient)"
        fillOpacity=".2"
        stroke="url(#prism-gradient)"
        strokeWidth="1.5"
      />
      <path
        d="m32 14 4.5 22L50 46H14l18-32Z"
        stroke="url(#prism-gradient)"
        strokeWidth="1.5"
      />
      <path
        d="m14 46 22.5-10L32 60M36.5 36 57 19M32 14V5"
        stroke="url(#prism-gradient)"
        strokeOpacity=".5"
      />
    </svg>
  );
}
