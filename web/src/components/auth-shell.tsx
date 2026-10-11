import { Link, Outlet } from '@tanstack/react-router';
import { LayersIcon } from 'lucide-react';

export const AuthShell = () => (
  <div className="flex min-h-svh flex-col bg-sidebar p-5 md:px-8 md:py-6 [&_a:focus-visible]:outline-2 [&_a:focus-visible]:outline-ring [&_a:focus-visible]:outline-offset-3">
    <header className="flex items-center justify-between gap-4">
      <Link
        className="inline-flex shrink-0 items-center gap-2 text-base font-semibold tracking-[-0.03em] no-underline focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-3"
        to="/"
      >
        <span
          className="flex size-6 items-center justify-center rounded-lg bg-primary text-primary-foreground [&_svg]:size-4"
          aria-hidden="true"
        >
          <LayersIcon />
        </span>
        Convex
      </Link>
      <Link className="text-[13px] text-muted-foreground" to="/system">
        System status
      </Link>
    </header>
    <main className="flex flex-1 items-center justify-center py-12">
      <Outlet />
    </main>
    <footer className="text-center text-xs text-muted-foreground">
      Convex · Self-hosted issue tracking
    </footer>
  </div>
);
