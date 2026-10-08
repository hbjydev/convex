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
    <div className="settings-directory">
      <h1>Teams</h1>
      <div className="settings-toolbar">
        <SettingsSearch
          label="Filter by name or identifier…"
          value={search}
          onChange={setSearch}
        />
        <div className="settings-toolbar-actions">
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
        </div>
      </div>
      <div className="settings-table-scroll">
        <table className="settings-table teams-table">
          <caption className="sr-only">Workspace teams</caption>
          <thead>
            <tr>
              <th aria-sort={ascending ? 'ascending' : 'descending'}>
                <button
                  ref={sortButton}
                  className="settings-sort"
                  onClick={() => setAscending(!ascending)}
                >
                  Name{' '}
                  <ArrowDownIcon className={ascending ? '' : 'rotate-180'} />
                </button>
              </th>
              <th>Identifier</th>
              <th>Visibility</th>
              <th>Workflow</th>
            </tr>
          </thead>
          <tbody>
            {teams.length > 0 && (
              <tr className="settings-group-row">
                <th colSpan={4} scope="rowgroup">
                  Active <span>{teams.length}</span>
                </th>
              </tr>
            )}
            {teams.map((item) => (
              <tr key={item.id}>
                <td>
                  <button
                    className="settings-name-button"
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
                <td colSpan={4} className="settings-empty">
                  {search ? 'No teams match your search.' : 'No teams yet.'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
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
            <DialogTitle className="settings-dialog-title">
              {team.name}
            </DialogTitle>
            <DialogDescription className="muted">
              {team.key} · Visible to all workspace members
            </DialogDescription>
            <section className="settings-block">
              <h2>Workflow</h2>
              <ol className="workflow" aria-label={`${team.name} workflow`}>
                {team.statuses.map((status) => (
                  <li key={status.id}>
                    {status.name}
                    <small>{status.category}</small>
                  </li>
                ))}
              </ol>
              <p className="muted">
                Team identifiers stay fixed to preserve issue identifiers.
              </p>
            </section>
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
    </div>
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
      <Button asChild variant="ghost" className="settings-back">
        <Link to="/$workspaceSlug" params={params} hash="teams">
          <ChevronLeftIcon />
          Back
        </Link>
      </Button>
      <div className="settings-narrow">
        <h1>Create a new team</h1>
        <p className="intro">
          Create a new team to organize work with its own issue identifiers and
          workflow.
        </p>
        <Form
          className="settings-profile-form"
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
          <div className="settings-panel">
            <div className="settings-row">
              <label className="settings-label" htmlFor="team-name">
                Icon &amp; Name
              </label>
              <div className="settings-name-field">
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
            </div>
            <div className="settings-row">
              <div>
                <label className="settings-label" htmlFor="team-key">
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
            </div>
          </div>
          <section className="settings-block">
            <h2>Team access</h2>
            <p className="intro">
              All workspace members can access the team and its content.
            </p>
            <div className="settings-panel">
              <div className="settings-row">
                <span className="settings-label">Visibility</span>
                <span>Workspace</span>
              </div>
            </div>
          </section>
          <section className="settings-block">
            <h2>Workflow</h2>
            <p className="intro">
              New teams start with the default workflow. Each new issue starts
              in Todo.
            </p>
            <div className="settings-panel">
              <ol
                className="settings-default-workflow"
                aria-label="Default team workflow"
              >
                {['Backlog', 'Todo', 'In Progress', 'Done', 'Canceled'].map(
                  (status) => (
                    <li key={status}>{status}</li>
                  ),
                )}
              </ol>
            </div>
          </section>
        </Form>
      </div>
    </>
  );
}
