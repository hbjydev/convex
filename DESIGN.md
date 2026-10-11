---
name: Convex
description: Compact workspace administration with system-aware neutral surfaces.
colors:
  background: "#ffffff"
  foreground: "#24242b"
  card: "#ffffff"
  card-foreground: "#24242b"
  popover: "#ffffff"
  popover-foreground: "#24242b"
  primary: "#5555ca"
  primary-foreground: "#ffffff"
  secondary: "#ededf1"
  secondary-foreground: "#24242b"
  muted: "#f4f4f6"
  muted-foreground: "#676773"
  accent: "#ededf1"
  accent-foreground: "#24242b"
  destructive: "#c0333c"
  border: "#e5e5e9"
  input: "#d5d5dc"
  ring: "#5555ca"
  sidebar: "#f7f7f8"
  sidebar-foreground: "#45454f"
  sidebar-accent: "#eaeaec"
  sidebar-accent-foreground: "#24242b"
  background-dark: "#141416"
  foreground-dark: "#e9e9ee"
  card-dark: "#1b1b1f"
  card-foreground-dark: "#e9e9ee"
  popover-dark: "#232328"
  popover-foreground-dark: "#e9e9ee"
  primary-dark: "#a5a5ff"
  primary-foreground-dark: "#151522"
  secondary-dark: "#2b2b32"
  secondary-foreground-dark: "#e9e9ee"
  muted-dark: "#232328"
  muted-foreground-dark: "#a0a0ad"
  accent-dark: "#2b2b32"
  accent-foreground-dark: "#e9e9ee"
  destructive-dark: "#ff858c"
  border-dark: "#303036"
  input-dark: "#404049"
  ring-dark: "#a5a5ff"
  sidebar-dark: "#1b1b1f"
  sidebar-foreground-dark: "#c8c8d0"
  sidebar-accent-dark: "#2b2b32"
  sidebar-accent-foreground-dark: "#e9e9ee"
typography:
  headline:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "22px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "-0.035em"
  title:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "15px"
    fontWeight: 600
  body:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.5
  paragraph:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.6
  control:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "14px"
    fontWeight: 500
    lineHeight: "20px"
  label:
    fontFamily: "\"Geist Variable\", sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: "16px"
rounded:
  sm: "3.6px"
  md: "4.8px"
  lg: "6px"
  xl: "8.4px"
  4xl: "15.6px"
spacing:
  step-4: "4px"
  step-6: "6px"
  step-8: "8px"
  step-10: "10px"
  step-12: "12px"
  step-16: "16px"
  step-20: "20px"
  step-24: "24px"
  step-28: "28px"
  step-32: "32px"
  step-40: "40px"
  step-48: "48px"
  step-64: "64px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.primary-foreground}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-outline:
    backgroundColor: "{colors.background}"
    textColor: "{colors.foreground}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-secondary:
    backgroundColor: "{colors.secondary}"
    textColor: "{colors.secondary-foreground}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-ghost:
    textColor: "{colors.foreground}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-destructive:
    textColor: "{colors.destructive}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  button-link:
    textColor: "{colors.primary}"
    typography: "{typography.control}"
    rounded: "{rounded.lg}"
    padding: "0 10px"
    height: "32px"
  input:
    textColor: "{colors.foreground}"
    rounded: "{rounded.lg}"
    padding: "4px 10px"
    height: "32px"
  navigation:
    backgroundColor: "{colors.sidebar}"
    textColor: "{colors.sidebar-foreground}"
    typography: "{typography.control}"
    rounded: "{rounded.md}"
    padding: "8px"
    height: "32px"
  chip-outline:
    textColor: "{colors.foreground}"
    typography: "{typography.label}"
    rounded: "{rounded.4xl}"
    padding: "2px 8px"
    height: "20px"
  auth-card:
    backgroundColor: "{colors.card}"
    textColor: "{colors.card-foreground}"
    rounded: "{rounded.xl}"
    padding: "16px"
    width: "min(100%, 448px)"
---

# Design System: Convex

## Overview

**Creative North Star: "The compact workspace"**

