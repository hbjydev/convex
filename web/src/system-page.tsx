import { useEffect, useState } from "react";
import { api } from "./api/client";
import type { components } from "./api/schema";

type Connection =
  | { state: "loading" }
  | { state: "ready"; health: components["schemas"]["HealthResponse"] }
  | { state: "error"; message: string };

export function SystemPage() {
  const [connection, setConnection] = useState<Connection>({
    state: "loading",
  });
  const [attempt, setAttempt] = useState(0);

  // biome-ignore lint/correctness/useExhaustiveDependencies: The attempt counter explicitly triggers a fresh request on retry.
  useEffect(() => {
    const controller = new AbortController();
    setConnection({ state: "loading" });
    api
      .GET("/api/v1/health", { signal: controller.signal })
      .then(({ data, error }) => {
        if (controller.signal.aborted) return;
        setConnection(
          data
            ? { state: "ready", health: data }
            : {
                state: "error",
                message:
                  error?.message ??
                  "The server returned an unexpected response.",
              },
        );
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setConnection({
            state: "error",
            message:
              "Could not reach the server. Check that the API is running, then try again.",
          });
      });
    return () => controller.abort();
  }, [attempt]);

  return (
    <>
      <h1>A place for your team's work.</h1>
      <p className="intro">
        Check the connection between Convex, its API, and PostgreSQL.
      </p>
      <section aria-labelledby="connection-heading" className="connection">
        <div className="section-heading">
          <h2 id="connection-heading">System connection</h2>
          <button
            type="button"
            disabled={connection.state === "loading"}
            onClick={() => setAttempt(attempt + 1)}
          >
            {connection.state === "loading" ? "Checking…" : "Check again"}
          </button>
        </div>
        <div
          role="status"
          aria-live="polite"
          className={`status status-${connection.state}`}
        >
          <span className="status-dot" aria-hidden="true" />
          <div>
            <strong>
              {connection.state === "loading"
                ? "Connecting to Convex"
                : connection.state === "ready"
                  ? "Ready to build"
                  : "Connection unavailable"}
            </strong>
            <p>
              {connection.state === "loading"
                ? "Checking the API and PostgreSQL connection…"
                : connection.state === "ready"
                  ? "The API is responding and PostgreSQL is ready."
                  : connection.message}
            </p>
          </div>
        </div>
        <dl>
          <div>
            <dt>Application version</dt>
            <dd>
              {connection.state === "ready" ? connection.health.version : "—"}
            </dd>
          </div>
          <div>
            <dt>API contract</dt>
            <dd>
              <a href="/api/openapi.json">Open specification</a>
            </dd>
          </div>
        </dl>
      </section>
    </>
  );
}
