import {
  createContext,
  type ReactNode,
  type SubmitEvent,
  useContext,
  useId,
  useState,
} from 'react';
import { Notice } from '@/components/feedback';
import { Button } from '@/components/ui/button';
import {
  FieldDescription,
  FieldGroup,
  FieldLabel,
  FieldSet,
  Field as UIField,
} from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  NativeSelect,
  NativeSelectOption,
} from '@/components/ui/native-select';
import { Spinner } from '@/components/ui/spinner';
import { errorMessage } from '@/lib/requests';
import { cn } from '@/lib/utils';

const FormPending = createContext(false);

export function Form({
  children,
  className,
  submit,
  action,
  onSuccess,
  inline = false,
  hideSubmit = false,
  variant = 'default',
}: {
  children: ReactNode;
  className?: string;
  submit: string;
  action: (form: FormData) => Promise<string | undefined>;
  onSuccess?: () => void;
  inline?: boolean;
  hideSubmit?: boolean;
  variant?: 'default' | 'outline' | 'destructive';
}) {
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  async function handle(event: SubmitEvent<HTMLFormElement>) {
    event.preventDefault();
    setPending(true);
    setError('');
    setMessage('');
    const form = event.currentTarget;
    try {
      const message = await action(new FormData(form));
      setMessage(message ?? 'Saved.');
      onSuccess?.();
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }

  const formClassName = cn('app-form', className, inline && 'inline-form');

  return (
    <form onSubmit={handle} className={formClassName} aria-busy={pending}>
      <FormPending.Provider value={pending}>
        <FieldSet disabled={pending} className="min-w-0">
          <FieldGroup
            className={cn(
              inline &&
                'flex-row flex-wrap items-center gap-2 [container-type:normal]',
            )}
          >
            {children}
            {!hideSubmit && (
              <Button
                variant={variant}
                className="w-fit"
                type="submit"
                disabled={pending}
              >
                {pending && (
                  <Spinner
                    data-icon="inline-start"
                    role="presentation"
                    aria-hidden="true"
                  />
                )}
                {pending ? 'Working…' : submit}
              </Button>
            )}
          </FieldGroup>
        </FieldSet>
      </FormPending.Provider>
      {error && <Notice error>{error}</Notice>}
      {message && <Notice>{message}</Notice>}
    </form>
  );
}
export function Field({
  name,
  label,
  type = 'text',
  autoComplete,
  value,
  hint,
  required = true,
}: {
  name: string;
  label: string;
  type?: string;
  autoComplete?: string;
  value?: string;
  hint?: string;
  required?: boolean;
}) {
  const id = useId();
  const pending = useContext(FormPending);
  return (
    <UIField data-disabled={pending}>
      <FieldLabel htmlFor={id}>{label}</FieldLabel>
      <Input
        id={id}
        name={name}
        type={type}
        autoComplete={autoComplete}
        defaultValue={value}
        required={required}
        disabled={pending}
        minLength={type === 'password' ? 12 : undefined}
        maxLength={type === 'password' ? 128 : type === 'email' ? 254 : 100}
        aria-describedby={hint ? `${id}-hint` : undefined}
      />
      {hint && <FieldDescription id={`${id}-hint`}>{hint}</FieldDescription>}
    </UIField>
  );
}
export function text(form: FormData, key: string) {
  return String(form.get(key) ?? '');
}
export function RoleField({
  label,
  value = 'member',
  owner,
  includeOwner = false,
  hiddenLabel = false,
}: {
  label: string;
  value?: string;
  owner: boolean;
  includeOwner?: boolean;
  hiddenLabel?: boolean;
}) {
  const id = useId();
  const pending = useContext(FormPending);
  return (
    <UIField
      data-disabled={pending}
      className={cn(hiddenLabel && 'w-28 shrink-0')}
    >
      <FieldLabel htmlFor={id} className={cn(hiddenLabel && 'sr-only')}>
        {label}
      </FieldLabel>
      <NativeSelect id={id} name="role" defaultValue={value} disabled={pending}>
        <NativeSelectOption value="member">Member</NativeSelectOption>
        {owner && <NativeSelectOption value="admin">Admin</NativeSelectOption>}
        {owner && includeOwner && (
          <NativeSelectOption value="owner">Owner</NativeSelectOption>
        )}
      </NativeSelect>
    </UIField>
  );
}
export function Credentials({ creating = false }: { creating?: boolean }) {
  return (
    <>
      {creating && (
        <Field name="display_name" label="Your name" autoComplete="name" />
      )}
      <Field name="email" label="Email" type="email" autoComplete="email" />
      <Field
        name="password"
        label="Password"
        type="password"
        autoComplete={creating ? 'new-password' : 'current-password'}
        hint={creating ? 'Use 12–128 characters.' : undefined}
      />
    </>
  );
}