Convex uses the Linear-inspired workspace convention chosen by the user: quiet white or charcoal surfaces, restrained indigo actions, fine dividers, Geist, and compact shadcn Nova controls. The workspace name leads navigation; the current task has a clear heading and a predictable reading order.

The interface is code-led. It uses SVG icons and initials rather than shipping decorative raster imagery. System light and dark themes retain the same spacing and hierarchy.

**Key Characteristics:**

- Compact navigation with generous task spacing.
- Neutral surfaces; indigo actions and focus.
- Small corners, fine borders, and restrained menu elevation.
- System themes, keyboard access, and responsive touch targets.

## Colors

The palette pairs restrained indigo with neutral surfaces and red destructive feedback. Frontmatter colors are the source values from `web/src/styles.css`; `-dark` siblings document the `.dark` overrides, not an independent theme API. Runtime code always uses the semantic CSS variables. Card/popover foregrounds mirror text; sidebar primary, border, and ring aliases mirror their corresponding semantic roles.

### Primary

- **Workspace Indigo:** `primary` and `primary-dark` identify principal actions, links, workspace avatars, selection, and text carets. `primary-foreground` supplies contrasting action text.
- **Focus Indigo:** `ring` follows primary; use the existing ring treatments for keyboard focus.

### Neutral

- **Task White / Task Charcoal:** `background` is the reading surface; `foreground` is its main text.
- **Panel White / Panel Charcoal:** `card` frames authentication; `popover` lifts menus above the current surface.
- **Quiet Gray:** `muted` and `muted-foreground` distinguish secondary prose and list headers. `secondary` and `accent` provide low-key controls and interaction fills.
- **Navigation Mist / Navigation Charcoal:** `sidebar` and its foreground/accent roles distinguish navigation and current location.
- **Fine Divider / Field Stroke:** `border` separates groups and rows; the stronger `input` outline identifies editable controls.

Destructive red is a feedback role, not a second brand accent; use the destructive component's translucent fill and red text.

**The Semantic Surface Rule.** Use the existing semantic CSS variables so backgrounds, text, borders, and interactive states change together with system preference.

## Typography

**Body and heading font:** locally served Geist Variable, with sans-serif fallback. There is no separate display face.

Body controls disable synthetic font styles (`font-synthesis: none`) and use antialiased rendering. Selection uses primary foreground/background; input carets use primary. Links have a (4px) underline offset.

- **Headline:** page headings use the frontmatter headline role, balanced wrapping, and (12px) bottom space; a heading inside a heading row removes that bottom margin.
- **Title:** section headings are compact and semibold, with (8px) icon/text gaps.
- **Body:** the root baseline is the body role; visible task prose uses paragraph, bounded to (65ch) for introductions and (70ch) for section descriptions.
- **Control:** buttons and navigation use compact (14px) labels. Inputs use (16px) below the desktop breakpoint and (14px) on desktop.
- **Label:** menu headings and badges use (12px) text. Secondary metadata is (13px); the sidebar footer caption is (11px).

**The Shared Type Rule.** Use Geist for headings, prose, and controls; establish hierarchy with size, weight, and spacing.

## Layout

Desktop starts at (768px). The fixed left sidebar is (14.5rem / 232px) wide and collapses off canvas. Use the shadcn inset variant with its inset content frame; the application shell has no header or context bar. Content is centered, capped at (1120px), and padded (40px 48px 64px). Forms cap at (560px), with (16px) gaps and (24px) initial spacing; settings sections begin (32px) apart.

Below (768px), navigation becomes an (18rem) Sheet drawer and content padding becomes (28px 20px 48px). A (44px) navigation trigger sits inside the content above the page, with (12px) bottom space. Navigation, menu items, and sub-triggers have (44px) minimum height. At (600px) and below, buttons, inputs, input groups, and native selects also receive (44px) minimum height; people and invitation rows stack.

Authentication retains its brand and system-status header and uses the sidebar surface as its full-height canvas, with a centered card capped at (448px), (16px) card spacing, and (48px) vertical main padding. Its outer padding is (24px 32px), becoming (20px) on mobile. Preserve shrinkable content, wrapping action groups, and truncated navigation names.

