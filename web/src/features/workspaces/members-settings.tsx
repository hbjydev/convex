import { ArrowDownIcon, MoreHorizontalIcon } from 'lucide-react';
import { useRef, useState, type SubmitEvent } from 'react';
import { api } from '@/api/client';
import { useAuth } from '@/auth';
import { Form, RoleField, text } from '@/components/app-form';
import { Notice } from '@/components/feedback';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  NativeSelect,
  NativeSelectOption,
} from '@/components/ui/native-select';
import { Spinner } from '@/components/ui/spinner';
import { Textarea } from '@/components/ui/textarea';
import { errorMessage, unwrap } from '@/lib/requests';
import { exportCsv } from './settings-csv';
import {
  initials,
  SettingsSearch,
  type Schema,
  type SettingsProps,
} from './settings-shared';

export function MembersSettings({
  detail,
  manage,
  load,
  invitations,
}: SettingsProps & { invitations: Schema['Invitation'][] }) {
  const { account, reload } = useAuth();
  const [search, setSearch] = useState('');
  const [filter, setFilter] = useState('all');
  const [ascending, setAscending] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);
  const [removing, setRemoving] = useState(false);
  const openingButton = useRef<HTMLButtonElement | null>(null);
  const sortButton = useRef<HTMLButtonElement | null>(null);
  const owner = detail.workspace.role === 'owner';
  const members = detail.members
    .filter(
      (member) =>
        member.display_name.toLowerCase().includes(search.toLowerCase()) &&
        (filter === 'all' ||
          (filter === 'disabled' ? member.disabled : !member.disabled)),
    )
    .sort(
      (a, b) =>
        (ascending ? 1 : -1) * a.display_name.localeCompare(b.display_name),
    );
  const member = detail.members.find((item) => item.id === selected);
  const params = { path: { workspace_id: detail.workspace.id } };
  async function changeMember(item: Schema['Member'], role: string | null) {
    unwrap(
      await api.POST('/api/v1/workspaces/{workspace_id}/members', {
        params,
        body: { membership_id: item.id, role, remove: role === null },
      }),
    );
    if (item.user_id === account?.user.id) await reload();
    else await load();
  }
  return (
    <div className="settings-directory">
      <h1>Members</h1>
      <div className="settings-toolbar">
        <SettingsSearch
          label="Search by name…"
          value={search}
          onChange={setSearch}
        />
        <NativeSelect
          aria-label="Filter members"
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        >
          <NativeSelectOption value="all">All</NativeSelectOption>
          <NativeSelectOption value="active">Active</NativeSelectOption>
          <NativeSelectOption value="disabled">Disabled</NativeSelectOption>
        </NativeSelect>
        <div className="settings-toolbar-actions">
          <Button
            variant="outline"
            className="rounded-full"
            onClick={() =>
              exportCsv(`${detail.workspace.slug}-members.csv`, [
                ['Name', 'Role', 'Status', 'Team access'],
                ...members.map((item) => [
                  item.display_name,
                  item.role,
                  item.disabled ? 'Disabled' : 'Active',
                  'All teams',
                ]),
              ])
            }
          >
            Export CSV
          </Button>
          {manage && (
            <InviteDialog
              key={detail.workspace.id}
              workspace={detail.workspace}
              load={load}
            />
          )}
        </div>
      </div>
      <div className="settings-table-scroll">
        <table className="settings-table members-table">
          <caption className="sr-only">Workspace members</caption>
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
              <th>Status</th>
              <th>Teams</th>
              <th>
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {members.length > 0 && (
              <tr className="settings-group-row">
                <th colSpan={4} scope="rowgroup">
                  Members <span>{members.length}</span>
                </th>
              </tr>
            )}
            {members.map((item) => (
              <tr key={item.id}>
                <td aria-label={item.display_name}>
                  <div className="settings-person">
                    <Avatar size="sm">
                      <AvatarFallback>
                        {initials(item.display_name)}
                      </AvatarFallback>
                    </Avatar>
                    <span>
                      {item.display_name}
                      {item.user_id === account?.user.id && <small>You</small>}
                    </span>
                  </div>
                </td>
                <td>
                  <Badge
                    variant={item.disabled ? 'outline' : 'secondary'}
                    className="capitalize"
                  >
                    {item.disabled ? 'Disabled' : item.role}
                  </Badge>
                </td>
                <td>All teams</td>
                <td>
                  {manage && (owner || item.role === 'member') && (
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      aria-label={`Manage ${item.display_name}`}
                      onClick={(event) => {
                        openingButton.current = event.currentTarget;
                        setSelected(item.id);
                        setRemoving(false);
                      }}
                    >
                      <MoreHorizontalIcon />
                    </Button>
                  )}
                </td>
              </tr>
            ))}
            {members.length === 0 && (
              <tr>
                <td colSpan={4} className="settings-empty">
                  {search || filter !== 'all'
                    ? 'No members match your filters.'
                    : 'No members yet.'}
                </td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
      <Dialog
        open={!!member}
        onOpenChange={(open) => {
          if (!open) setSelected(null);
        }}
      >
        {member && (
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
              Manage {member.display_name}
            </DialogTitle>
            <DialogDescription className="muted">
              Change workspace access for this member.
            </DialogDescription>
            <Form
              submit="Save role"
              action={async (form) => {
                await changeMember(member, text(form, 'role'));
                return 'Role saved. Their sessions were revoked.';
              }}
            >
              <RoleField
                label={`Role for ${member.display_name}`}
                value={member.role}
                owner={owner}
                includeOwner
              />
            </Form>
            <section className="settings-block">
              <h2>Remove member</h2>
              {removing ? (
                <Form
                  variant="destructive"
                  submit="Confirm removal"
                  action={async () => {
                    await changeMember(member, null);
                    setSelected(null);
                    return 'Membership removed.';
                  }}
                >
                  <p className="muted">
                    Remove {member.display_name} from this workspace? Their
                    workspace access and sessions will be revoked.
                  </p>
                  <Button
                    variant="outline"
                    type="button"
                    onClick={() => setRemoving(false)}
                  >
                    Cancel
                  </Button>
                </Form>
              ) : (
                <Button variant="destructive" onClick={() => setRemoving(true)}>
                  Remove
                </Button>
              )}
            </section>
          </DialogContent>
        )}
      </Dialog>
      {manage && invitations.length > 0 && (
        <section className="settings-block">
          <h2>Invitations</h2>
          <ul className="invitation-list">
            {invitations.map((invite) => (
              <li key={invite.id}>
                <span className="capitalize">
                  {invite.role} ·{' '}
                  {invite.accepted_at
                    ? 'Accepted'
                    : invite.revoked_at
                      ? 'Revoked'
                      : new Date(invite.expires_at) < new Date()
                        ? 'Expired'
                        : `Expires ${new Date(invite.expires_at).toLocaleDateString()}`}
                </span>
                {!invite.accepted_at &&
                  !invite.revoked_at &&
                  new Date(invite.expires_at) >= new Date() && (
                    <Form
                      inline
                      variant="outline"
                      submit="Revoke"
                      action={async () => {
                        const result = unwrap(
                          await api.POST(
                            '/api/v1/workspaces/{workspace_id}/invitations/revoke',
                            { params, body: { invitation_id: invite.id } },
                          ),
                        );
                        await load();
                        return result.message;
                      }}
                    >
                      <span className="sr-only">Revoke invitation</span>
                    </Form>
                  )}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

type InvitationResult = { email: string; link: Schema['InviteLink'] };

function InviteDialog({
  workspace,
  load,
}: {
  workspace: Schema['Workspace'];
  load: () => Promise<void>;
}) {
  const { bootstrap } = useAuth();
  const [open, setOpen] = useState(false);
  const [emails, setEmails] = useState('');
  const [role, setRole] = useState('member');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const [results, setResults] = useState<InvitationResult[]>([]);
  const [copyMessage, setCopyMessage] = useState('');
  async function send(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setError('');
    setMessage('');
    const recipients = bootstrap.mail_available
      ? [
          ...new Set(
            emails
              .split(/[,;\n]+/)
              .map((email) => email.trim())
              .filter(Boolean),
          ),
        ]
      : [''];
    if (recipients.length === 0 || recipients.length > 20) {
      setError('Enter between 1 and 20 email addresses.');
      return;
    }
    const validator = document.createElement('input');
    validator.type = 'email';
    if (
      bootstrap.mail_available &&
      recipients.some((email) => {
        validator.value = email;
        return !validator.checkValidity() || email.length > 254;
      })
    ) {
      setError('Check the email addresses and try again.');
      return;
    }
    setPending(true);
    const remaining = [...recipients];
    try {
      for (const email of recipients) {
        const link = unwrap(
          await api.POST('/api/v1/workspaces/{workspace_id}/invitations', {
            params: { path: { workspace_id: workspace.id } },
            body: { role, deliver_to: email || null },
          }),
        );
        setResults((current) => [...current, { email, link }]);
        remaining.shift();
        setEmails(remaining.join(', '));
      }
      setMessage(
        bootstrap.mail_available
          ? 'Invitations created. Delivery status and links are below.'
          : 'Invitation ready to share.',
      );
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      try {
        await load();
      } catch (error) {
        setError(`Invitations could not be refreshed. ${errorMessage(error)}`);
      }
      setPending(false);
    }
  }
  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (!pending) setOpen(value);
      }}
    >
      <DialogTrigger asChild>
        <Button className="rounded-full">Invite</Button>
      </DialogTrigger>
      <DialogContent
        onEscapeKeyDown={(event) => {
          if (pending) event.preventDefault();
        }}
        onPointerDownOutside={(event) => {
          if (pending) event.preventDefault();
        }}
      >
        <DialogTitle className="settings-dialog-title">
          <span className="workspace-avatar" aria-hidden="true">
            {initials(workspace.name)}
          </span>
          Invite to your workspace
        </DialogTitle>
        <DialogDescription className="sr-only">
          Create single-use workspace invitations. Links expire after seven
          days.
        </DialogDescription>
        <form
          className="settings-invite-form"
          onSubmit={send}
          aria-busy={pending}
        >
          <fieldset disabled={pending} className="flex min-w-0 flex-col gap-4">
            {bootstrap.mail_available ? (
              <div>
                <label htmlFor="invite-emails" className="settings-label">
                  Email
                </label>
                <Textarea
                  id="invite-emails"
                  name="emails"
                  value={emails}
                  onChange={(event) => setEmails(event.target.value)}
                  placeholder="email@example.com, email2@example.com…"
                  required
                  aria-describedby="invite-hint"
                />
                <p id="invite-hint" className="muted mt-2">
                  Separate addresses with commas or new lines. Up to 20 at a
                  time.
                </p>
              </div>
            ) : (
              <p className="muted">
                Email delivery is unavailable. Create a single-use link to share
                with your teammate.
              </p>
            )}
            <div>
              <label htmlFor="invite-role" className="settings-label">
                Invitation role
              </label>
              <NativeSelect
                id="invite-role"
                value={role}
                onChange={(event) => setRole(event.target.value)}
              >
                <NativeSelectOption value="member">Member</NativeSelectOption>
                {workspace.role === 'owner' && (
                  <NativeSelectOption value="admin">Admin</NativeSelectOption>
                )}
              </NativeSelect>
            </div>
            <p className="muted">
              Links expire after seven days. Share only with the intended
              teammate; anyone with the link can accept it.
            </p>
            <Button
              type="submit"
              className="ml-auto rounded-full"
              disabled={pending}
            >
              {pending && <Spinner aria-hidden="true" />}
              {pending
                ? 'Working…'
                : bootstrap.mail_available
                  ? 'Send invites'
                  : 'Create invitation'}
            </Button>
          </fieldset>
        </form>
        {error && <Notice error>{error}</Notice>}
        {message && <Notice>{message}</Notice>}
        {results.length > 0 && (
          <div className="settings-invite-results">
            {results.map(({ email, link }, index) => (
              <div className="invite-link" key={link.id}>
                <label
                  className="settings-label"
                  htmlFor={`invite-link-${index}`}
                >
                  {email || 'Single-use invitation link'}
                </label>
                {email && (
                  <p className="muted">
                    {link.email_delivered
                      ? 'Sent by email'
                      : 'Email delivery failed. Share this link directly.'}
                  </p>
                )}
                <div className="flex w-full min-w-0 gap-2">
                  <Input
                    id={`invite-link-${index}`}
                    aria-label={
                      email
                        ? `Invitation link for ${email}`
                        : 'Single-use invitation link'
                    }
                    value={link.url}
                    readOnly
                    onFocus={(event) => event.currentTarget.select()}
                  />
                  <Button
                    variant="outline"
                    onClick={async () => {
                      try {
                        await navigator.clipboard.writeText(link.url);
                        setCopyMessage('Copied');
                      } catch {
                        setCopyMessage(
                          'Select and copy the invitation link manually.',
                        );
                      }
                    }}
                  >
                    Copy link
                  </Button>
                </div>
              </div>
            ))}
            <p className="muted" role="status">
              {copyMessage}
            </p>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
