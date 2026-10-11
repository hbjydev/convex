import { Link, useLocation } from '@tanstack/react-router';
import {
  ChevronDownIcon,
  ChevronRightIcon,
  HomeIcon,
  ListTodoIcon,
  UsersIcon,
} from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
} from '@/components/ui/sidebar';
import type { WorkspaceDetail } from '@/features/issues/model';
import { useIssueWorkspace } from '@/features/issues/use-workspace';

function TeamMenu({
  team,
  slug,
  active,
  home,
  closeNavigation,
}: {
  team: WorkspaceDetail['teams'][number];
  slug: string;
  active: boolean;
  home: boolean;
  closeNavigation: () => void;
}) {
  const [open, setOpen] = useState(true);
  const menuId = `team-navigation-${team.id}`;
  return (
    <SidebarMenuItem>
      <SidebarMenuButton
        onClick={() => setOpen((value) => !value)}
        aria-expanded={open}
        aria-controls={menuId}
        title={team.name}
      >
        <UsersIcon />
        <span>{team.name}</span>
        {open ? (
          <ChevronDownIcon className="ml-auto" />
        ) : (
          <ChevronRightIcon className="ml-auto" />
        )}
      </SidebarMenuButton>
      <SidebarMenuSub
        id={menuId}
        hidden={!open}
        className="mr-0 ml-4 border-0 px-0 hidden:hidden"
      >
        <SidebarMenuSubItem>
          <SidebarMenuSubButton asChild isActive={active && home}>
            <Link
              to="/$workspaceSlug/team/$teamSlug"
              params={{ workspaceSlug: slug, teamSlug: team.key.toLowerCase() }}
              onClick={closeNavigation}
              aria-current={active && home ? 'page' : undefined}
            >
              <HomeIcon />
              <span>Home</span>
            </Link>
          </SidebarMenuSubButton>
        </SidebarMenuSubItem>
        <SidebarMenuSubItem>
          <SidebarMenuSubButton asChild isActive={active && !home}>
            <Link
              to="/$workspaceSlug/team/$teamSlug/all"
              params={{ workspaceSlug: slug, teamSlug: team.key.toLowerCase() }}
              onClick={closeNavigation}
              aria-current={active && !home ? 'page' : undefined}
            >
              <ListTodoIcon />
              <span>Issues</span>
            </Link>
          </SidebarMenuSubButton>
        </SidebarMenuSubItem>
      </SidebarMenuSub>
    </SidebarMenuItem>
  );
}

export function TeamNavigation({
  slug,
  closeNavigation,
}: {
  slug: string;
  closeNavigation: () => void;
}) {
  const { detail, error, retry } = useIssueWorkspace(slug);
  const { pathname } = useLocation();
  const segments = pathname.split('/').filter(Boolean);
  const selectedKey =
    segments[0] === slug && segments[1] === 'team'
      ? segments[2]?.toUpperCase()
      : segments[0] === slug && segments[1] === 'issue'
        ? segments[2]?.slice(0, segments[2].lastIndexOf('-')).toUpperCase()
        : undefined;
  const home = segments[1] === 'team' && segments.length === 3;
  return (
    <nav aria-label="Team navigation">
      <SidebarGroup>
        <SidebarGroupLabel>Your teams</SidebarGroupLabel>
        <SidebarGroupContent>
          {error ? (
            <div className="px-2 text-sm text-muted-foreground">
              <p role="alert">Could not load teams.</p>
              <Button variant="ghost" size="sm" onClick={retry}>
                Try again
              </Button>
            </div>
          ) : !detail ? (
            <p className="px-2 text-sm text-muted-foreground" role="status">
              Loading teams…
            </p>
          ) : (
            <SidebarMenu>
              {detail.teams.map((team) => (
                <TeamMenu
                  key={team.id}
                  team={team}
                  slug={slug}
                  active={selectedKey === team.key}
                  home={home}
                  closeNavigation={closeNavigation}
                />
              ))}
            </SidebarMenu>
          )}
        </SidebarGroupContent>
      </SidebarGroup>
    </nav>
  );
}
