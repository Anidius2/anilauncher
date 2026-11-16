import { useId } from "react";

/** Hand-drawn vector decorations for seasonal events. */

export function Pumpkin({ size = 120, face = 0 }: { size?: number; face?: 0 | 1 }) {
  const id = useId().replace(/:/g, "");
  const lobes = [
    { cx: 30, rx: 26, ry: 36 },
    { cx: 90, rx: 26, ry: 36 },
    { cx: 44, rx: 26, ry: 39 },
    { cx: 76, rx: 26, ry: 39 },
    { cx: 60, rx: 24, ry: 41 },
  ];
  return (
    <svg className="sa-pumpkin" width={size} height={size * 0.95} viewBox="0 0 120 114" aria-hidden="true">
      <defs>
        <radialGradient id={`pb${id}`} cx="45%" cy="38%" r="70%">
          <stop offset="0" stopColor="#ffc46b" />
          <stop offset=".45" stopColor="#f97316" />
          <stop offset=".85" stopColor="#c2410c" />
          <stop offset="1" stopColor="#7c2d12" />
        </radialGradient>
        <linearGradient id={`ps${id}`} x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#84cc16" />
          <stop offset="1" stopColor="#3f6212" />
        </linearGradient>
        <radialGradient id={`pf${id}`} cx="50%" cy="55%" r="60%">
          <stop offset="0" stopColor="#fffbe0" />
          <stop offset=".4" stopColor="#fde047" />
          <stop offset="1" stopColor="#f97316" />
        </radialGradient>
        <filter id={`pg${id}`} x="-50%" y="-50%" width="200%" height="200%">
          <feGaussianBlur stdDeviation="2.4" result="b" />
          <feMerge>
            <feMergeNode in="b" />
            <feMergeNode in="SourceGraphic" />
          </feMerge>
        </filter>
      </defs>
      <ellipse cx="60" cy="108" rx="46" ry="5" fill="rgba(0,0,0,.35)" />
      {lobes.map((l, i) => (
        <ellipse key={i} cx={l.cx} cy="68" rx={l.rx} ry={l.ry} fill={`url(#pb${id})`} stroke="#7c2d12" strokeOpacity=".45" strokeWidth="1.2" />
      ))}
      <ellipse cx="46" cy="46" rx="9" ry="14" fill="#fff" opacity=".16" transform="rotate(-18 46 46)" />
      <path d="M57 32 C55 22 56 14 61 8 L66 10 C63 16 63 23 64 32 Z" fill={`url(#ps${id})`} stroke="#365314" strokeWidth="1" />
      <path d="M64 18 C74 10 86 14 90 22 C80 22 72 24 64 22 Z" fill="#65a30d" stroke="#365314" strokeWidth=".8" />
      <path d="M58 26 C46 22 44 12 52 10 C58 9 58 16 53 16" fill="none" stroke="#4d7c0f" strokeWidth="1.6" strokeLinecap="round" />
      <g className="sa-pumpkin__face" filter={`url(#pg${id})`} fill={`url(#pf${id})`}>
        {face === 0 ? (
          <>
            <path d="M34 60 L46 50 L50 64 Z" />
            <path d="M86 60 L74 50 L70 64 Z" />
            <path d="M56 70 L60 63 L64 70 Z" />
            <path d="M30 76 Q60 104 90 76 L82 79 L78 86 L72 81 L66 89 L60 82 L54 89 L48 81 L42 86 L38 79 Z" />
          </>
        ) : (
          <>
            <ellipse cx="44" cy="58" rx="7" ry="9" />
            <ellipse cx="76" cy="58" rx="7" ry="9" />
            <path d="M34 76 Q60 96 86 76 Q60 86 34 76 Z" />
          </>
        )}
      </g>
    </svg>
  );
}

