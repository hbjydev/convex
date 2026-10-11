import { SettingsTable } from '@/components/settings-table';
import { SettingsSortButton } from '@/components/settings-controls';
import {
  SettingsDirectory,
  SettingsToolbar,
  SettingsToolbarActions,
  SettingsBlock,
  SettingsPage,
  SettingsPanel,
  SettingsRow,
} from '@/components/settings-layout';
import { PageTitle, SectionTitle, Intro } from '@/components/typography';
import { Link, useNavigate } from '@tanstack/react-router';
import { ArrowDownIcon, ChevronLeftIcon, UsersRoundIcon } from 'lucide-react';
import { useRef, useState } from 'react';
import { api } from '@/api/client';
import { Field, Form, text } from '@/components/app-form';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { unwrap } from '@/lib/requests';
import { exportCsv } from './settings-csv';
import { SettingsSearch, type SettingsProps } from './settings-shared';

export function TeamsSettings({ detail, manage, load }: SettingsProps) {
  const [search, setSearch] = useState('');
  const [selected, setSelected] = useState<string | null>(null);
  const [ascending, setAscending] = useState(true);
  const openingButton = useRef<HTMLButtonElement | null>(null);
  const sortButton = useRef<HTMLButtonElement | null>(null);
  const teams = detail.teams
    .filter((team) =>
      `${team.name} ${team.key}`.toLowerCase().includes(search.toLowerCase()),
    )
    .sort((a, b) => (ascending ? 1 : -1) * a.name.localeCompare(b.name));
  const team = detail.teams.find((item) => item.id === selected);
  return (
    <SettingsDirectory>
      <PageTitle>Teams</PageTitle>
      <SettingsToolbar>
        <SettingsSearch
          label="Filter by name or identifier…"
          value={search}
          onChange={setSearch}
        />
        <SettingsToolbarActions>
          <Button
            variant="outline"
            className="rounded-full"
            onClick={() =>
              exportCsv(`${detail.workspace.slug}-teams.csv`, [
                ['Name', 'Identifier', 'Visibility'],
                ...teams.map((item) => [item.name, item.key, 'Workspace']),
              ])
            }
          >
            Export CSV
          </Button>
          {manage && (
            <Button asChild className="rounded-full">
              <Link
                to="/$workspaceSlug"
                params={{ workspaceSlug: detail.workspace.slug }}
                hash="create-team"
              >
                Create team
              </Link>
            </Button>
          )}
        </SettingsToolbarActions>
      </SettingsToolbar>
      <div className="overflow-x-auto">
        <SettingsTable kind="teams">
          <caption className="sr-only">Workspace teams</caption>
          <thead>
            <tr>
              <th aria-sort={ascending ? 'ascending' : 'descending'}>
                <SettingsSortButton
                  ref={sortButton}

                  onClick={() => setAscending(!ascending)}
                >
                  Name{' '}
                  <ArrowDownIcon className={ascending ? '' : 'rotate-180'} />
                </SettingsSortButton>
              </th>
              <th>Identifier</th>
              <th>Visibility</th>
              <th>Workflow</th>
            </tr>
          </thead>
          <tbody>
            {teams.length > 0 && (
              <tr
                data-group
                className="[&_th]:bg-muted [&_th]:py-1.5 [&_th]:text-xs [&_span]:ml-1.5 max-md:[&_th]:col-span-full"
              >
                <th colSpan={4} scope="rowgroup">
                  Active <span>{teams.length}</span>
                </th>
              </tr>
            )}
            {teams.map((item) => (
              <tr key={item.id}>
                <td>
                  <button
                    className="flex min-w-0 items-center gap-2 text-left font-medium text-foreground max-md:min-h-11 [&_svg]:size-4 [&_svg]:shrink-0 [&_svg]:text-muted-foreground focus-visible:rounded-lg focus-visible:outline-2 focus-visible:outline-ring focus-visible:outline-offset-4"
                    onClick={(event) => {
                      openingButton.current = event.currentTarget;
                      setSelected(item.id);
                    }}
                  >
                    <UsersRoundIcon aria-hidden="true" />
                    <span>{item.name}</span>
                  </button>
                </td>
                <td>{item.key}</td>
                <td>Workspace</td>
                <td>{item.statuses.length} statuses</td>
              </tr>
            ))}
            {teams.length === 0 && (
              <tr>
                <td
                  colSpan={4}
                  className="py-10! text-center max-md:col-span-full"
                >
                  {search ? 'No teams match your search.' : 'No teams yet.'}
                </td>
              </tr>
            )}
          </tbody>
        </SettingsTable>
      </div>
      <Dialog
        open={!!team}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      >
        {team && (
          <DialogContent
            onCloseAutoFocus={(event) => {
              event.preventDefault();
              (openingButton.current?.isConnected
                ? openingButton.current
                : sortButton.current
              )?.focus();
            }}
          >
            <DialogTitle className="mb-3 flex items-center gap-2 pr-8 text-[15px] font-medium">
              {team.name}
            </DialogTitle>
            <DialogDescription className="text-[13px] leading-[1.6] text-muted-foreground">
              {team.key} · Visible to all workspace members
            </DialogDescription>
            <SettingsBlock>
              <SectionTitle>Workflow</SectionTitle>
              <ol
                className="m-0 flex list-none flex-wrap gap-2 p-0 [&_li]:flex [&_li]:min-w-0 [&_li]:flex-col [&_li]:gap-1.5 [&_li]:rounded-lg [&_li]:border [&_li]:px-3 [&_li]:py-2.5 [&_li]:text-[13px] [&_small]:text-xs [&_small]:text-muted-foreground"
                aria-label={`${team.name} workflow`}
              >
                {team.statuses.map((status) => (
                  <li key={status.id}>
                    {status.name}
                    <small>{status.category}</small>
                  </li>
                ))}
              </ol>
              <p className="text-[13px] leading-[1.6] text-muted-foreground">
                Team identifiers stay fixed to preserve issue identifiers.
              </p>
            </SettingsBlock>
            {manage && (
              <Form
                submit="Save team name"
                action={async (form) => {
                  const result = unwrap(
                    await api.POST(
                      '/api/v1/workspaces/{workspace_id}/teams/{team_id}/rename',
                      {
                        params: {
                          path: {
                            workspace_id: detail.workspace.id,
                            team_id: team.id,
                          },
                        },
                        body: { name: text(form, 'name') },
                      },
                    ),
                  );
                  await load();
                  return result.message;
                }}
              >
                <Field
                  name="name"
                  label={`Name for ${team.key}`}
                  value={team.name}
                />
              </Form>
            )}
          </DialogContent>
        )}
      </Dialog>
    </SettingsDirectory>
  );
}

