// サイトのマーク(「L」+ コミットの点(緑))。icon.svg と同じ図形を、ImageResponse(apple-icon・PWA・OGP)でも使う。
// 図形を変えるときは icon.svg も同じにする(DESIGN.md「アイコン」)。

export const MARK_COLORS = { tile: "#121212", ink: "#ffffff", accent: "#1ed760" } as const;

/**
 * 32x32 の格子で描いたマーク。`rx` はタイルの角丸。
 * `inset` は、全面を塗るアイコン(iOS・maskable)で、マークを内側へ縮める割合(0〜0.3)。
 */
export function MarkSvg({
  size,
  rx = 7,
  inset = 0,
  colors = MARK_COLORS,
}: {
  size: number;
  rx?: number;
  inset?: number;
  colors?: { tile: string; ink: string; accent: string };
}) {
  const s = 1 - inset * 2;
  const t = 16 * (1 - s);
  return (
    <svg width={size} height={size} viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
      <rect width="32" height="32" rx={rx} fill={colors.tile} />
      <g transform={`translate(${t} ${t}) scale(${s})`}>
        <path d="M11.5 7.5v16.5h10" fill="none" stroke={colors.ink} strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round" />
        <circle cx="22" cy="10.5" r="2.6" fill={colors.accent} />
      </g>
    </svg>
  );
}
