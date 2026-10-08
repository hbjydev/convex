import { Link, useLocation } from '@tanstack/react-router';
import {
  ActivityIcon,
  LayersIcon,
  SettingsIcon,
  UsersIcon,
} from 'lucide-react';
import { useAuth } from '@/auth';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarRail,
  SidebarTrigger,
  useSidebar,
} from '@/components/ui/sidebar';
import { WorkspaceMenu } from '@/components/workspace-menu';
import { useCurrentWorkspace } from '@/features/workspaces/use-current-workspace';

export function AppSidebar({
  onError,
}: {
  onError: (message: string) => void;
}) {
  const { account } = useAuth();
  const { pathname, hash } = useLocation();
  const { setOpenMobile } = useSidebar();
  const workspace = useCurrentWorkspace();
  const closeNavigation = () => setOpenMobile(false);
  const workspaceOpen = pathname === `/${workspace?.slug}`;

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
        <nav aria-label="Main navigation">
          <SidebarGroup>
            <SidebarGroupContent>
              <SidebarMenu>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild isActive={pathname === '/'}>
                    <Link
                      to="/"
                      aria-current={pathname === '/' ? 'page' : undefined}
                      onClick={closeNavigation}
                    >
                      <LayersIcon />
                      <span>All workspaces</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </nav>
        {workspace && (
          <nav aria-label="Workspace navigation">
            <SidebarGroup className="mt-4">
              <SidebarGroupLabel>Workspace</SidebarGroupLabel>
              <SidebarGroupContent>
                <SidebarMenu>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      asChild
                      isActive={workspaceOpen && !hash}
                    >
                      <Link
                        to="/$workspaceSlug"
                        params={{ workspaceSlug: workspace.slug }}
                        onClick={closeNavigation}
                        aria-current={
                          workspaceOpen && !hash ? 'page' : undefined
                        }
                      >
                        <SettingsIcon />
                        <span>Settings</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      asChild
                      isActive={workspaceOpen && hash === 'people'}
                    >
                      <Link
                        to="/$workspaceSlug"
                        params={{ workspaceSlug: workspace.slug }}
                        hash="people"
                        onClick={closeNavigation}
                        aria-current={
                          workspaceOpen && hash === 'people'
                            ? 'page'
                            : undefined
                        }
                      >
                        <UsersIcon />
                        <span>People</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                  <SidebarMenuItem>
                    <SidebarMenuButton
                      asChild
                      isActive={workspaceOpen && hash === 'teams'}
                    >
                      <Link
                        to="/$workspaceSlug"
                        params={{ workspaceSlug: workspace.slug }}
                        hash="teams"
                        onClick={closeNavigation}
                        aria-current={
                          workspaceOpen && hash === 'teams' ? 'page' : undefined
                        }
                      >
                        <LayersIcon />
                        <span>Teams</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                </SidebarMenu>
              </SidebarGroupContent>
            </SidebarGroup>
          </nav>
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
        <p className="sidebar-caption">Self-hosted issue tracking</p>
      </SidebarFooter>
      <SidebarRail />
    </Sidebar>
  );
}
