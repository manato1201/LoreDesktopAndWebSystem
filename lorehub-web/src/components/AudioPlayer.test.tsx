import { describe, expect, it } from "vitest";
import { formatTime } from "./AudioPlayer";

describe("formatTime", () => {
  it("formats whole minutes and seconds with zero-padding", () => {
    expect(formatTime(65)).toBe("1:05");
  });

  it("formats durations under a minute", () => {
    expect(formatTime(9)).toBe("0:09");
  });

  it("floors fractional seconds rather than rounding", () => {
    expect(formatTime(125.9)).toBe("2:05");
  });

  it("handles zero", () => {
    expect(formatTime(0)).toBe("0:00");
  });

  it("returns 0:00 for non-finite input (NaN, Infinity)", () => {
    expect(formatTime(NaN)).toBe("0:00");
    expect(formatTime(Infinity)).toBe("0:00");
    expect(formatTime(-Infinity)).toBe("0:00");
  });

  it("does not pad the minutes component past one digit", () => {
    expect(formatTime(3661)).toBe("61:01");
  });
});
