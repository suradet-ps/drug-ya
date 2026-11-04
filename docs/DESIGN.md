# Drug-Ya - Design System & Product Design

Drug-Ya is a **medication-order review console for hospital pharmacists**. It
reads today's OPD orders and the patient context needed to judge them, then
shows explainable findings. This document covers product scope, UX flows, the
visual system, components, and interaction states.

---

## 1. Product scope

### Who uses it

Pharmacists at the dispensing window / medication review desk of a Thai
hospital using HOSxP. They know the drugs; they need the patient's context and
a fast, trustworthy second pass over today's orders.

### What it is

- A read-only view of **one patient's today OPD orders**, across all of today's
  visits.
- A context board: allergies, labs, renal function, diagnoses, pregnancy, PMH.
- A deterministic rule-based review with source and evidence on every finding.
- A configurable rule pack that a site pharmacist can tune and sign off.
- A session-local acknowledgement workflow.

### What it is not

- Not a prescribing system; it never writes to HOSxP.
- Not a complete interaction database or a validated medical device.
- Not a full medication reconciliation; it does not aggregate prior visits or
  external hospitals.
- Not a patient-facing app.
- Not a system that stores patient data on disk in v1.

### Core promises

1. **Do not hide uncertainty.** Every missing lab, diagnosis, weight, sig, or
   drug mapping appears in the coverage report.
2. **Every finding is explainable.** The order lines, patient facts, mechanism,
   recommendation, and source are visible.
3. **The pharmacist decides.** The app suggests; the pharmacist reviews,
   contacts the prescriber, and confirms.
4. **Read-only, always.** The visual language never suggests that a click has
   changed a HOSxP order.

---

## 2. UX flows

### 2.1 First launch → connection → sign-in

1. The app opens on the **HOSxP officer login gate**. No patient UI exists
   behind it.
2. If no connection is saved, a compact notice in the login card says so and
   offers **ตั้งค่า**; the toolbar also keeps **ตั้งค่า** reachable. Both open
   the settings modal on the **การเชื่อมต่อ** tab.
3. The operator enters Host, Port, Database, User, Password, tests, and saves.
   The backend connects before persisting; credentials are encrypted and the
   key lives in the OS keychain.
4. Sign in with the HOSxP `officer` credentials. The session is memory-only.
   Five failed attempts trigger a growing lockout.
5. On success, the review console opens with the search rail focused.

### 2.2 Search

One search box, with an explicit mode selector for ambiguous input:

| Input | Detection |
|---|---|
| 13 digits | CID |
| 5-10 digits | HN |
| 1-4 digits | today's queue |
| `HN...` | HN |
| `Q...`, `QH...`, `คิว...` | today's queue |
| otherwise | name |

- Debounce 320 ms; show a skeleton after the first loading state.
- Search results show name, HN, age, sex, masked CID. Clicking a result loads
  the review. The search field is focused on entry.
- Keyboard: Up/Down moves through results, Enter opens the active result,
  Escape clears the query. The active row uses the same blue outline as the
  selected row.
- An empty query shows a quiet rail hint; the empty workspace carries the
  four search examples and the rule-pack link.
- Queue search is a first-class path: a pharmacist can type `Q12` at the
  window, or choose **คิว** and type `12`.

### 2.3 Review screen

Top to bottom:

1. **Patient header** - one card: avatar initials, name, age, sex, HN, CID,
   birth date, today's visit chips (queue, department, time), rule-pack
   sign-off link, last checked time, refresh.
2. **Data warning line** - only when a section degraded; one collapsed row
   with a count; the Thai warnings expand in place.
3. **Clinical summary** - one five-stat card: kidney (eGFR / CrCl with stage),
   creatinine, potassium, INR, weight/BMI. Each stat has a tone (ok / warn /
   critical / unknown) and a date or missing-data line.
4. **Context bar** - one quiet row: allergies (agent, symptom, force-no-order
   flag, or an explicit "read failed" state), diagnoses (ICD-10 plus local
   label, first four with a +N chip), pregnancy selector when applicable, and
   PMH behind a disclosure.
5. **Review layout**:
   - left: today's orders grouped by visit, with a queue badge, department,
     prescriber, time, and table rows;
   - right: findings, compact counts, filter tabs, coverage, acknowledgement,
     note.
6. **Disclaimer footer** - scope and "not a do-not-dispense order".

### 2.4 Findings

The right panel is the emotional centre of the screen, but it stays quiet.

- One **action chip** next to the panel title summarises the most urgent state:
  `ติดต่อแพทย์`, `ทบทวนก่อนจ่าย`, `เฝ้าระวัง`, `ตรวจซ้ำ high-alert`, or
  `ไม่พบข้อท้วงติง`.