export function CreateTeam({ detail, load }: SettingsProps) {
  const navigate = useNavigate();
  const [name, setName] = useState('');
  const [key, setKey] = useState('');
  const [keyEdited, setKeyEdited] = useState(false);
  const params = { workspaceSlug: detail.workspace.slug };
  return (
    <>
      <Button
        asChild
        variant="ghost"
        className="mb-4 flex w-fit md:absolute md:top-2 md:left-6 md:mb-0"
      >
        <Link to="/$workspaceSlug" params={params} hash="teams">
          <ChevronLeftIcon />
          Back
        </Link>
      </Button>
      <SettingsPage>
        <PageTitle>Create a new team</PageTitle>
        <Intro>
          Create a new team to organize work with its own issue identifiers and
          workflow.
        </Intro>
        <Form
          className="mt-0 max-w-none [&>fieldset>[data-slot=field-group]]:gap-4 [&>fieldset>[data-slot=field-group]>button]:self-end"
          submit="Create team"
          action={async () => {
            const result = unwrap(
              await api.POST('/api/v1/workspaces/{workspace_id}/teams', {
                params: { path: { workspace_id: detail.workspace.id } },
                body: { name, key },
              }),
            );
            await load();
            await navigate({ to: '/$workspaceSlug', params, hash: 'teams' });
            return result.message;
          }}
        >
          <SettingsPanel>
            <SettingsRow>
              <label
                className="mb-0.5 block text-[13px] font-medium"
                htmlFor="team-name"
              >
                Icon &amp; Name
              </label>
              <div
                data-name-field
                className="flex items-center gap-2.5 [&>svg]:size-7 [&>svg]:shrink-0 [&>svg]:rounded-lg [&>svg]:bg-accent [&>svg]:p-1 [&>svg]:text-muted-foreground"
              >
                <UsersRoundIcon aria-hidden="true" />
                <Input
                  id="team-name"
                  aria-label="Team name"
                  name="name"
                  placeholder="e.g. Engineering"
                  required
                  maxLength={100}
                  value={name}
                  onChange={(event) => {
                    setName(event.target.value);
                    if (!keyEdited)
                      setKey(
                        event.target.value
                          .replace(/[^a-z0-9]/gi, '')
                          .slice(0, 3)
                          .toUpperCase(),
                      );
                  }}
                />
              </div>
            </SettingsRow>
            <SettingsRow>
              <div>
                <label
                  className="mb-0.5 block text-[13px] font-medium"
                  htmlFor="team-key"
                >
                  Identifier
                </label>
                <p id="team-key-hint">
                  Used as a prefix for team issues (e.g. ENG-123).
                  <br />
                  2–10 letters or numbers, starting with a letter.
                </p>
              </div>
              <Input
                id="team-key"
                className="md:w-64"
                aria-label="Team key"
                name="key"
                placeholder="e.g. ENG"
                value={key}
                onChange={(event) => {
                  setKey(event.target.value.toUpperCase());
                  setKeyEdited(true);
                }}
                required
                minLength={2}
                maxLength={10}
                pattern="[A-Z][A-Z0-9]{1,9}"
                aria-describedby="team-key-hint"
              />
            </SettingsRow>
          </SettingsPanel>
          <SettingsBlock>
            <SectionTitle>Team access</SectionTitle>
            <Intro>
              All workspace members can access the team and its content.
            </Intro>
            <SettingsPanel>
              <SettingsRow>
                <span className="mb-0.5 block text-[13px] font-medium">
                  Visibility
                </span>
                <span>Workspace</span>
              </SettingsRow>
            </SettingsPanel>
          </SettingsBlock>
          <SettingsBlock>
            <SectionTitle>Workflow</SectionTitle>
            <Intro>
              New teams start with the default workflow. Each new issue starts
              in Todo.
            </Intro>
            <SettingsPanel>
              <ol
                className="flex flex-wrap items-center gap-x-5 gap-y-3 py-5 text-[13px] text-muted-foreground"
                aria-label="Default team workflow"
              >
                {['Backlog', 'Todo', 'In Progress', 'Done', 'Canceled'].map(
                  (status) => (
                    <li key={status}>{status}</li>
                  ),
                )}
              </ol>
            </SettingsPanel>
          </SettingsBlock>
        </Form>
      </SettingsPage>
    </>
  );
}
