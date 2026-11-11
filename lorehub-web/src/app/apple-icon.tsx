import { ImageResponse } from "next/og";
import { MarkSvg } from "@/lib/mark";

// iOS のホーム画面用。角は OS が丸めるので、全面を塗り、マークは少し内側へ置く。
export const size = { width: 180, height: 180 };
export const contentType = "image/png";

export default function AppleIcon() {
  return new ImageResponse(<MarkSvg size={180} rx={0} inset={0.1} />, size);
}