- A thin count row uses dots, not pills: วิกฤต / สูง / เฝ้าระวัง / ต่ำ / ข้อมูล.
- Filter tabs: ทั้งหมด / ต้องจัดการ / เฝ้าระวัง / ข้อมูล.
- Each finding card shows:
  - category icon and label;
  - severity badge;
  - title, detail;
  - patient-specific factor chips (e.g. `CrCl 24 mL/min (เกณฑ์ <30)`);
  - involved drug chips;
  - a recommendation box;
  - evidence and source behind a disclosure;
  - an acknowledgement button.
- Findings that `requires_ack` must be acknowledged before the review can be
  confirmed.
- The review note and confirmation are **session-only**. The UI says so.
- If no findings exist, an all-clear state still shows coverage counts and the
  caveat.

### 2.5 Coverage

The coverage disclosure lists every category with a state:

- **ตรวจแล้ว** - prerequisites were available.
- **ตรวจได้บางส่วน** - some data or drug mappings were missing.
- **ยังไม่ได้ตรวจ** - the check could not run; the reason is shown.

It also lists:

- missing data (creatinine, potassium, INR, weight, diagnoses, pregnancy,
  dose parsing);
- unresolved drug names;
- known/total orders;
- checked/applied rule counts;
- notes (today's OPD only; rule pack unsigned; min severity filter).

This is the antidote to a false "all clear".

### 2.6 Settings

Three tabs:

1. **การเชื่อมต่อ** - Host/Port/Database/User/Password, test, save. Available
   before sign-in so first-run setup works.
2. **กฎการตรวจ** - global options (site label, elderly age, renal metric,
   minimum severity, pregnancy/pediatric/high-alert toggles), category toggles,
   searchable rule list with per-rule enabled switch, severity select, and
   threshold input. Save/reset.
3. **การรับรอง** - pack metadata, disclaimer, current sign-off, and a sign-off
   form (name, licence, note, confirmation checkbox).

Changes are saved as plain JSON and apply to the next review. If a review is
open, saving reloads it.

### 2.7 Sign-out

Logout clears the backend session and every patient-scoped signal (search
results, review, clinical context, acknowledgement, note, pregnancy override).
No patient data survives the session.

---

## 3. Visual design system

### 3.1 Brand

The supplied mark is a dark rounded square with a white/olive capsule and sage
corner brackets. The **outer drop shadow has been removed** by request. The
in-app mark keeps its olive identity; it is the only filled mark in the UI and
receives no additional outer shadow.

Brand personality: a confident clinical instrument, not a consumer wellness
app. The shell follows a Webflow-inspired system:

- a generous **white canvas** with **near-black `#080808`** for primary CTAs,
  headings, ink, and the search rail;
- one **five-stop accent palette** used sparingly as surface fills: purple
  `#7A3DFF`, pink `#ED52CB`, blue `#3B89FF`, orange `#FF6B00`, green `#00D722`;
- a weight ceiling of **600**, tight tracking on display type, and uppercase
  eyebrows with positive tracking;
- **4 px** buttons and inputs, **8 px** cards and panels. No pill CTAs.
- Severity stays semantic: red `#EE1D36`, orange `#FF6B00`, yellow `#FFAE13`,
  green `#008F18`, info blue `#146EF5`.

The chromatic accents appear only on the 22 px finding-category icon tiles,
the five-stop accent rule on the login and empty states, and the empty-state
mark. They are never button backgrounds.

### 3.2 Tokens

| Token | Value | Role |
|---|---|---|
| `--wf-ink` / `--ink` | `#080808` | primary text, headings, primary CTA |
| `--wf-body` / `--slate` | `#363636` | body text |
| `--wf-body-mid` / `--muted` | `#5A5A5A` | secondary text |
| `--wf-mute-soft` / `--muted-2` | `#898989` | meta, placeholders |
| `--wf-hairline` / `--hairline` | `#D8D8D8` | 1 px borders |
| `--bone` / `--paper` | `#FFFFFF` | canvas and cards |
| `--cream` | `#F7F7F7` | quiet fills, selected tabs |
| `--accent-purple` | `#7A3DFF` | category tile |
| `--accent-pink` | `#ED52CB` | category tile |
| `--accent-blue` | `#3B89FF` | category tile, focus ring on black |
| `--accent-orange` | `#FF6B00` | category tile |
| `--accent-green` | `#00D722` | category tile |
| `--accent-yellow` | `#FFAE13` | warning tile, unsigned pack |
| `--danger` | `#EE1D36` | critical |
| `--high` | `#FF6B00` | high |
| `--warn` | `#9A6A00` | moderate / warnings |
| `--ok` | `#008F18` | stable / checked |
| `--info` | `#146EF5` | info findings |

Severity is carried by **both** color and text/icon; color is never the only
signal.

Radii: 4 px buttons/inputs/badges, 8 px cards/panels/modal, full only for
circular icon buttons. Elevation: a hairline plus one soft panel shadow; the
layered drop-shadow recipe is reserved for the login card and the modal.

Typography:

- Latin and UI: **Inter** (bundled Latin WOFF2, weights 400-600), with
  **Sarabun** (bundled Thai WOFF2) as the Thai fallback. Both fall back to
  Leelawadee UI / Segoe UI.
- Codes, numbers, HN/CID: **Inconsolata** / IBM Plex Mono / Cascadia Mono.
- Base 14 px, line-height 1.55. Weight ceiling 600.
- Display: login 34-42 px, empty-state 36-48 px, patient name 24 px, panel
  title 18 px, finding title 14 px.
- Thai headlines use slightly positive line-height and near-zero tracking;
  the Webflow-style `-0.8 px` display tracking applies to Latin-only surfaces.
- Eyebrows: 12 px, weight 500, `1.5 px` tracking, uppercase.

### 3.3 Layout

```
┌──────────────────────────── top bar (56px) ─────────────────────────────┐
│ title · date                    status · officer · settings · logout   │
├──────────────────────┬──────────────────────────────────────────────────┤
│ search rail (360px)  │ workspace (scroll)                               │
│  black feature band  │  patient header                                  │
│  brand               │  warning line                                    │
│  search + mode       │  clinical summary (5 stats)                      │
│  results             │  context bar (allergy / dx / pregnancy / PMH)    │
│  footer              │  ┌──────────────────────┬────────────────────┐  │
│                      │  │ orders by visit      │ findings + coverage│  │
│                      │  └──────────────────────┴────────────────────┘  │
│                      │  disclaimer                                      │
└──────────────────────┴──────────────────────────────────────────────────┘
```

- Search rail scrolls independently; workspace scrolls independently.
- The findings panel flows with the page (no nested scroll); long reviews use
  the workspace scrollbar.
- Breakpoints: 1280 px (narrower rail/findings), 1180 px (stack the review
  columns), 1080 px (narrower split login), 960 px (login becomes one column),
  900 px (3-column clinical summary), 760 px (compact top bar, single-column
  settings).

### 3.4 Components

- **Login** - black brand band with the mark, a five-stop accent rule, a
  short two-line headline, one decorative review-card preview, and a privacy
  footnote. A light-grey panel carries one white auth card. Connection status
  sits in the toolbar only when a connection is saved; an unconfigured site
  sees a compact notice in the card instead. No marketing paragraph, feature
  chips, or version footer.
- **Search rail** - the single black feature band: brand lockup, dark search
  field, mode select, result cards, instructions, read-only footer.
- **Patient header** - white card, 4 px avatar tile, 24 px name, mono
  identifiers, visit chips, unsigned-pack link, checked-at and refresh.
- **Clinical summary** - one white card with five equal stats. 19 px mono
  value, uppercase label, date or stage below. Missing data is a dash plus a
  reason, never a fabricated value.
- **Context bar** - one row of 4 px chips: allergy chips (danger fill when
  `force_no_order`), ICD-10 chips, pregnancy selector, PMH disclosure.
- **Order table** - compact 5-column table: drug/strength, directions for use,
  quantity, estimated days, finding severity. The directions cell shows the
  complete joined HOSxP sig (`drugusage` + `sp_use` names) as source text,
  including timing and meal instructions. Text wraps rather than truncating.
  No generated shorthand replaces the prescriber's wording. Visit grouping is
  a quiet grey header with a black queue badge.
  A severity color appears as a 3 px left border, not a full-row wash.
- **Finding card** - white card with a 3 px severity bar; a 22 px chromatic
  category icon tile (purple/pink/blue/orange/green/yellow), category label,
  severity badge, title, detail, factor/drug chips, grey recommendation box,
  evidence behind a disclosure, acknowledgement button.
- **Findings header** - title plus one action chip and a dot count row; no
  separate risk hero.
- **Coverage disclosure** - quiet card with category rows; checked green,
  partial amber, skipped grey.
- **Settings modal** - 1020 px max, dark blurred backdrop, header, tabs, scroll
  body, fixed footer. Rule rows are dense but readable: switch, title,
  id/source, severity select, threshold input.
- **Buttons** - black primary fill, white hairline quiet button, green-tinted
  acknowledged state. 4 px radius. Focus ring 2 px ink plus a 3 px soft halo.
- **Icons** - 1.8 px line icons; the logo is the only filled mark.

### 3.5 Content tone

- Thai-first. Avoid English-only clinical instructions.
- Use "ต้องทบทวน", "ควรเฝ้าระวัง", "ติดต่อแพทย์" rather than "ห้ามจ่าย" except
  for genuinely contraindicated findings.
- Every all-clear says what was and was not covered.
- Never say "ปลอดภัย" (safe); say "ไม่พบข้อท้วงติงจากกฎที่เปิดใช้".

---

## 4. Interaction states

| State | Presentation |
|---|---|
| Startup | dark splash with the mark, a small progress bar; window hidden until mounted |
| Signed out | login gate; username focused; connection settings reachable |
| Login validation | inline error, focus moves to the empty field; error clears on typing |
| Password field | reveal/hide toggle; Caps Lock warning while the field has focus |
| Search debounce | no spinner for <320 ms; skeleton only when loading |
| Search empty | quiet rail hint; the workspace shows examples |
| Search no results | quiet empty row; hint to change mode |
| Search error | red rail panel with the Thai message |
| Review loading | skeleton header/strip/two columns |
| Review empty (no orders) | orders panel empty state; clinical context still shown |
| Review error | centered error card with retry |
| Findings empty | all-clear card + coverage + caveat |
| Finding acknowledged | card dimmed; button becomes "รับทราบแล้ว" |
| Confirmed | green banner with timestamp; button disabled; note remains |
| Health lost | the status pill turns red and keeps the "HOSxP" label; the tooltip states the failure |
| Settings load | spinner + "กำลังโหลดการตั้งค่า…" |
| Settings open | focus moves into the dialog; Tab is trapped inside |
| Settings dismiss | Escape, the close button, or the backdrop closes the dialog |
| Rule save | button "กำลังบันทึก…"; review reloads on success |
| Unsigned pack | amber chip in patient header + note in coverage |
| Missing data | amber warning banner + coverage rows |

### Keyboard / accessibility

- Focus-visible rings on all interactive elements.
- Buttons are real `<button>`s; the search results are buttons, not divs.
- Inputs have labels via visible text; icons are `aria-hidden`.
- Filter tabs use `role="tab"` and `aria-selected`; the list is a tab panel.
- Modal has `role="dialog"`, `aria-modal`, a labelled title, initial focus,
  Escape-to-close, and a keyboard focus trap.
- Search has an explicit label, keyboard navigation, and a polite result count.
- Status changes (connection, search count, confirmation, Caps Lock) are
  announced with `role="status"` or `aria-live="polite"`.
- Color contrast: critical/danger and primary button text are chosen for AA on
  their backgrounds; meta text uses `--muted`, not pure grey.
- `prefers-reduced-motion` disables transitions and splash animations.
- The UI is Thai-first; long drug names wrap rather than truncate in findings.
  Table cells may truncate visually, but the full name is visible in the
  finding card.

---

## 5. Data presentation rules

1. **Never invent a value.** If creatinine is missing, show `—` and a reason.
2. **Show the date** on every lab value.
3. **Show both renal numbers** when available: eGFR (CKD-EPI 2021) and CrCl
   (Cockcroft-Gault), with the source. A rule uses the site's chosen metric.
4. **Pregnancy is never assumed negative.** Unknown stays unknown and is shown
   as a gap; the pharmacist can confirm pregnant/not pregnant in the session.
5. **Unknown drugs stay visible.** A "ยังไม่รู้จัก" badge and coverage entry.
6. **Findings are ordered by severity**, then category, then title.
7. **The rule pack version is always visible** in the patient header and
   settings; unsigned state is amber.
8. **Acknowledged is not the same as resolved.** The UI marks review, not
   clinical resolution. No order state changes are implied.

---

## 6. Why this looks the way it does

- The **left black rail** is the one polarity-flipped feature band. It keeps
  patient search separate from the clinical canvas and matches the near-black
  primary used for every CTA.
- The **white canvas** and hairline cards make the clinical content the
  interface; there is no decorative colour wash competing with findings.
- The **five-stop accent palette** is deliberately constrained to category
  icon tiles and the accent rule. It gives findings a fast visual index
  without turning severity into a rainbow.
- Severity colors stay semantic and are carried by text as well as color; the
  UI is used for long shifts, and a wall of red destroys triage.
- The **display weight ceiling of 600** keeps headings confident without
  shouting, and the uppercase eyebrows create section rhythm without extra
  copy.
- The **coverage disclosure is below the findings**, not hidden in settings,
  because trust depends on knowing what was not checked.
- The **rule sign-off state** sits in the patient header because an unsigned
  pack changes how much the findings should be trusted.
