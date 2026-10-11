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
      <SidebarProvider className="wrap-anywhere">
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
      <a
        className="fixed top-2 left-2 z-50 -translate-y-[150%] rounded-lg border bg-background px-3 py-2 focus:translate-y-0 focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-3"
        href="#main-content"
      >
        Skip to content
      </a>
      <AppSidebar onError={setError} />
      <SidebarInset id="main-content" tabIndex={-1}>
        <div className="mx-auto w-full min-w-0 max-w-[1120px] px-5 pt-7 pb-12 md:px-12 md:pt-10 md:pb-16 has-[[data-layout=settings]]:relative has-[[data-layout=settings]]:max-w-none md:has-[[data-layout=directory]]:px-14 has-[[data-layout=directory]]:relative has-[[data-layout=directory]]:max-w-none has-[[data-layout=issues]]:max-w-none has-[[data-layout=issues]]:p-0 max-md:has-[[data-layout=issues]]:pt-4 max-md:has-[[data-layout=issues]]:[&>[data-slot=sidebar-trigger]]:ml-5 has-[[data-layout=issue-detail]]:max-w-none has-[[data-layout=issue-detail]]:p-0">
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
