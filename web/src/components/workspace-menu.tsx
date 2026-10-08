import { Link } from '@tanstack/react-router';
import {
  ArrowLeftIcon,
  ArrowLeftRightIcon,
  CheckIcon,
  ChevronDownIcon,
  ChevronRightIcon,
  LayersIcon,
  LogOutIcon,
  PlusIcon,
  SettingsIcon,
  UsersIcon,
} from 'lucide-react';
import { useState } from 'react';
import { api } from '@/api/client';
import type { components } from '@/api/schema';
import { useAuth } from '@/auth';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { SidebarMenuButton, useSidebar } from '@/components/ui/sidebar';
import { errorMessage, unwrap } from '@/lib/requests';

export function WorkspaceMenu({
  workspace,
  closeNavigation,
  onError,
}: {
  workspace: components['schemas']['Workspace'] | undefined;
  closeNavigation: () => void;
  onError: (message: string) => void;
}) {
  const { account, reload } = useAuth();
  const { isMobile } = useSidebar();
  const [mobileSwitching, setMobileSwitching] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
  async function signOut() {
    setSigningOut(true);
    onError('');
    try {
      unwrap(await api.POST('/api/v1/auth/logout'));
      closeNavigation();
      await reload();
    } catch (error) {
      onError(errorMessage(error));
      closeNavigation();
    } finally {
      setSigningOut(false);
    }
  }

  const workspaceChoices = account && (
    <>
      <DropdownMenuLabel>Your workspaces</DropdownMenuLabel>
      <DropdownMenuGroup>
        {account.workspaces.map((space) => (
          <DropdownMenuItem asChild key={space.id}>
            <Link
              to="/$workspaceSlug"
              params={{ workspaceSlug: space.slug }}
              onClick={closeNavigation}
              aria-current={workspace?.id === space.id ? 'page' : undefined}
            >
              <span className="workspace-mark" aria-hidden="true">
                {space.name.slice(0, 1).toUpperCase()}
              </span>
              <span className="truncate">{space.name}</span>
              {workspace?.id === space.id && (
                <CheckIcon className="ml-auto" aria-label="Current workspace" />
              )}
            </Link>
          </DropdownMenuItem>
        ))}
      </DropdownMenuGroup>
      <DropdownMenuSeparator />
      <DropdownMenuGroup>
        <DropdownMenuItem asChild>
          <Link to="/" onClick={closeNavigation}>
            <LayersIcon />
            All workspaces
          </Link>
        </DropdownMenuItem>
        {account.user.is_instance_admin && (
          <DropdownMenuItem asChild>
            <Link to="/" hash="create-workspace" onClick={closeNavigation}>
              <PlusIcon />
              Create a workspace
            </Link>
          </DropdownMenuItem>
        )}
      </DropdownMenuGroup>
    </>
  );

  return (
    <DropdownMenu
      onOpenChange={(open) => {
        if (!open) setMobileSwitching(false);
      }}
    >
      <DropdownMenuTrigger asChild>
        <SidebarMenuButton
          className="w-auto max-w-full"
          disabled={signingOut}
          aria-label={`${workspace?.name ?? 'Convex'} workspace menu`}
        >
          <span className="workspace-avatar" aria-hidden="true">
            {workspace?.name.slice(0, 2).toUpperCase() ?? <LayersIcon />}
          </span>
          <span className="truncate">{workspace?.name ?? 'Convex'}</span>
          <ChevronDownIcon />
        </SidebarMenuButton>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="start" className="w-60" sideOffset={8}>
        {isMobile && mobileSwitching ? (
          <>
            <DropdownMenuGroup>
              <DropdownMenuItem
                onSelect={(event) => {
                  event.preventDefault();
                  setMobileSwitching(false);
                }}
              >
                <ArrowLeftIcon />
                Back
              </DropdownMenuItem>
            </DropdownMenuGroup>
            <DropdownMenuSeparator />
            {workspaceChoices}
          </>
        ) : (
          <>
            {workspace && (
              <DropdownMenuGroup>
                <DropdownMenuItem asChild>
                  <Link
                    to="/$workspaceSlug"
                    params={{ workspaceSlug: workspace.slug }}
                    onClick={closeNavigation}
                  >
                    <SettingsIcon />
                    Settings
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuItem asChild>
                  <Link
                    to="/$workspaceSlug"
                    params={{ workspaceSlug: workspace.slug }}
                    hash="people"
                    onClick={closeNavigation}
                  >
                    <UsersIcon />
                    {workspace.role === 'member'
                      ? 'View members'
                      : 'Invite and manage members'}
                  </Link>
                </DropdownMenuItem>
              </DropdownMenuGroup>
            )}
            {workspace && <DropdownMenuSeparator />}
            {account && (
              <>
                <DropdownMenuGroup>
                  {isMobile ? (
                    <DropdownMenuItem
                      onSelect={(event) => {
                        event.preventDefault();
                        setMobileSwitching(true);
                      }}
                    >
                      <ArrowLeftRightIcon />
                      Switch workspace
                      <ChevronRightIcon className="ml-auto" />
                    </DropdownMenuItem>
                  ) : (
                    <DropdownMenuSub>
                      <DropdownMenuSubTrigger>
                        <ArrowLeftRightIcon />
                        Switch workspace
                      </DropdownMenuSubTrigger>
                      <DropdownMenuSubContent className="w-60">
                        {workspaceChoices}
                      </DropdownMenuSubContent>
                    </DropdownMenuSub>
                  )}
                </DropdownMenuGroup>
                <DropdownMenuSeparator />
                <DropdownMenuGroup>
                  <DropdownMenuItem
                    disabled={signingOut}
                    onSelect={() => void signOut()}
                  >
                    <LogOutIcon />
                    Sign out
                  </DropdownMenuItem>
                </DropdownMenuGroup>
              </>
            )}
            {!account && (
              <DropdownMenuGroup>
                <DropdownMenuItem asChild>
                  <Link to="/auth/login" onClick={closeNavigation}>
                    Sign in
                  </Link>
                </DropdownMenuItem>
              </DropdownMenuGroup>
            )}
          </>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
