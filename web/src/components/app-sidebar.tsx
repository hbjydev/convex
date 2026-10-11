import { Link, useLocation } from '@tanstack/react-router';
import { ActivityIcon } from 'lucide-react';
import { useAuth } from '@/auth';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar';
import { WorkspaceMenu } from '@/components/workspace-menu';
import { SettingsSidebar } from '@/components/settings-sidebar';
import { TeamNavigation } from '@/components/team-navigation';
import { useCurrentWorkspace } from '@/features/workspaces/use-current-workspace';

export function AppSidebar({
  onError,
}: {
  onError: (message: string) => void;
}) {
  const { account } = useAuth();
  const { pathname } = useLocation();
  const { setOpenMobile } = useSidebar();
  const workspace = useCurrentWorkspace();
  const closeNavigation = () => setOpenMobile(false);
  const workspaceOpen = pathname === `/${workspace?.slug}`;

  if (workspace && workspaceOpen)
    return (
      <Sidebar variant="inset">
        <SettingsSidebar
          workspace={workspace}
          closeNavigation={closeNavigation}
          onError={onError}
        />
        <SidebarRail />
      </Sidebar>
    );

  return (
    <Sidebar variant="inset">
      <SidebarHeader className="flex-row items-center gap-1 px-2 py-3">
        <SidebarMenu>
          <SidebarMenuItem>
            <WorkspaceMenu
              workspace={workspace}
              closeNavigation={closeNavigation}
              onError={onError}
            />
          </SidebarMenuItem>
        </SidebarMenu>
        <SidebarTrigger className="md:hidden" aria-label="Close navigation" />
      </SidebarHeader>
      <SidebarContent>
        {workspace && (
          <TeamNavigation
            key={workspace.id}
            slug={workspace.slug}
            closeNavigation={closeNavigation}
          />
        )}
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <SidebarMenuButton asChild isActive={pathname === '/system'}>
              <Link
                to="/system"
                aria-current={pathname === '/system' ? 'page' : undefined}
                onClick={closeNavigation}
              >
                <ActivityIcon />
                <span>System status</span>
              </Link>
            </SidebarMenuButton>
          </SidebarMenuItem>
          {account && (
            <SidebarMenuItem>
              <SidebarMenuButton asChild isActive={pathname === '/account'}>
                <Link
                  to="/account"
                  onClick={closeNavigation}
                  aria-label="Your account"
                  aria-current={pathname === '/account' ? 'page' : undefined}
                >
                  <Avatar size="sm">
                    <AvatarFallback>
                      {account.user.display_name.slice(0, 1).toUpperCase()}
                    </AvatarFallback>
                  </Avatar>
                  <span>{account.user.display_name}</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          )}
        </SidebarMenu>
        <p className="px-2 pt-1 pb-2 text-[11px] text-muted-foreground">
          Self-hosted issue tracking
        </p>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
