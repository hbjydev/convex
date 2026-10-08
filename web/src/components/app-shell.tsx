import { Outlet } from '@tanstack/react-router';
import { type ReactNode, useState } from 'react';
import { AppSidebar } from '@/components/app-sidebar';
import { Notice } from '@/components/feedback';
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar';
import { TooltipProvider } from '@/components/ui/tooltip';

export function AppShell({ children }: { children?: ReactNode }) {
  return (
    <TooltipProvider>
      <SidebarProvider className="app-shell">
        <ShellContent>{children}</ShellContent>
      </SidebarProvider>
    </TooltipProvider>
  );
}

function ShellContent({ children }: { children?: ReactNode }) {
  const { openMobile } = useSidebar();
  const [error, setError] = useState('');
  return (
    <>
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <AppSidebar onError={setError} />
      <SidebarInset id="main-content" tabIndex={-1}>
        <div className="app-content">
          <SidebarTrigger
            className="mb-3 md:hidden"
            aria-label="Toggle navigation"
            aria-expanded={openMobile}
          />
          {error && <Notice error>{error}</Notice>}
          {children ?? <Outlet />}
        </div>
      </SidebarInset>
    </>
  );
}
