import { afterEach, describe, expect, it, vi } from "vitest";
import { fetchWithRefresh } from "./api";

function jsonResponse(status: number, body: unknown = {}): Response {
  return new Response(JSON.stringify(body), {
    status,
    headers: { "Content-Type": "application/json" },
  });
}

/**
 * `fetchWithRefresh` is the CSR half of the transparent access-token
 * refresh story (see src/lib/api.ts's doc comment and src/proxy.ts for the
 * SSR half). `doFetch` stands in for the caller's actual request (e.g.
 * `apiGet`'s `() => fetch(...)`); the module-level `fetch` global is
 * stubbed separately to represent lorehub-api's own `/api/auth/refresh`
 * call that `refreshAccessToken()` makes internally — the two are
 * deliberately decoupled here exactly like they are in production.
 */
describe("fetchWithRefresh", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("returns the response unchanged when the first call already succeeds", async () => {
    const doFetch = vi.fn().mockResolvedValue(jsonResponse(200, { ok: true }));
    const refreshFetch = vi.fn();
    vi.stubGlobal("fetch", refreshFetch);

    const res = await fetchWithRefresh(doFetch);

    expect(res.status).toBe(200);
    expect(doFetch).toHaveBeenCalledTimes(1);
    expect(refreshFetch).not.toHaveBeenCalled();
  });

  it("retries once after a silent refresh on 401, and the caller only ever sees the final success", async () => {
    const refreshFetch = vi.fn().mockResolvedValue(jsonResponse(200));
    vi.stubGlobal("fetch", refreshFetch);

    const doFetch = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401))
      .mockResolvedValueOnce(jsonResponse(200, { data: "after-refresh" }));

    const res = await fetchWithRefresh(doFetch);

    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ data: "after-refresh" });
    expect(doFetch).toHaveBeenCalledTimes(2);
    expect(refreshFetch).toHaveBeenCalledTimes(1);
    const [refreshUrl] = refreshFetch.mock.calls[0] as [string];
    expect(refreshUrl).toContain("/api/auth/refresh");
  });

  it("does not loop forever when the retried request also 401s", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(jsonResponse(200)));
    const doFetch = vi.fn().mockResolvedValue(jsonResponse(401));

    const res = await fetchWithRefresh(doFetch);

    expect(res.status).toBe(401);
    // Original attempt + exactly one retry — never more.
    expect(doFetch).toHaveBeenCalledTimes(2);
  });

  it("returns the original 401 unchanged, without attempting a refresh, when a cookie was supplied (SSR path)", async () => {
    const refreshFetch = vi.fn().mockResolvedValue(jsonResponse(200));
    vi.stubGlobal("fetch", refreshFetch);
    const doFetch = vi.fn().mockResolvedValue(jsonResponse(401));

    const res = await fetchWithRefresh(doFetch, "lorehub_token=abc");

    expect(res.status).toBe(401);
    expect(doFetch).toHaveBeenCalledTimes(1);
    expect(refreshFetch).not.toHaveBeenCalled();
  });

  it("does not retry when the refresh call itself fails", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockRejectedValue(new Error("network down")),
    );
    const doFetch = vi.fn().mockResolvedValue(jsonResponse(401));

    const res = await fetchWithRefresh(doFetch);

    expect(res.status).toBe(401);
    expect(doFetch).toHaveBeenCalledTimes(1);
  });

  it("shares a single in-flight refresh across concurrent 401s", async () => {
    let refreshCalls = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn().mockImplementation(async () => {
        refreshCalls += 1;
        await new Promise((resolve) => setTimeout(resolve, 5));
        return jsonResponse(200);
      }),
    );

    const doFetchA = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401))
      .mockResolvedValueOnce(jsonResponse(200));
    const doFetchB = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse(401))
      .mockResolvedValueOnce(jsonResponse(200));

    const [resA, resB] = await Promise.all([
      fetchWithRefresh(doFetchA),
      fetchWithRefresh(doFetchB),
    ]);

    expect(resA.status).toBe(200);
    expect(resB.status).toBe(200);
    expect(refreshCalls).toBe(1);
  });
});
