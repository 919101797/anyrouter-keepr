export function LiquidGlassBackdrop() {
  return (
    <div className="lg-refraction-layer" aria-hidden="true">
      <svg viewBox="0 0 1460 830" preserveAspectRatio="none">
        <defs>
          <linearGradient id="lg-wall" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0" stopColor="#fbfaf1" />
            <stop offset="0.44" stopColor="#edf9f1" />
            <stop offset="1" stopColor="#f9f2ff" />
          </linearGradient>
          <radialGradient id="lg-lime" cx="15%" cy="18%" r="34%">
            <stop offset="0" stopColor="#ccff32" stopOpacity="0.9" />
            <stop offset="1" stopColor="#ccff32" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="lg-blue" cx="82%" cy="14%" r="34%">
            <stop offset="0" stopColor="#7497ff" stopOpacity="0.78" />
            <stop offset="1" stopColor="#7497ff" stopOpacity="0" />
          </radialGradient>
          <radialGradient id="lg-cyan" cx="76%" cy="82%" r="36%">
            <stop offset="0" stopColor="#37ddd6" stopOpacity="0.68" />
            <stop offset="1" stopColor="#37ddd6" stopOpacity="0" />
          </radialGradient>
          <filter id="lg-refraction" x="-20%" y="-20%" width="140%" height="140%">
            <feTurbulence
              type="fractalNoise"
              baseFrequency="0.008 0.016"
              numOctaves="2"
              seed="8"
              result="noise"
            />
            <feDisplacementMap
              in="SourceGraphic"
              in2="noise"
              scale="22"
              xChannelSelector="R"
              yChannelSelector="G"
              result="displaced"
            />
            <feGaussianBlur in="displaced" stdDeviation="0.7" result="blurred" />
            <feColorMatrix
              in="blurred"
              type="matrix"
              values="1.08 0 0 0 0 0 1.1 0 0 0 0 0 1.12 0 0 0 0 0 0.96 0"
            />
          </filter>
        </defs>
        <rect width="1460" height="830" fill="url(#lg-wall)" />
        <rect width="1460" height="830" fill="url(#lg-lime)" />
        <rect width="1460" height="830" fill="url(#lg-blue)" />
        <rect width="1460" height="830" fill="url(#lg-cyan)" />
        <g filter="url(#lg-refraction)" opacity="0.74">
          <path
            d="M-150 184 C130 52 340 172 560 126 C820 70 940 180 1160 132 C1290 104 1390 42 1580 76"
            fill="none"
            stroke="#ccff32"
            strokeWidth="44"
            strokeLinecap="round"
            opacity="0.42"
          />
          <path
            d="M60 648 C198 450 330 690 470 450 C650 146 776 596 980 286 C1120 72 1286 212 1440 104"
            fill="none"
            stroke="#151917"
            strokeWidth="2"
            strokeLinecap="round"
            opacity="0.16"
          />
          <path
            d="M88 566 C240 416 350 510 492 356 C666 168 812 454 1012 280 C1166 146 1276 284 1420 180"
            fill="none"
            stroke="#1fc877"
            strokeWidth="5"
            strokeLinecap="round"
            opacity="0.38"
          />
        </g>
      </svg>
    </div>
  );
}
