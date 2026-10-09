import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  NativeSelect,
  NativeSelectOption,
} from '@/components/ui/native-select';
import { Textarea } from '@/components/ui/textarea';
import { priorities, type Draft, type WorkspaceDetail } from './model';
export function IssueFields({
  draft,
  onChange,
  detail,
  teamId,
  disabled = false,
}: {
  draft: Draft;
  onChange: (draft: Draft) => void;
  detail: WorkspaceDetail;
  teamId: string;
  disabled?: boolean;
}) {
  const team = detail.teams.find((team) => team.id === teamId);
  return (
    <FieldGroup>
      <Field>
        <FieldLabel htmlFor="issue-title">Title</FieldLabel>
        <Input
          id="issue-title"
          value={draft.title}
          onChange={(event) =>
            onChange({ ...draft, title: event.target.value })
          }
          required
          maxLength={240}
          disabled={disabled}
        />
      </Field>
      <Field>
        <FieldLabel htmlFor="issue-description">Description</FieldLabel>
        <Textarea
          id="issue-description"
          value={draft.description}
          onChange={(event) =>
            onChange({ ...draft, description: event.target.value })
          }
          maxLength={50000}
          rows={9}
          disabled={disabled}
          aria-describedby="description-hint"
        />
        <FieldDescription id="description-hint">
          Write in Markdown. Your formatting is kept as text.
        </FieldDescription>
      </Field>
      <FieldGroup className="grid gap-4 sm:grid-cols-3">
        <Field>
          <FieldLabel htmlFor="issue-status">Status</FieldLabel>
          <NativeSelect
            id="issue-status"
            value={draft.status_id}
            onChange={(event) =>
              onChange({ ...draft, status_id: event.target.value })
            }
            required
            disabled={disabled}
          >
            {team?.statuses.map((status) => (
              <NativeSelectOption key={status.id} value={status.id}>
                {status.name}
              </NativeSelectOption>
            ))}
          </NativeSelect>
        </Field>
        <Field>
          <FieldLabel htmlFor="issue-priority">Priority</FieldLabel>
          <NativeSelect
            id="issue-priority"
            value={draft.priority}
            onChange={(event) =>
              onChange({
                ...draft,
                priority:
                  priorities.find((value) => value === event.target.value) ??
                  'none',
              })
            }
            disabled={disabled}
          >
            {priorities.map((priority) => (
              <NativeSelectOption key={priority} value={priority}>
                {priority.charAt(0).toUpperCase() + priority.slice(1)}
              </NativeSelectOption>
            ))}
          </NativeSelect>
        </Field>
        <Field>
          <FieldLabel htmlFor="issue-assignee">Assignee</FieldLabel>
          <NativeSelect
            id="issue-assignee"
            value={draft.assignee_membership_id ?? ''}
            onChange={(event) =>
              onChange({
                ...draft,
                assignee_membership_id: event.target.value || null,
              })
            }
            disabled={disabled}
          >
            <NativeSelectOption value="">Unassigned</NativeSelectOption>
            {detail.members
              .filter((member) => !member.disabled)
              .map((member) => (
                <NativeSelectOption key={member.id} value={member.id}>
                  {member.display_name}
                </NativeSelectOption>
              ))}
          </NativeSelect>
        </Field>
      </FieldGroup>
    </FieldGroup>
  );
}
