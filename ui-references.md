# Landing page references

Reviewed on 2026-09-14. These are inspiration notes, not an approved final design or stack selection.

## Project direction

- Confirmed user choice (2026-09-15): use the px0.ai footer as the footer design reference. Match its layout and visual treatment with this project's own branding, links, and content. Reinspect the current footer when implementing; this choice is fixed even while the rest of the page direction remains open.

- A landing page for a Redis implementation with extra features, written from scratch in Rust.
- Show commands, interactive demonstrations, measured benchmarks, architecture, and practical usefulness.
- No dashboard or separate HTTP API requested. Browser simulations should be labelled; benchmark claims must come from actual measurements.
- Keep frontend work separate from the Rust implementation and minimize changes to existing server code.
- The user is still collecting references. Wait for the remaining references before choosing the final visual direction.

## px0

- Website: https://px0.ai/
- Product repository: https://github.com/px0-ai/px0
- Role: developer-product storytelling, section structure, command examples, benchmark presentation, and interactive demos.
- Landing page assets show Astro, custom scoped CSS, JetBrains Mono, dark/light themes, amber accents, thin borders, and square corners.
- Inspected scripts use native JavaScript, IntersectionObserver, CSS transitions, theme persistence, copy buttons, and scroll progress.
- The hero demo uses embedded JSON and client-side interactions. Do not assume it connects to a live IDE server.
- The repository's `web/` directory is the IDE frontend: HTML, custom CSS, JavaScript modules, and theme stylesheets. It is distinct from the Astro landing page; the landing source/package manifest was not found in that repository.

## Sift

- Website: https://sift1.vercel.app/
- Role: typography, spacing, palette restraint, and an immediately understandable interactive demonstration.
- Reviewed the rendered desktop page at 1440 x 1100 and inspected its public HTML/CSS. Extraction was not submitted; live success/error behavior was not tested.
- Warm off-white background, near-black text, lime/acid accent, thin rules, and square panels.
- Minimal header: small wordmark at left, small status indicator at right, with a horizontal divider.
- Large left-aligned two-line hero. First line is solid heavy text; second line uses outlined text via `-webkit-text-stroke`. Tight tracking and line height give it a print-like character.
- A narrow right-side note balances the headline. Supporting copy is short with generous surrounding space.
- Beneath the hero, a dark input panel sits beside a wider white output panel. Strong contrast makes the action and its result easy to follow.
- Small monospace labels, a lime primary button, restrained empty-state copy, and a minimal footer.
- Public assets indicate Next.js/React and Tailwind-generated styles alongside custom CSS. Geist fonts are loaded, but inspected custom body styling uses Arial/Helvetica and labels use system monospace; do not assume Geist is the visible headline font.
- CSS collapses the hero and working area at a 760px breakpoint. Mobile rendering was not visually checked.

## Possible synthesis, pending remaining references

Use px0 for the information structure and technical evidence. Use Sift for editorial typography, whitespace, high contrast, and a focused demo area. A command/request trace beside an admission result could explain the feature without adopting a dashboard layout. Color, font, and implementation stack remain undecided.

## ObsidianUI component shortlist

Reviewed documentation and catalog on 2026-09-15: https://www.obsidianui.dev/

- React/TypeScript components using Tailwind CSS and Motion. Setup documentation targets Next.js.
- Copy/customize workflow with a shadcn-compatible registry at `https://www.obsidianui.dev/r/{name}.json`.
- Repository license verified as MIT: https://github.com/Atharvsinh-codez/ObsidianUI/blob/main/LICENSE. Preserve the required license/copyright notice when incorporating source.
- Potential candidates: Arrow Fill Button for the main CTA, Magnet Tabs for workload or command examples, Rectangular Text Reveal for a restrained headline entrance.
- Treat it as a source of selected interactions, not a replacement for the px0/Sift page direction. Elaborate cursor trails, WebGL glass, and gallery effects are lower priority for this developer landing page.
- Component-level source/dependency review and rendered interaction testing remain pending; registry JSON and several individual docs pages were unavailable through the browsing tool. No packages installed and no final stack selected.
