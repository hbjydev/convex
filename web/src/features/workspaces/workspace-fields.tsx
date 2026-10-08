import { useId, useState } from "react";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupInput,
  InputGroupText,
} from "@/components/ui/input-group";

const RESERVED_SLUGS = [
  "account",
  "api",
  "assets",
  "auth",
  "invite",
  "setup",
  "system",
  "workspaces",
];

function suggestSlug(name: string) {
  const slug = name
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .slice(0, 63)
    .replace(/^-+|-+$/g, "");
  return RESERVED_SLUGS.includes(slug) ? `${slug}-workspace` : slug;
}

export function WorkspaceFields({ setup = false }: { setup?: boolean }) {
  const id = useId();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [editedSlug, setEditedSlug] = useState(false);
  const prefix = `${window.location.host}/`;
  const slugError = RESERVED_SLUGS.includes(slug)
    ? "This URL is reserved for the application. Choose another slug."
    : "";
  return (
    <>
      <Field>
        <FieldLabel htmlFor={`${id}-name`}>Workspace name</FieldLabel>
        <Input
          id={`${id}-name`}
          name={setup ? "workspace_name" : "name"}
          value={name}
          required
          maxLength={100}
          autoComplete="off"
          onChange={(event) => {
            setName(event.target.value);
            if (!editedSlug) setSlug(suggestSlug(event.target.value));
          }}
        />
      </Field>
      <Field data-invalid={!!slugError}>
        <FieldLabel htmlFor={`${id}-slug`}>Workspace URL</FieldLabel>
        <InputGroup>
          <InputGroupInput
            id={`${id}-slug`}
            ref={(input) => {
              input?.setCustomValidity(slugError);
            }}
            name={setup ? "workspace_slug" : "slug"}
            value={slug}
            required
            minLength={3}
            maxLength={63}
            pattern="[a-z0-9]+(-[a-z0-9]+)*"
            title="Use 3–63 lowercase letters or numbers, separated by single hyphens."
            autoComplete="off"
            autoCapitalize="none"
            spellCheck={false}
            aria-describedby={`${id}-prefix ${id}-hint${slugError ? ` ${id}-error` : ""}`}
            aria-invalid={!!slugError}
            onChange={(event) => {
              setEditedSlug(true);
              setSlug(event.target.value.toLowerCase());
            }}
          />
          <InputGroupAddon className="max-w-1/2 min-w-0">
            <InputGroupText
              id={`${id}-prefix`}
              className="min-w-0"
              title={prefix}
            >
              <span className="truncate">{prefix}</span>
            </InputGroupText>
          </InputGroupAddon>
        </InputGroup>
        {slugError && <FieldError id={`${id}-error`}>{slugError}</FieldError>}
        <FieldDescription id={`${id}-hint`}>
          Choose a unique slug with 3–63 letters, numbers, or hyphens. This URL
          stays the same when you rename your workspace.
        </FieldDescription>
      </Field>
    </>
  );
}
