import { useNavigate } from '@tanstack/react-router';
import { ChevronRightIcon, UsersIcon, LayersIcon } from 'lucide-react';
import { useState } from 'react';
import { PropertyMenu } from './property-menu';
import { api } from '@/api/client';
import { Notice } from '@/components/feedback';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldGroup, FieldLabel, FieldSet } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Textarea } from '@/components/ui/textarea';
import { Spinner } from '@/components/ui/spinner';
import { errorMessage, unwrap } from '@/lib/requests';
import {
  issueRouteParams,
  priorities,
  type Draft,
  type WorkspaceDetail,
} from './model';
import { PriorityIcon, StatusIcon } from './icons';
export function CreateIssueDialog({
  detail,
  slug,
  open,
  onOpenChange,
  onCreated,
  initialStatusId,
  initialTeamId,
}: {
  detail: WorkspaceDetail;
  slug: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: () => void;
  initialStatusId?: string;
  initialTeamId?: string;
}) {
  const navigate = useNavigate();
  const first =
    detail.teams.find((team) =>
      team.statuses.some((status) => status.id === initialStatusId),
    ) ??
    detail.teams.find((team) => team.id === initialTeamId) ??
    detail.teams[0];
  const [teamId, setTeamId] = useState(first?.id ?? '');
  const emptyDraft: Draft = {
    title: '',
    description: '',
    priority: 'none',
    status_id:
      initialStatusId ??
      first?.statuses.find((status) => status.category === 'unstarted')?.id ??
      '',
    assignee_membership_id: null,
  };
  const [draft, setDraft] = useState<Draft>(emptyDraft);
  const [createMore, setCreateMore] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState('');
  const [message, setMessage] = useState('');
  const team = detail.teams.find((team) => team.id === teamId);
  async function create() {
    setPending(true);
    setError('');
    setMessage('');
    try {
      const issue = unwrap(
        await api.POST('/api/v1/workspaces/{workspace_id}/issues', {
          params: { path: { workspace_id: detail.workspace.id } },
          body: { team_id: teamId, ...draft },
        }),
      );
      onCreated();
      if (createMore) {
        setDraft({ ...draft, title: '', description: '' });
        setMessage(`${issue.identifier} created.`);
        window.requestAnimationFrame(() =>
          document.getElementById('new-issue-title')?.focus(),
        );
      } else {
        onOpenChange(false);
        await navigate({
          to: '/$workspaceSlug/issue/$issueKey/$issueTitle',
          params: issueRouteParams(slug, issue),
        });
      }
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setPending(false);
    }
  }
  return (
    <Dialog
      open={open}
      onOpenChange={(value) => {
        if (!pending) onOpenChange(value);
      }}
    >
      <DialogContent
        className="w-[min(750px,calc(100vw-32px))] max-w-[750px] rounded-xl p-4"
        onOpenAutoFocus={(event) => {
          event.preventDefault();
          document.getElementById('new-issue-title')?.focus();
        }}
        onInteractOutside={(event) => {
          if (draft.title || draft.description || pending)
            event.preventDefault();
        }}
      >
        <DialogDescription className="sr-only">
          Create an issue for your team. Write a title, add a Markdown
          description, and choose its properties.
        </DialogDescription>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void create();
          }}
          aria-busy={pending}
        >
          <FieldSet disabled={pending} className="gap-0">
            <div className="mb-3.5 flex items-center gap-2 pr-8 [&>svg]:size-3.5 [&>svg]:text-muted-foreground [&_h2]:text-[13px] [&_h2]:font-medium">
              <PropertyMenu
                label="Team"
                value={teamId}
                options={detail.teams.map((team) => ({
                  value: team.id,
                  label: team.key,
                }))}
                icon={<LayersIcon data-icon="inline-start" />}
                onChange={(value) => {
                  const team = detail.teams.find((team) => team.id === value);
                  setTeamId(value);
                  setDraft({
                    ...draft,
                    status_id:
                      team?.statuses.find(
                        (status) => status.category === 'unstarted',
                      )?.id ?? '',
                  });
                }}
              />
              <ChevronRightIcon aria-hidden="true" />
              <DialogTitle>New issue</DialogTitle>
            </div>
            <FieldGroup className="gap-1 [container-type:normal]">
              <Field>
                <FieldLabel className="sr-only" htmlFor="new-issue-title">
                  Title
                </FieldLabel>
                <Input
                  className="h-auto min-h-9 border-transparent bg-transparent p-1.5 text-lg font-medium shadow-none md:text-lg"
                  id="new-issue-title"
                  placeholder="Issue title"
                  value={draft.title}
                  onChange={(event) =>
                    setDraft({ ...draft, title: event.target.value })
                  }
                  required
                  maxLength={240}
                />
              </Field>
              <Field>
                <FieldLabel className="sr-only" htmlFor="new-issue-description">
                  Description
                </FieldLabel>
                <Textarea
                  className="min-h-25 resize-y border-transparent bg-transparent p-1.5 shadow-none"
                  id="new-issue-description"
                  placeholder="Add description…"
                  value={draft.description}
                  onChange={(event) =>
                    setDraft({ ...draft, description: event.target.value })
                  }
                  maxLength={50000}
                  rows={4}
                />
              </Field>
            </FieldGroup>
            <div className="mt-3.5 mb-6 flex flex-wrap gap-1.5">
              <PropertyMenu
                label="Status"
                value={draft.status_id}
                options={
                  team?.statuses.map((status) => ({
                    value: status.id,
                    label: status.name,
                  })) ?? []
                }
                icon={
                  <StatusIcon
                    category={
                      team?.statuses.find(
                        (status) => status.id === draft.status_id,
                      )?.category
                    }
                  />
                }
                onChange={(value) => setDraft({ ...draft, status_id: value })}
              />
              <PropertyMenu
                label="Priority"
                value={draft.priority}
                options={priorities.map((value) => ({
                  value,
                  label:
                    value === 'none'
                      ? 'Priority'
                      : value.charAt(0).toUpperCase() + value.slice(1),
                }))}
                icon={<PriorityIcon priority={draft.priority} />}
                onChange={(value) =>
                  setDraft({
                    ...draft,
                    priority:
                      priorities.find((priority) => priority === value) ??
                      'none',
                  })
                }
              />
              <PropertyMenu
                label="Assignee"
                value={draft.assignee_membership_id ?? ''}
                options={[
                  { value: '', label: 'Assignee' },
                  ...detail.members
                    .filter((member) => !member.disabled)
                    .map((member) => ({
                      value: member.id,
                      label: member.display_name,
                    })),
                ]}
                icon={<UsersIcon data-icon="inline-start" />}
                onChange={(value) =>
                  setDraft({ ...draft, assignee_membership_id: value || null })
                }
              />
            </div>
            {error && <Notice error>{error}</Notice>}
            {message && <Notice>{message}</Notice>}
            <div className="flex items-center justify-between gap-4 [&_label]:text-xs [&_label]:text-muted-foreground [&_input[type=checkbox]]:accent-primary max-[600px]:flex-col max-[600px]:items-start max-[600px]:gap-3 max-[600px]:[&>div]:w-full max-[600px]:[&>div]:justify-end">
              <span className="text-xs text-muted-foreground">
                Markdown supported
              </span>
              <div className="flex flex-wrap items-center gap-3">
                <Field orientation="horizontal" className="w-auto">
                  <input
                    id="create-more"
                    type="checkbox"
                    checked={createMore}
                    onChange={(event) => setCreateMore(event.target.checked)}
                  />
                  <FieldLabel htmlFor="create-more">Create more</FieldLabel>
                </Field>
                <Button type="submit" disabled={pending || !draft.title.trim()}>
                  {pending && <Spinner data-icon="inline-start" />}Create issue
                </Button>
              </div>
            </div>
          </FieldSet>
        </form>
      </DialogContent>
    </Dialog>
  );
}
