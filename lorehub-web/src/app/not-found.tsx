import Link from "next/link";

/**
 * A fish drawn from a grid of dots. The dots are computed once, at module
 * load, from fixed shapes, so the markup is identical on every render and
 * the picture reads fine as a still image.
 */
const STEP = 9;
const WIDTH = 220;
const HEIGHT = 110;

function insideFish(x: number, y: number): boolean {
  const cy = HEIGHT / 2;
  const body = ((x - 95) / 72) ** 2 + ((y - cy) / 36) ** 2 <= 1;
  const tailSpread = (x - 160) * 0.85;
  const tail = x >= 160 && x <= 210 && Math.abs(y - cy) <= tailSpread;
  const eye = (x - 58) ** 2 + (y - (cy - 8)) ** 2 <= 36;
  return (body || tail) && !eye;
}

const DOTS: { x: number; y: number }[] = [];
for (let y = STEP; y < HEIGHT; y += STEP) {
  for (let x = STEP; x < WIDTH; x += STEP) {
    if (insideFish(x, y)) DOTS.push({ x, y });
  }
}

export default function NotFound() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-6 px-6 text-center">
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        role="img"
        aria-label="A fish made of dots"
        className="lore-swim w-56 text-text-secondary"
      >
        {DOTS.map((dot) => (
          <circle
            key={`${dot.x}-${dot.y}`}
            cx={dot.x}
            cy={dot.y}
            r={2.2}
            fill="currentColor"
          />
        ))}
      </svg>
      <div className="flex flex-col gap-1">
        <h1 className="text-2xl font-bold text-text-primary">
          This page doesn&apos;t exist
        </h1>
        <p className="max-w-sm text-sm text-text-secondary">
          The repository or page may have been deleted or moved, or you may not
          have access to it.
        </p>
      </div>
      <Link
        href="/"
        className="rounded-pill bg-accent px-5 py-2 text-sm font-bold text-black transition-opacity hover:opacity-90"
      >
        Back to repositories
      </Link>
    </main>
  );
}
