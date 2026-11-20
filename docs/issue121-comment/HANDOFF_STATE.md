# Handoff: cyme#121 typec PR, post-publication state

Read this before touching anything related to this issue/PR again. Written 2026-07-23, end of
the session that opened the real PR. **This file lives in `docs/issue121-comment/`, which is
deliberately UNTRACKED (never `git add`ed) -- do not move it to the repo root and do not stage
it.** That's exactly the mistake made earlier this session with `HANDOFF_TYPEC.md`: it got
committed at repo root and shipped as noise inside the actual PR diff shown to the maintainer,
had to be removed with a follow-up commit. Keep every local-only planning/handoff doc inside
this directory, never at repo root, never staged.

## Where things stand

- **PR #124, OPEN, DRAFT**: https://github.com/tuna-f1sh/cyme/pull/124
  From fork `soy-chrislo/cyme`, branch `typec-ports-prototype`, against `tuna-f1sh/cyme` `main`.
- **Issue comment posted**, points at the PR without repeating its content:
  https://github.com/tuna-f1sh/cyme/issues/121#issuecomment-5063955613
- **CI validated green** in the fork before opening the PR, via manual `workflow_dispatch`
  (not a PR-triggered run, since `build.yml` only triggers on push-to-`main`/`pull_request`):
  https://github.com/soy-chrislo/cyme/actions/runs/30047085490 -- all 4 platforms
  (windows-gnu, macos-universal, linux-arm64, linux-x86_64) plus `cargo fmt --check`, all
  success.
- **Latest commit on the branch**: `2a8350d` (removed `HANDOFF_TYPEC.md` from the PR --
  internal notes, shouldn't ship to a third-party reviewer). Full commit history on this
  branch: `b47a83f` (typec module) → `4453cde` (review fixes) → `4d03255` (SystemProfile
  wiring) → `787cceb` (security fix: capped `read_attr`) → `c5f4bdb` (design diagrams added)
  → `2a8350d` (removed internal handoff doc).
- Local `main` and upstream `tuna-f1sh/cyme` `main` are at the same commit
  (`a00589c108059b2b491dcfb1f1b94c827a043c0c`) as of this session -- re-check this before
  citing any permalink against that SHA in future writing, it may have moved since.

## What the PR actually says (in case this file outlives the PR body itself)

Adds `SystemProfile.typec_ports: Option<Vec<TypecPort>>`, reachable via `--json` only (no
tree/table display yet, no watch-mode re-enumeration, no real ACPI hardware validation --
all explicitly out of scope, documented as such in the PR body, not oversights). Design
question tuna-f1sh raised (link `Cable`/`Port` back to `Device` via `DeviceLocation`, `Path`,
or `Bus`) resolved by: top-level section (not nested in `Bus`), not extending
`DeviceLocation` (macOS-specific string format), reusing `PortPath` (not extending `Path`
itself) purely as an opportunistic correlation key via `TypecPort::device_links`. Full
reasoning + 2 diagrams live in the PR description itself -- don't re-derive it, read
https://github.com/tuna-f1sh/cyme/pull/124 directly, it's the source of truth now, more
current than any local draft file.

## What's actually being waited on

Two open questions posted in the PR, waiting on a human to answer either:
1. Whether the `Bus`/`DeviceLocation`/`Path` reasoning matches what tuna-f1sh intended when he
   raised those three options, or if there's a case they were meant to cover that this design
   misses.
2. An ACPI-based (not Device-Tree/UCSI) Type-C laptop dump, to validate the one correlation
   path (`device_links` actually resolving to a real `Device`) that no hardware seen in this
   thread so far can exercise.

**Nothing is actionable on our side until one of those gets a response.** Don't ping again,
don't add more design analysis preemptively -- wait for tuna-f1sh or another issue participant.

## How to resume

```
cd /home/chris/Documents/code/oss-work/cyme
git checkout typec-ports-prototype
gh pr view 124 --repo tuna-f1sh/cyme --comments        # check for new PR review comments
gh issue view 121 --repo tuna-f1sh/cyme --comments      # check for new issue comments
```

If there's a new response:
- If it's design feedback on the correlation approach: re-read the PR body's "Design history"
  section first (source of truth), don't re-litigate from memory of this file.
- If it's an ACPI hardware dump: that's the actual unblocking event for "What's not
  implemented" item 3 in the PR -- validate `device_links` resolution against it as a fixture
  (same pattern as the existing Device-Tree fixtures in `src/usb/typec.rs` tests), don't
  guess at ACPI behavior without the real dump.
- If it's approval to continue: next natural step (already flagged as out-of-scope-for-this-PR
  in the PR body) is wiring `typec_ports` into `src/display.rs` tree/table rendering -- that's
  a new, separate unit of work, likely a new branch/PR of its own rather than piling onto this
  one, but confirm with the user before assuming scope.
- If it's a request to mark the PR "Ready for review" (currently draft): `gh pr ready 124
  --repo tuna-f1sh/cyme`.

## Lessons from this session worth not re-learning

- **Never commit local planning/handoff docs into a branch that backs an open PR.** Keep them
  untracked, in this directory. See the note at the top of this file.
- **GitHub Flavored Markdown renders a single `\n` inside a paragraph as a real `<br>`**, not a
  space (unlike CommonMark). Any PR body/issue comment written with manual line-wrap (~80-100
  cols) will render broken. Reflow to one line per paragraph/list item before posting. Full
  writeup: `reference_github_flavored_markdown_linebreaks` in the persistent memory system.
- **Actions on a new fork need a one-time manual web UI click** to enable -- no API/CLI
  workaround exists. `build.yml` here only triggers on push-to-`main` or `pull_request`, not
  push to arbitrary branches -- use `gh workflow run "Test, build and package" --ref <branch>
  --repo <fork>` to validate CI without needing an actual PR.
- Issue comment tone vs. PR body tone are genuinely different registers (conversational +
  @mention-ok vs. pure first person, never "you", structured headers) -- confirm which one is
  being asked for before drafting, don't assume from a casual "prepare the comment" phrasing.
