import { Link, useLocation } from '@tanstack/react-router';
import {
  Building2Icon,
  ChevronLeftIcon,
  LayersIcon,
  UserRoundIcon,
  UsersIcon,
} from 'lucide-react';
import { useState } from 'react';
import type { components } from '@/api/schema';
import {
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarTrigger,
} from '@/components/ui/sidebar';
import { WorkspaceMenu } from '@/components/workspace-menu';
import { SettingsSearch } from '@/features/workspaces/settings-shared';

export function SettingsSidebar({
  workspace,
  closeNavigation,
  onError,
}: {
  workspace: components['schemas']['Workspace'];
  closeNavigation: () => void;
  onError: (message: string) => void;
}) {
  const { hash } = useLocation();
  const [search, setSearch] = useState('');
  const destinations = [
    {
      label: 'Workspace',
      hash: '',
      icon: Building2Icon,
      active: !['teams', 'people', 'members', 'create-team'].includes(hash),
    },
    {
      label: 'Teams',
      hash: 'teams',
      icon: LayersIcon,
      active: hash === 'teams' || hash === 'create-team',
    },
    {
      label: 'Members',
      hash: 'people',
      icon: UsersIcon,
      active: hash === 'people' || hash === 'members',
    },
  ].filter((item) => item.label.toLowerCase().includes(search.toLowerCase()));
  return (
    <>
      <SidebarHeader className="px-2 py-3">
        <div className="flex items-center gap-1">
          <SidebarMenu>
            <SidebarMenuItem>
              <SidebarMenuButton asChild>
                <Link to="/" onClick={closeNavigation}>
                  <ChevronLeftIcon />
                  <span>Back to app</span>
                </Link>
              </SidebarMenuButton>
            </SidebarMenuItem>
          </SidebarMenu>
          <SidebarTrigger className="md:hidden" aria-label="Close navigation" />
        </div>
        <SettingsSearch
          label="Search settings…"
          value={search}
          onChange={setSearch}
        />
      </SidebarHeader>
      <SidebarContent>
        <nav aria-label="Settings navigation">
          <SidebarGroup>
            <SidebarGroupLabel>Administration</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                {destinations.map((item) => (
                  <SidebarMenuItem key={item.label}>
                    <SidebarMenuButton asChild isActive={item.active}>
                      <Link
                        to="/$workspaceSlug"
                        params={{ workspaceSlug: workspace.slug }}
                        hash={item.hash}
                        onClick={closeNavigation}
                        aria-current={item.active ? 'page' : undefined}
                      >
                        <item.icon />
                        <span>{item.label}</span>
                      </Link>
                    </SidebarMenuButton>
                  </SidebarMenuItem>
                ))}
              </SidebarMenu>
              {destinations.length === 0 && (
                <p className="px-2 py-3 text-xs text-muted-foreground">
                  No matching settings.
                </p>
              )}
            </SidebarGroupContent>
          </SidebarGroup>
          <SidebarGroup className="mt-4">
            <SidebarGroupLabel>Personal</SidebarGroupLabel>
            <SidebarGroupContent>
              <SidebarMenu>
                <SidebarMenuItem>
                  <SidebarMenuButton asChild>
                    <Link
                      to="/account"
                      aria-label="Your account"
                      onClick={closeNavigation}
                    >
                      <UserRoundIcon />
                      <span>Your account</span>
                    </Link>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              </SidebarMenu>
            </SidebarGroupContent>
          </SidebarGroup>
        </nav>
      </SidebarContent>
      <SidebarFooter>
        <SidebarMenu>
          <SidebarMenuItem>
            <WorkspaceMenu
              workspace={workspace}
              closeNavigation={closeNavigation}
              onError={onError}
            />
          </SidebarMenuItem>
        </SidebarMenu>
        <p className="px-2 pt-1 pb-2 text-[11px] text-muted-foreground">
          Workspace settings
        </p>
      </SidebarFooter>
    </>
  );
}