## Elevation & Depth

Depth comes primarily from surface changes and fine borders. Cards use a (1px) foreground ring at (10%) opacity rather than a drop shadow. Root menus use the standard medium shadow; nested menus use the large shadow, both with the same subtle foreground ring. The mobile drawer uses the existing shadcn Sheet elevation and overlay. Exact shadows and transition timings are recorded in the sidecar.

**The Flat Task Surface Rule.** Keep task content and sidebar flat; reserve elevation for menus and the mobile drawer.

## Shapes

Controls use the base small corner (`rounded.lg`); navigation and menu rows use the tighter `rounded.md`. Cards use `rounded.xl`. Badges use `rounded.4xl`, while workspace avatars are circular. Workspace choice initials use a (4px) corner. List rows stay square with bottom dividers; avoid converting each administrative row into a card.

## Components

Components own their Tailwind utilities. Reuse `PageTitle`, `SectionTitle`, `Intro`, the settings layout and table components, and `DirectoryList` for shared patterns. Control variants own their responsive hit targets and reduced-motion behavior. Keep `styles.css` limited to imports, theme tokens, and browser-wide defaults; do not add screen classes or component overrides there.

### Buttons

Compact, clear actions. The default height is (32px), with (10px) horizontal padding, (6px) gaps, and medium control text. Primary uses indigo; outline uses a field-like border; secondary uses a neutral fill; ghost reveals a quiet hover fill; link is underlined on hover; destructive uses red text over a translucent red surface. Dark outline controls use the input surface at (30%) opacity. Hover and expanded states follow their existing semantic variants. Keyboard focus uses a (3px) half-opacity ring; disabled controls use half opacity. Pressed buttons shift down (1px), except popup triggers.

### Inputs / Fields

A small-corner, full-width field with a (1px) input stroke, (4px 10px) padding, and muted placeholder. It is transparent in light mode and uses a translucent input surface in dark mode. Focus changes the border to ring and adds the (3px) ring; invalid fields use destructive treatments. Preserve native types and validation, disabled styling, and the responsive font and minimum heights.

Workspace creation places Workspace URL after Workspace name. Use one shadcn input group with the fixed current-host prefix (`<domain>/`) in muted text and an editable slug, producing `<domain>/<slug>`. Cap the prefix at half the available width and truncate it with its full value in a title so the editable suffix stays usable on mobile. Suggest a URL-safe slug from the name until the user edits it; retain manual edits after name changes and submission errors. Preserve native format and length validation, linked reserved-path feedback, and the form's duplicate-URL error. The URL stays stable when the workspace name changes.

### Chips

Compact badges with (20px) height, fine corners approaching a pill, and (2px 8px) padding. Outline badges identify workspace roles and team keys; neutral secondary badges show counts. Keep labels brief.

### Cards / Containers

Authentication cards use the card surface, restrained foreground ring, and the rendered (16px) spacing. Generic cards retain the existing (16px) default spacing; smaller cards use (12px). Administrative content uses open sections, bordered rows, and dividers instead of a repeating card grid.

### Navigation

The main sidebar contains team navigation. Workspace switching, Create a workspace, and settings access live in the top workspace dropdown. The dedicated settings sidebar provides section navigation while settings are open.

Under **Your teams**, each team has a collapsible group with indented **Home** and **Issues** links. Use real team names and immutable keys. Team Home shows its workflow; Issues shows only that team's work, with the team name in the breadcrumb. Preserve the selected team through active/archived navigation, direct issue links, and creation defaults. These groups follow the user's sidebar reference, using flat submenus without connecting borders.

Team destinations use `/<workspace-slug>/team/<lowercase-team-key>`: Home at the root, active issues at `/all`, archived issues at `/archive`, and creation at `/new`. Issue details use `/<workspace-slug>/issue/<issue-key>/<issue-title-slug>`; key-only and stale-title URLs redirect to the current title. The identifier drives identity and sidebar selection; the title is a readable URL suffix. Title edits update the URL without resetting the editor or its drafts. Back to issues returns to the owning team's list.