export function Cake({ age }: { age: number | null }) {
  const id = useId().replace(/:/g, "");
  const digits = age && age > 0 ? String(age).split("") : null;
  const sprinkles = ["#f43f5e", "#facc15", "#22c55e", "#38bdf8", "#a78bfa", "#fb923c"];
  return (
    <svg className="sa-cake" width="104" height="114" viewBox="0 0 140 154" aria-hidden="true">
      <defs>
        <linearGradient id={`ct${id}`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#f472b6" />
          <stop offset=".45" stopColor="#f9a8d4" />
          <stop offset="1" stopColor="#db2777" />
        </linearGradient>
        <linearGradient id={`cb${id}`} x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="#7c3aed" />
          <stop offset=".45" stopColor="#a78bfa" />
          <stop offset="1" stopColor="#5b21b6" />
        </linearGradient>
        <linearGradient id={`cd${id}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#38bdf8" />
          <stop offset="1" stopColor="#1d4ed8" />
        </linearGradient>
        <radialGradient id={`cf${id}`} cx="50%" cy="70%" r="60%">
          <stop offset="0" stopColor="#fff" />
          <stop offset=".35" stopColor="#fde047" />
          <stop offset="1" stopColor="#f97316" stopOpacity=".9" />
        </radialGradient>
        <radialGradient id={`ch${id}`}>
          <stop offset="0" stopColor="#fde68a" stopOpacity=".7" />
          <stop offset="1" stopColor="#fde68a" stopOpacity="0" />
        </radialGradient>
      </defs>
      <ellipse cx="70" cy="146" rx="62" ry="7" fill="#e5e7eb" />
      <ellipse cx="70" cy="144" rx="56" ry="5" fill="#f9fafb" />
      {/* bottom tier */}
      <rect x="16" y="96" width="108" height="46" rx="8" fill={`url(#cb${id})`} />
      <path d="M16 104 Q16 96 24 96 H116 Q124 96 124 104 V108 Q118 116 114 108 Q110 122 104 108 Q98 114 92 108 Q86 124 80 108 Q74 114 66 108 Q60 120 54 108 Q48 114 42 108 Q36 122 30 108 Q24 114 16 110 Z" fill="#fdf2f8" />
      {/* top tier */}
      <rect x="34" y="60" width="72" height="40" rx="7" fill={`url(#ct${id})`} />
      <path d="M34 67 Q34 60 41 60 H99 Q106 60 106 67 V70 Q101 78 97 70 Q93 82 88 70 Q83 76 78 70 Q73 84 68 70 Q63 76 58 70 Q53 80 48 70 Q43 76 34 72 Z" fill="#fff" />
      {Array.from({ length: 16 }, (_, i) => (
        <rect key={i} x={22 + ((i * 37) % 96)} y={i % 2 ? 120 + (i % 3) * 6 : 82 + (i % 3) * 5} width="5" height="2" rx="1"
          fill={sprinkles[i % sprinkles.length]} transform={`rotate(${(i * 47) % 180} ${24 + ((i * 37) % 96)} ${i % 2 ? 121 + (i % 3) * 6 : 83 + (i % 3) * 5})`}
          opacity={i % 2 || (22 + ((i * 37) % 96) > 36 && 22 + ((i * 37) % 96) < 100) ? 1 : 0} />
      ))}
      {/* candles */}
      {(digits ?? ["", "", ""]).map((d, i, all) => {
        const step = digits ? 30 : 20;
        const cx = 70 + (i - (all.length - 1) / 2) * step;
        return (
          <g key={i}>
            <circle cx={cx} cy={digits ? 14 : 26} r="14" fill={`url(#ch${id})`} className="sa-cake__halo" />
            {digits ? (
              <>
                <line x1={cx} y1="28" x2={cx} y2="22" stroke="#374151" strokeWidth="1.5" />
                <text x={cx} y="60" textAnchor="middle" fontFamily="Inter, Arial, sans-serif" fontWeight="900" fontSize="38"
                  fill={`url(#cd${id})`} stroke="#fff" strokeWidth="1.5" paintOrder="stroke">{d}</text>
              </>
            ) : (
              <>
                <rect x={cx - 3} y="38" width="6" height="24" rx="2" fill="#fef3c7" />
                <path d={`M${cx - 3} 44 l6 -4 M${cx - 3} 52 l6 -4 M${cx - 3} 60 l6 -4`} stroke="#f43f5e" strokeWidth="2" />
                <line x1={cx} y1="38" x2={cx} y2="33" stroke="#374151" strokeWidth="1.5" />
              </>
            )}
            <path className="sa-flame" style={{ animationDelay: `${i * 0.23}s` }}
              d={digits ? `M${cx} 6 C${cx + 6} 14 ${cx + 5} 22 ${cx} 23 C${cx - 5} 22 ${cx - 6} 14 ${cx} 6 Z` : `M${cx} 18 C${cx + 5} 25 ${cx + 4} 32 ${cx} 33 C${cx - 4} 32 ${cx - 5} 25 ${cx} 18 Z`}
              fill={`url(#cf${id})`} />
          </g>
        );
      })}
    </svg>
  );
}

export function Balloon({ color, size = 54 }: { color: string; size?: number }) {
  const id = useId().replace(/:/g, "");
  return (
    <svg width={size} height={size * 2.2} viewBox="0 0 50 110" aria-hidden="true">
      <defs>
        <radialGradient id={`b${id}`} cx="35%" cy="30%" r="75%">
          <stop offset="0" stopColor="#fff" stopOpacity=".85" />
          <stop offset=".18" stopColor={color} />
          <stop offset="1" stopColor={color} stopOpacity=".75" />
        </radialGradient>
      </defs>
      <path d="M25 58 C22 70 30 78 24 88 C18 98 28 104 25 110" fill="none" stroke="rgba(255,255,255,.7)" strokeWidth="1" />
      <path d="M25 2 C40 2 48 16 48 28 C48 44 34 56 25 58 C16 56 2 44 2 28 C2 16 10 2 25 2 Z" fill={`url(#b${id})`} />
      <path d="M22 58 L28 58 L26 63 L24 63 Z" fill={color} />
      <ellipse cx="16" cy="18" rx="4" ry="8" fill="#fff" opacity=".5" transform="rotate(25 16 18)" />
    </svg>
  );
}

export function Spider() {
  return (
    <svg className="sa-spider" width="60" height="240" viewBox="0 0 40 160" aria-hidden="true">
      <line x1="20" y1="0" x2="20" y2="128" stroke="rgba(255,255,255,.55)" strokeWidth=".8" />
      <g stroke="#111" strokeWidth="1.8" fill="none" strokeLinecap="round">
        <path d="M14 134 Q4 126 2 132" /><path d="M14 138 Q2 136 1 142" /><path d="M14 142 Q4 146 3 154" /><path d="M15 145 Q8 152 9 159" />
        <path d="M26 134 Q36 126 38 132" /><path d="M26 138 Q38 136 39 142" /><path d="M26 142 Q36 146 37 154" /><path d="M25 145 Q32 152 31 159" />
      </g>
      <ellipse cx="20" cy="143" rx="8" ry="10" fill="#18181b" />
      <circle cx="20" cy="131" r="5.5" fill="#27272a" />
      <circle cx="18" cy="130" r="1.3" fill="#f97316" />
      <circle cx="22" cy="130" r="1.3" fill="#f97316" />
      <ellipse cx="17" cy="139" rx="2" ry="3.5" fill="#fff" opacity=".12" />
    </svg>
  );
}

/** One repeatable garland segment: a sagging wire with a glass bulb. */
export function GarlandSegment({ color }: { color: string }) {
  const id = useId().replace(/:/g, "");
  return (
    <svg className="sa-garland__seg" width="74" height="54" viewBox="0 0 74 54" aria-hidden="true">
      <defs>
        <radialGradient id={`g${id}`} cx="38%" cy="30%" r="80%">
          <stop offset="0" stopColor="#fff" />
          <stop offset=".25" stopColor={color} />
          <stop offset="1" stopColor={color} stopOpacity=".7" />
        </radialGradient>
        <linearGradient id={`m${id}`} x1="0" x2="1">
          <stop offset="0" stopColor="#9ca3af" />
          <stop offset=".5" stopColor="#f3f4f6" />
          <stop offset="1" stopColor="#6b7280" />
        </linearGradient>
      </defs>
      <path d="M0 4 Q37 26 74 4" fill="none" stroke="#14532d" strokeWidth="2.2" />
      <path d="M0 4 Q37 26 74 4" fill="none" stroke="#22c55e" strokeOpacity=".35" strokeWidth=".8" />
      <g className="sa-bulb" style={{ color }}>
        <rect x="33" y="14" width="8" height="7" rx="1.5" fill={`url(#m${id})`} />
        <path d="M37 20 C47 24 46 40 37 49 C28 40 27 24 37 20 Z" fill={`url(#g${id})`} />
        <ellipse cx="34" cy="30" rx="1.6" ry="5" fill="#fff" opacity=".6" />
      </g>
    </svg>
  );
}

/** A fir branch with baubles for the hero corner. */
export function FirBranch() {
  const needles = Array.from({ length: 34 }, (_, i) => {
    const t = i / 33;
    const x = 4 + t * 196, y = 10 + t * 26 + Math.sin(t * 3) * 4;
    const side = i % 2 ? 1 : -1;
    return `M${x} ${y} l${10 - t * 4} ${side * (14 - t * 6)}`;
  }).join(" ");
  return (
    <svg className="sa-fir" width="210" height="90" viewBox="0 0 210 90" aria-hidden="true">
      <path d="M0 8 Q100 22 205 40" stroke="#5b3a1a" strokeWidth="4" fill="none" strokeLinecap="round" />
      <path d={needles} stroke="#166534" strokeWidth="3" strokeLinecap="round" />
      <path d={needles} stroke="#22c55e" strokeOpacity=".5" strokeWidth="1" strokeLinecap="round" transform="translate(2 -1)" />
      {[
        { x: 70, y: 22, l: 24, c: "#ef4444" },
        { x: 140, y: 32, l: 34, c: "#facc15" },
        { x: 185, y: 40, l: 18, c: "#3b82f6" },
      ].map((b, i) => (
        <g key={i} className="sa-bauble" style={{ animationDelay: `${i * 0.6}s`, transformOrigin: `${b.x}px ${b.y}px` }}>
          <line x1={b.x} y1={b.y} x2={b.x} y2={b.y + b.l} stroke="#d1d5db" strokeWidth=".8" />
          <rect x={b.x - 3} y={b.y + b.l} width="6" height="4" rx="1" fill="#d1d5db" />
          <circle cx={b.x} cy={b.y + b.l + 13} r="10" fill={b.c} />
          <circle cx={b.x - 3.5} cy={b.y + b.l + 9} r="3" fill="#fff" opacity=".6" />
          <path d={`M${b.x - 10} ${b.y + b.l + 13} Q${b.x} ${b.y + b.l + 18} ${b.x + 10} ${b.y + b.l + 13}`} stroke="#fff" strokeOpacity=".5" strokeWidth="1.5" fill="none" />
        </g>
      ))}
    </svg>
  );
}
