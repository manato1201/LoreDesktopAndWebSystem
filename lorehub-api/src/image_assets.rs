//! Illustrative SVG stand-ins for the mock binary image assets referenced
//! in the seeded repository tree. There is no real texture/skybox raster
//! data in this mock backend — these are hand-authored placeholders so the
//! preview and diff-review screens have something real to render instead
//! of an icon.

pub const HERO_DIFFUSE_AFTER: &str = r##"<svg viewBox="0 0 400 400" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="400" fill="#d99a6c"/>
  <rect x="0" y="0" width="200" height="160" fill="#c98a5c"/>
  <rect x="200" y="0" width="200" height="160" fill="#3a5a7a"/>
  <rect x="0" y="160" width="140" height="240" fill="#6b4423"/>
  <rect x="140" y="160" width="130" height="120" fill="#8a8f94"/>
  <rect x="140" y="280" width="130" height="120" fill="#5a5f64"/>
  <rect x="270" y="160" width="130" height="240" fill="#2b1f14"/>
  <g stroke="#00000055" stroke-width="2">
    <line x1="0" y1="160" x2="400" y2="160"/>
    <line x1="200" y1="0" x2="200" y2="160"/>
    <line x1="140" y1="160" x2="140" y2="400"/>
    <line x1="270" y1="160" x2="270" y2="400"/>
    <line x1="140" y1="280" x2="270" y2="280"/>
  </g>
  <g fill="#ffffff30">
    <circle cx="34" cy="38" r="3"/>
    <circle cx="78" cy="112" r="2.4"/>
    <circle cx="150" cy="46" r="3"/>
    <circle cx="240" cy="90" r="2.6"/>
    <circle cx="320" cy="40" r="3"/>
    <circle cx="40" cy="210" r="2.6"/>
    <circle cx="95" cy="330" r="3"/>
    <circle cx="185" cy="220" r="2.4"/>
    <circle cx="200" cy="350" r="3"/>
    <circle cx="310" cy="260" r="2.6"/>
    <circle cx="340" cy="360" r="3"/>
  </g>
  <g fill="#00000025">
    <circle cx="60" cy="70" r="2.2"/>
    <circle cx="260" cy="130" r="2"/>
    <circle cx="120" cy="300" r="2.4"/>
    <circle cx="300" cy="320" r="2"/>
  </g>
</svg>
"##;

pub const HERO_DIFFUSE_BEFORE: &str = r##"<svg viewBox="0 0 400 400" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="400" fill="#b8977c"/>
  <rect x="0" y="0" width="400" height="180" fill="#5a6a78"/>
  <rect x="0" y="180" width="400" height="220" fill="#5a4632"/>
  <rect width="400" height="400" fill="#00000022"/>
</svg>
"##;

pub const SKYBOX_DUSK_AFTER: &str = r##"<svg viewBox="0 0 400 225" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="sky" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#241b45"/>
      <stop offset="45%" stop-color="#7a3b5e"/>
      <stop offset="75%" stop-color="#f0714a"/>
      <stop offset="100%" stop-color="#ffcf7d"/>
    </linearGradient>
    <radialGradient id="sun" cx="50%" cy="50%" r="50%">
      <stop offset="0%" stop-color="#fff3cf"/>
      <stop offset="100%" stop-color="#ffcf7d00"/>
    </radialGradient>
  </defs>
  <rect width="400" height="225" fill="url(#sky)"/>
  <circle cx="200" cy="165" r="70" fill="url(#sun)"/>
  <circle cx="200" cy="165" r="26" fill="#fff7e0"/>
  <g fill="#ffffff">
    <circle cx="40" cy="20" r="1.2"/>
    <circle cx="90" cy="35" r="1"/>
    <circle cx="140" cy="15" r="1.1"/>
    <circle cx="260" cy="25" r="1"/>
    <circle cx="310" cy="40" r="1.2"/>
    <circle cx="360" cy="18" r="1"/>
    <circle cx="20" cy="55" r="1"/>
    <circle cx="380" cy="60" r="1.1"/>
  </g>
  <path d="M0,225 L0,190 Q60,170 120,190 T240,185 T400,195 L400,225 Z" fill="#241522"/>
</svg>
"##;