Use the shadcn Sidebar inset variant: (32px) default rows, tight corners, (8px) padding and icon gaps, and (16px) SVG icons. Hover and active rows use sidebar accent; active labels gain medium weight. Keyboard focus uses a (2px) sidebar ring. The content-level mobile trigger and Cmd/Ctrl+B toggle the sidebar; mobile navigation closes after selecting a destination and has a close trigger in its header. Workspace destinations use the slug path (`/<slug>`). Preserve the skip-to-content link and current-page semantics.

### Workspace Menu

The sidebar header shows a circular initials avatar, current workspace name, and caret. The root dropdown is (240px) wide and offset (8px) from its trigger. Desktop workspace switching opens a nested (240px) menu, with a checkmark on the current workspace. Mobile replaces the root menu contents with workspace choices and a Back action. Settings, membership actions, and workspace creation stay conditioned on the implemented permissions.

Sidebar transitions use (200ms); menus use (100ms); workspace row hover uses (150ms) ease-out. Theme changes suppress transitions. Reduced motion disables the shell's sidebar, menu, drawer, button, row, spinner, and skeleton motion.

## Do's and Don'ts

### Do:

- **Do** use semantic CSS variables for both system themes.
- **Do** keep the current workspace at the top of the sidebar and show the current choice in the switcher.
- **Do** preserve keyboard focus, form validation, pending states, and disabled controls.
- **Do** expand mobile navigation and menu rows to the implemented touch targets.
- **Do** honor reduced motion for sidebar, menu, drawer, and loading transitions.

### Don't:

- **Don't** replace the shadcn Sidebar and menu behavior with custom static imitations.
- **Don't** add decorative raster assets to this code-led shell.
- **Don't** invent issue features or navigation destinations beyond implemented functionality.

## Issues (CP04)

The user's issue-list and create-dialog reference extends the compact workspace system. Issue lists use a full-width surface, a small workspace/Issues breadcrumb, All issues and Archived views, and collapsible status groups. Rows prioritize the human identifier and title, with quiet priority/status icons, assignee initials, and creation dates. Mobile rows wrap titles and omit dates. Only implemented properties appear.

Creation is a focused shadcn Dialog, approximately 750px wide on desktop and inset 16px on mobile. The team and New issue heading sit above a borderless title and Markdown source editor. Status, priority, and assignee use compact rounded dropdown controls. The footer supports Create more and the primary create action. Opening focuses the title; outside clicks preserve a nonempty draft, and failed requests keep all inputs. Existing semantic colors supply system light/dark themes. Issue details save edits automatically and retain explicit conflict-review controls.

Issue details use an in-place editor following the supplied Linear reference: a compact team/Issues/key breadcrumb, the issue title and rendered description in the main column, and status, priority, and assignee in a quiet properties column. Mobile stacks properties below the description. The shared `Prose` React component styles Markdown headings, lists, task checkboxes, links, tables, and code with Tailwind utilities and existing theme tokens. It uses 15px desktop body text, 1.65 leading, and 1.25em flow with a maximum measure of 75ch. Its descendant utilities cover the HTML generated by Markdown and ProseMirror; app headings use explicit React components so document hierarchy remains intact.

The description is a borderless ProseMirror editor through Tiptap, with Markdown storage, task checkboxes, tables, and keyboard formatting. Selecting text reveals a compact shadcn ButtonGroup toolbar centered above the first selected line, with viewport clamping and wrapping on mobile. Group text styles, inline marks and links, and quote/list/clear actions; active marks use the built-in secondary button variant. Preserve the selection across menus and link entry. Alt+F10 focuses the toolbar, arrow keys navigate its buttons, and Escape returns to the editor. The global sidebar shortcut defers to editable fields and handled events. Title and description changes save after an 800ms pause or on blur; property menus save on selection. Writes are serialized and preserve edits made while a request is pending. Failures retain drafts with Retry save and Discard changes actions; conflicts pause saving until the current version is adopted or changed fields are reviewed and explicitly saved. Navigation flushes pending edits and guards unresolved drafts. Archived issues are read-only until restored. Markdown is parsed into the supported document schema and unsafe URL protocols are filtered; HTML is never inserted directly into the page.
