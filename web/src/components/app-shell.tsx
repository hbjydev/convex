import { Link, Outlet, useLocation } from "@tanstack/react-router";
import { LogOutIcon } from "lucide-react";
import { useState } from "react";
import { api } from "@/api/client";
import { useAuth } from "@/auth";
import { Notice } from "@/components/feedback";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { errorMessage, unwrap } from "@/lib/requests";
import { cn } from "@/lib/utils";

export function AppShell() {
  const { account, reload } = useAuth();
  const location = useLocation();
  const [error, setError] = useState("");

  return (
    <div className="shell">
      <header>
        <Link className="brand" to="/">
          Convex
          <span className="brand-dot" />
        </Link>
        <nav aria-label="Main navigation">
          <Button variant="ghost" asChild>
            <Link
              to="/system"
              aria-current={
                location.pathname === "/system" ? "page" : undefined
              }
            >
              System status
            </Link>
          </Button>
          {account && (
            <>
              <Button variant="ghost" asChild className="min-w-0 max-w-full">
                <Link to="/account">
                  <span className="truncate">{account.user.display_name}</span>
                </Link>
              </Button>
              <Button
                variant="outline"
                type="button"
                onClick={async () => {
                  try {
                    unwrap(await api.POST("/api/v1/auth/logout"));
                    await reload();
                  } catch (error) {
                    setError(errorMessage(error));
                  }
                }}
              >
                <LogOutIcon data-icon="inline-start" />
                Sign out
              </Button>
            </>
          )}
        </nav>
      </header>
      <main
        className={cn(
          !account && location.pathname !== "/system" && "auth-page",
        )}
      >
        {error && <Notice error>{error}</Notice>}
        <Outlet />
      </main>
      <Separator />
      <footer>Convex · Self-hosted issue tracking</footer>
    </div>
  );
}
