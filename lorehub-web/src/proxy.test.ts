import { describe, expect, it } from "vitest";
import { cookieNameValue, getSetCookies, mergeCookieHeader } from "./proxy";

describe("cookieNameValue", () => {
  it("extracts the name/value pair, ignoring trailing attributes", () => {
    expect(
      cookieNameValue(
        "lorehub_token=abc123; Path=/; HttpOnly; SameSite=Lax; Max-Age=1800",
      ),
    ).toEqual(["lorehub_token", "abc123"]);
  });

  it("handles a cookie with no trailing attributes", () => {
    expect(cookieNameValue("foo=bar")).toEqual(["foo", "bar"]);
  });

  it("handles an empty value", () => {
    expect(cookieNameValue("foo=; Path=/")).toEqual(["foo", ""]);
  });

  it("returns null when there is no '=' at all", () => {
    expect(cookieNameValue("garbage")).toBeNull();
  });
});

describe("mergeCookieHeader", () => {
  it("keeps existing cookies not mentioned in the fresh set untouched", () => {
    const merged = mergeCookieHeader("a=1; b=2", ["c=3; Path=/"]);
    const jar = Object.fromEntries(merged.split("; ").map((p) => p.split("=")));
    expect(jar).toEqual({ a: "1", b: "2", c: "3" });
  });

  it("overrides same-named cookies with the fresh value", () => {
    const merged = mergeCookieHeader(
      "lorehub_token=old; lorehub_refresh=old-r",
      [
        "lorehub_token=new; Path=/; HttpOnly",
        "lorehub_refresh=new-r; Path=/; HttpOnly",
      ],
    );
    expect(merged).toContain("lorehub_token=new");
    expect(merged).toContain("lorehub_refresh=new-r");
    expect(merged).not.toMatch(/=old(;|$)/);
    expect(merged).not.toContain("old-r");
  });

  it("works starting from a null existing header", () => {
    const merged = mergeCookieHeader(null, ["a=1; Path=/"]);
    expect(merged).toBe("a=1");
  });

  it("ignores blank segments in the existing header", () => {
    const merged = mergeCookieHeader("a=1; ; b=2", []);
    const jar = Object.fromEntries(merged.split("; ").map((p) => p.split("=")));
    expect(jar).toEqual({ a: "1", b: "2" });
  });
});

describe("getSetCookies", () => {
  it("returns each Set-Cookie header value separately via Headers.getSetCookie()", () => {
    const headers = new Headers();
    headers.append("set-cookie", "a=1; Path=/");
    headers.append("set-cookie", "b=2; Path=/");
    expect(getSetCookies(headers)).toEqual(["a=1; Path=/", "b=2; Path=/"]);
  });

  it("returns an empty array when there is no set-cookie header", () => {
    expect(getSetCookies(new Headers())).toEqual([]);
  });
});
