import createClient from "openapi-fetch";
import type { paths } from "./schema";

export function createApiClient(fetcher: typeof fetch = globalThis.fetch) {
  return createClient<paths>({ fetch: fetcher, credentials: "same-origin" });
}

export const api = createApiClient();
