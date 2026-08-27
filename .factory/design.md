# Visual thesis — The marked-up desk

## Direction and rationale

Rubric Comment Queue uses a **neo-brutalist utility** system that feels like the
best parts of a teacher's physical marking desk: cream paper, blue rubric tabs,
orange editing pencil, black ink, clipped notes, and unambiguous red correction
marks. Hard 2px borders and offset shadows make controls feel like movable paper
tools rather than a generic SaaS dashboard. Decoration is sparse and explains
the core promise: reusable teacher-authored comments flow through a review queue
and leave as personal feedback. The interface never suggests that a machine is
grading a student.

## Tokens

### Light treatment

- `paper` `#F4F0E6`: warm desk/paper background
- `sheet` `#FFFCF5`: primary reading surface
- `ink` `#151515`: text and hard edges
- `muted-ink` `#555149`: supporting copy (7.1:1 on paper)
- `cobalt` `#2447D8`: selected rubric tab and links
- `cobalt-dark` `#17309B`: hover state
- `orange` `#F05A28`: primary action/pencil mark
- `orange-dark` `#A8320F`: accessible orange text
- `mint` `#C9F1D8`: completed/reviewed surface
- `amber` `#FFD66B`: needs-review surface
- `danger` `#B42318`: destructive text and error edge

### Dark treatment

The app supports a deliberate ink-at-night theme, selected from the header:
background `#171713`, surface `#24231F`, text `#FAF6EA`, muted `#C5C0B4`, cobalt
`#8DA2FF`, orange `#FF8B62`, mint `#284B38`, amber `#5B4719`, danger `#FF8A80`.
Borders remain high-contrast and shadows become pale offset rules.

## Typography

- Display and labels: `Arial Black`, `Arial`, sans-serif. Its compressed visual
  authority resembles a stamped folder label and keeps calls to action blunt.
- Reading and form content: `Georgia`, `Times New Roman`, serif. Student excerpts
  and comments read like edited prose, not app chrome.
- No font files or runtime font requests. Body is 17px with 1.55 leading;
  metadata is never smaller than 13px. Tabular figures are used for counts.

## Spacing and layout

All spacing follows a 4px base: 4, 8, 12, 16, 24, 32, 48, 64. Desktop uses a
250px queue rail and a flexible 760px review sheet, with an optional 300px
comment-bank panel. At 900px the bank moves below the editor. At 680px all
regions stack; secondary metadata is shortened, but actions and progress stay
visible. Controls are at least 44px tall and separated by at least 8px.

## Interaction grammar

- Items enter the queue as squared paper slips. The current slip carries a
  cobalt top edge and a short `Now` tag; completed slips turn mint and retain a
  visible check label so color is never the only signal.
- Comment blocks behave as reusable stamps: choosing one visibly presses it
  into the editable draft, but the teacher always edits before marking ready.
- The main forward motion is `Save & next`; it advances one paper slip and
  announces the state change. `Back` never loses work.
- Destructive deletion requires a named confirmation. Clearing the workspace
  offers a five-second undo.

## Motion policy

Transitions last 160–220ms and use only transform/opacity: a newly active queue
slip shifts 3px toward the page; confirmation ink appears with a single short
scale. There is no looping motion. Under `prefers-reduced-motion: reduce`, all
movement and smooth scrolling are removed, while color, labels, and focus
continue to communicate state.

## Asset plan and provenance

One original hero illustration is used only in the first-run empty state to
explain the workflow. It depicts an abstract desk with three paper excerpts
passing through teacher-controlled stamps into a neat feedback stack. It must
contain no people, handwriting, readable text, brands, logos, or AI imagery.

**Prompt sheet:** "Editorial neo-brutalist still life, top-down teacher marking
desk, three cream paper slips moving from an untidy left stack through a bold
blue rubric tab and an orange wooden feedback stamp toward a neat mint-green
right stack, black ink outlines, screen-printed paper grain, slightly imperfect
registration, strong geometric shadows, limited palette warm cream cobalt blue
burnt orange mint black, tactile cut-paper collage, wide horizontal composition,
no people, no hands, no faces, no readable text, no letters, no watermark, no
logos, no gradients, no photorealism, no laptop, no robots."

Generated with the factory image deployment (Azure OpenAI image generation) on
2026-08-27. The selected output and prompt sidecar live in `assets/src/`;
optimized WebP/AVIF derivatives live in `public/`. Generated specifically for
this product and treated as an original project asset. Footer disclosure:
"Illustration generated for Rubric Comment Queue."
