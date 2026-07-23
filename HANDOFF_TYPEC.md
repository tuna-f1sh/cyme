# Handoff: USB Type-C sysfs prototype for tuna-f1sh/cyme#121

Read this before touching typec code again. Written 2026-07-23 at the end of a work
session, to resume cold in a future session.

## Where things stand

- Branch: `typec-ports-prototype` (local only — **no fork on GitHub yet**, nothing pushed,
  nothing commented on the issue).
- Issue: https://github.com/tuna-f1sh/cyme/issues/121 (feature request: expose USB-C
  alt-mode/e-marker/role info from `/sys/class/typec`).
- Last real activity on the issue: tuna-f1sh (maintainer) commented 2026-07-21, gave a
  green light to prototype, and raised the open design question of how to link a
  `Cable`/`Port` back to an existing `Device` in cyme's tree. He proposed 3 options
  (extend `DeviceLocation`, extend `Path`, or give `Bus` its own `ports`/`cables`) without
  committing to one.
- **Explicit user decision this session: do not comment on the issue yet.** Don't post
  anything to GitHub until told to.

## Design decided (with advisor, verified against real code + kernel source)

`SystemProfile.typec_ports: Option<Vec<TypecPort>>` **top-level**, not nested inside `Bus`
(rejects tuna-f1sh's literal option 3) and not inside `DeviceLocation`/`Path` (options 1/2).
Reasoning that held up under review: `Bus` has no syspath of its own (root-hub syspath lives
on the `Device`), so a typec port with no resolvable device correlation would have nowhere
to live if owned by `Bus` — "orphan port" problem. `DeviceLocation` carries macOS
`system_profiler` compat baggage and is deliberately platform-agnostic, wrong layer for
Linux-only data. Each `TypecPort` instead carries `device_links: Option<Vec<PortPath>>`,
resolved independently, reusing the existing `PortPath` type as the correlation key — same
id/index idiom used by `indextree`/`petgraph`/`sysinfo` in the wider Rust ecosystem, and by
the kernel itself (`port-mapper.c` links pre-existing objects rather than embedding).

Display can still group ports under their `Bus` visually once `device_links` resolves — that
gives tuna-f1sh the UX he asked for without the data-ownership problem.

## What's implemented (commits `b47a83f`, `4453cde` on this branch)

`src/usb/typec.rs` (~910 lines), registered in `src/usb.rs` (`pub mod typec; pub use typec::*;`).

- Types: `TypecPort`, `Partner`, `Cable`, `AltMode`, and enums `DataRole`, `PowerRole`,
  `PreferredRole`, `PortType`, `Orientation`, `PowerOperationMode`, `ProductType`,
  `CableType`, `PlugType`. Every valid value verified against BOTH
  `Documentation/ABI/testing/sysfs-class-typec` AND current `drivers/usb/typec/class.c`
  source (torvalds/linux master) — the ABI doc has drifted from source in real ways (see
  "Traps already found" below), don't trust the doc alone if extending this further.
- `enumerate_typec_ports(root: &Path)` — **root is injectable**, which is how this is tested
  without any real Type-C hardware: tests build fixture directory trees under
  `std::env::temp_dir()` with real attribute values (captured from a collaborator's
  Snapdragon X Elite dumps on the issue) and point `enumerate_typec_ports` at them. No new
  dependency needed (no `tempfile` crate) — see `fixture_root()` in the test module for the
  pattern (atomic counter + pid, safe under `cargo test`'s parallel execution).
- Public entry point `enumerate_default_typec_ports()` wraps it with the real
  `/sys/class/typec/` path, `#[cfg(target_os = "linux")]`-gated, empty `Vec` (not error) on
  other platforms or if the directory doesn't exist.
- Device correlation: reads the reverse `typec` symlink the kernel creates inside a
  `portN-partner/` directory (named after the linked USB device, e.g. `2-2`) — confirmed by
  reading `typec_partner_link_device()` in `class.c` line by line, not assumed. Deliberately
  does **not** follow any `device` symlink (that's the mechanism that caused the UCSI
  recursion bug documented earlier in this issue's research — irrelevant here since this
  code never touches that symlink at all).
- 13 tests, all passing. `cargo clippy --all-targets -- -D warnings` clean (including
  cross-compiled for `x86_64-pc-windows-msvc` — see traps below). `cargo fmt` clean.
  `cargo test --doc typec` clean (1 doctest on `current_choice`).

### Traps already found and fixed (don't reintroduce)

1. `port_type` looked like a plain-value attribute per the ABI doc, but `class.c`
   (`port_type_show()`) **always** emits it bracketed — `"[dual] source sink"` on DRP ports,
   `"[source]"` on fixed-role ports. Must go through `read_choice_attr`, not a plain
   `.parse()`. Was silently returning `None` on 100% of real hardware before the fix.
2. A hub/dock enumerates on the USB2 *and* USB3 bus simultaneously, and the kernel calls
   `typec_partner_link_device()` once per bus — so a partner can have **two** device links,
   not one. `readdir` order is not guaranteed, so picking "the first match" is
   non-deterministic. Field is `device_links: Option<Vec<PortPath>>` (sorted, deduped), not
   a single `Option<PortPath>` — don't regress this back to a scalar.
3. `ProductType`/`CableType` string vocab: the ABI doc says `undefined`; current `class.c`
   emits `not_ufp`/`not_dfp`/`not_cable`/`vpd` instead on modern kernels. Both vocabularies
   are accepted now — if adding new variants, check `class.c` source, not just the doc.
4. `#[cfg(target_os = "linux")] use std::path::Path;` — without the cfg gate this is an
   unused-import warning on Windows/macOS, and `.github/workflows/build.yml` sets
   `RUSTFLAGS=-Dwarnings`, so it silently breaks CI on those runners. Verified by actually
   running `cargo check --target x86_64-pc-windows-msvc`, not by inspection.
5. `name["port".len()..].chars().all(is_ascii_digit)` is vacuously `true` on an empty
   string — a directory literally named `port` would misparse as a port. Guard with a
   non-empty check (already fixed, has a regression test).

Both `code-reviewer` and `security-reviewer` (Fable) ran against the reviewed commit;
security review found no CRITICAL/HIGH (one MEDIUM advisory about `read_attr` having no
size cap, only relevant if `root` ever becomes a CLI-exposed option — it isn't today).

## What's NOT implemented yet

- **Not wired into `SystemProfile` / the profiler pipeline at all.** `typec_ports` doesn't
  exist as a field anywhere outside `src/usb/typec.rs` yet. This is the next step — see
  below.
- No test against a real ACPI machine. All fixtures are DT-style (no `device_links`
  resolved) plus one synthetic ACPI-style fixture built from reading kernel source, not from
  an observed real dump. The DT dumps available on the issue are from Snapdragon
  (Device-Tree) hardware, which structurally cannot produce the ACPI symlink this code reads
  — so this exact machine cannot generate a fixture that exercises the real correlation path
  end to end. Treat `device_links` resolution as *inferred*, not *observed*, until a real
  x86/ACPI dump shows up.
- No fork of `tuna-f1sh/cyme` under the user's GitHub account exists yet.
- Nothing posted to the issue.

## Recommended next step (from an advisor consult at the end of this session)

**Wire to `SystemProfile` next, before chasing ACPI hardware access.** Reasoning: ACPI
validation is fully blocked right now (this machine has no `/sys/class/typec` at all, no
borrowed hardware, no fork/issue engagement yet) — waiting on it stalls the session for
nothing. A runnable prototype (`cyme --json` showing `typec_ports`) is also the thing most
likely to get a real ACPI dump out of an issue participant or the maintainer, so wiring
*unblocks* validation rather than needing to wait for it.

Three decisions to make deliberately during wiring, because they become semi-frozen the
moment they're shown on the issue:

1. Add `#[serde(skip_serializing_if = "Option::is_none")]` on the new `typec_ports` field so
   JSON output on macOS/Windows/no-typec-support stays byte-identical to today. Semantics:
   `None` = platform/kernel doesn't support it, `Some(vec![])` = supported but nothing
   plugged in. Decide and document this explicitly.
2. Do **not** wire hotplug/watch-mode re-enumeration in this pass — `typec_ports` would be a
   stale snapshot in `SystemProfileStream` otherwise, but that's an acceptable, documentable
   v1 limitation, not a blocker. Say so in the module docs so it reads as a scoped decision
   when tuna-f1sh reviews it.
3. Scope the first wiring to `--json` output only (comes for free via serde). Tree/table
   display rendering in `src/display.rs` is a separate, larger task — don't rabbit-hole into
   it in the same pass.

Natural injection point identified earlier: `get_spusb_with_options()` in
`src/profiler.rs:878` — wrap the existing backend dispatch (nusb/libusb) to attach
`typec_ports` once afterward, since it's a system-wide enumeration independent of either
backend, not something that belongs duplicated into `nusb.rs`/`libusb.rs`.

If real ACPI hardware access shows up before the next session (borrowed laptop, a
volunteer's dump), validate `device_links` correlation first instead — a correlation bug
found *after* wiring means re-litigating an already-shown JSON shape, which is more
expensive than finding it before.

## 2026-07-23 (later same day): security-hardening follow-up to `4d03255`

After the wiring commit (`4d03255`, `get_spusb_with_options()` now calls
`enumerate_default_typec_ports()`), two review passes ran against it: `code-reviewer` and
`security-reviewer` (both Fable). `security-reviewer` escalated `read_attr`'s unbounded
`fs::read_to_string()` from the earlier LOW/advisory rating (see "Traps already found" above,
item 5 in that review's original notes) to **MEDIUM**, for one specific reason: reachability
changed.

- **Before `4d03255`**: `read_attr` was inert — nothing outside `src/usb/typec.rs`'s own test
  module called anything in this file. An unbounded read here couldn't be triggered by running
  `cyme` at all.
- **After `4d03255`**: `enumerate_default_typec_ports()` runs on the **default path of every
  Linux invocation** of `cyme` — every output mode (not just `--json`), and in watch mode too,
  since typec re-enumeration was deliberately left un-gated behind any flag (see decision 3 in
  the "Recommended next step" section above). `read_attr` reads whatever `/sys/class/typec/**`
  resolves to, unconditionally, on every run.
- **Risk scenario the reviewer gave**: an adversarial or merely malformed mount under
  `/sys/class/typec` (eg. a symlink pointing at `/dev/zero`, or a FIFO with nothing on the
  writing end) would make `fs::read_to_string` either allocate unbounded memory or block
  indefinitely — on the default path, not an opt-in one. Real kernel sysfs attribute files are
  bounded by the kernel's own `sysfs_kf_read()` to one `PAGE_SIZE` (4096 bytes) per attribute,
  so matching that bound in `read_attr` costs nothing on legitimate hardware.

**Fix applied**: `read_attr` now does `fs::File::open(...)` +
`.take(MAX_ATTR_LEN as u64).read_to_end(&mut buf)` (new `pub(crate) const MAX_ATTR_LEN: usize =
4096` inside the `sysfs` module) instead of `fs::read_to_string`, then `String::from_utf8(buf)`.
Behaviour preserved exactly for every existing case: invalid UTF-8 still yields `None` (via
`.ok()?` on `String::from_utf8`, same as `.ok()?` on `read_to_string` before), and the
trim-then-empty-check-then-`to_string()` tail is untouched. Only new behaviour: reads past 4096
bytes are silently truncated to the cap rather than read in full. `read_choice_attr` needed no
separate fix — it already calls `read_attr` internally rather than reading the file itself, so
capping `read_attr` fixes it transitively; checked, no other unbounded read exists in this file.

New regression test `test_read_attr_caps_oversized_file` (fixture-based, same pattern as the
rest of the module): writes a `usb_power_delivery_revision` attribute file of `MAX_ATTR_LEN +
904` repeated `'a'` bytes, asserts the parsed value's length is exactly `MAX_ATTR_LEN` and
equals `"a".repeat(MAX_ATTR_LEN)` — proves truncation happens at the right byte boundary, not
just "some" truncation. `MAX_ATTR_LEN` re-exported `pub(crate)` under `#[cfg(all(test,
target_os = "linux"))]` so the test can reference the same constant `read_attr` uses, instead of
hardcoding `4096` twice.

Verified clean after the fix: `cargo test` (61 tests, full crate), `cargo clippy --all-targets
--all-features -- -D warnings`, `cargo fmt --check`, and `cargo check --target
x86_64-pc-windows-msvc` (re-ran the cross-compile check from trap #4 since this touches
`#[cfg(target_os = "linux")]`-gated code).

**Why this section exists**: this fix has not been shown to `tuna-f1sh` or posted to the issue —
per the standing instruction in this doc, nothing is being pushed or commented today. This entry
exists so that when a PR is eventually opened upstream, the commit that hardens `read_attr` can
be described accurately and cite the actual security-review finding and reasoning behind it
(escalation from LOW to MEDIUM specifically because of the reachability change in `4d03255`),
rather than reconstructing the justification from scratch in a future session.

## How to resume

```
cd /home/chris/Documents/code/oss-work/cyme
git checkout typec-ports-prototype
git log --oneline b47a83f~1..HEAD   # full commit history of this work
cargo test --lib usb::typec::       # confirm nothing regressed
```
