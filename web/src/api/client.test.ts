import { describe, expect, it } from "vitest";
import { createApiClient } from "./client";

describe("shared API client", () => {
  it("uses the documented route and same-origin credentials", async () => {
    const client = createApiClient(async (input) => {
      const request = new Request(input);
      expect(new URL(request.url).pathname).toBe("/api/v1/health");
      expect(request.credentials).toBe("same-origin");
      return Response.json({ status: "ready", version: "0.1.0" });
    });
    const { data } = await client.GET("/api/v1/health", {
      baseUrl: "http://localhost",
    });
    expect(data).toEqual({ status: "ready", version: "0.1.0" });
  });

  it("preserves the documented error response", async () => {
    const client = createApiClient(async () =>
      Response.json(
        { code: "service_unavailable", message: "Database unavailable" },
        { status: 503 },
      ),
    );
    const { data, error, response } = await client.GET("/api/v1/health", {
      baseUrl: "http://localhost",
    });
    expect(data).toBeUndefined();
    expect(response.status).toBe(503);
    expect(error?.code).toBe("service_unavailable");
  });
});
