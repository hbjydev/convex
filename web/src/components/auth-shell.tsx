import { Link, Outlet } from "@tanstack/react-router";
import { LayersIcon } from "lucide-react";

export const AuthShell = () => (
  <div className="auth-shell">
    <header>
      <Link className="brand" to="/">
        <span className="brand-mark" aria-hidden="true">
          <LayersIcon />
        </span>
        Convex
      </Link>
      <Link to="/system">System status</Link>
    </header>
    <main>
      <Outlet />
    </main>
    <footer>Convex · Self-hosted issue tracking</footer>
  </div>
);
