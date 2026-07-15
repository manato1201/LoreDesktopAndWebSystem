import "server-only";
import { cookies } from "next/headers";

const SESSION_COOKIE = "lorehub_token";

/**
 * Server Components run in Node, which has no ambient browser cookie jar —
 * unlike client-side `fetch(..., { credentials: "include" })`, a
 * server-side call to lorehub-api needs the session cookie forwarded
 * explicitly. Only import this from Server Components/layouts; the
 * `server-only` import makes accidentally pulling it into a Client
 * Component a build error instead of a silent bug.
 */
export async function getSessionCookieHeader(): Promise<string | undefined> {
  const token = (await cookies()).get(SESSION_COOKIE)?.value;
  return token ? `${SESSION_COOKIE}=${token}` : undefined;
}
