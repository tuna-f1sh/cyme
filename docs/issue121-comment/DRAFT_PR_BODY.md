# Draft PR body for tuna-f1sh/cyme (targeting #121) -- NOT POSTED, review only

Status: local draft, 2026-07-23. No fork exists on GitHub yet. Nothing has been posted or
opened. Do not fork/PR/comment from this without explicit go-ahead in a future session.

Sources verified against the live issue thread (`gh issue view 121 --repo tuna-f1sh/cyme
--comments`, fetched 2026-07-23) and against the actual struct definitions in the repo, not
against memory/notes. Citations point at real comment timestamps and file:line.

Fourth revision (2026-07-23): added permalinks and pinned diagram placement, per explicit
feedback that the previous pass was missing links/code anchors like the ones tuna-f1sh used in
his own last issue comment, and that diagram placement needs to be exact, not "attached
somewhere below."

**Link method.** Local `main` is at the exact same commit tuna-f1sh linked against in his
2026-07-21 comment: `a00589c108059b2b491dcfb1f1b94c827a043c0c`. Verified by pulling that file
from `git show main:src/profiler/types.rs` and matching against his citations directly: his
`#L339` (`Bus`), `#L920` (`DeviceLocation`), `#L1229` (`Device`) all landed on the exact struct
declarations, not off by a line. Every link below is a GitHub permalink pinned to that same
commit SHA, not to `main` (which would silently drift if the file changes upstream before this
PR is opened) -- same discipline he used, and the two library links (`indextree`/`petgraph`)
below just point at their published crate docs. **What's NOT linked**: `TypecPort`,
`device_links`, and the rest of the code this PR itself introduces. None of it exists on
GitHub yet, since there's no fork, so there's nothing to permalink to. Marked inline below;
convert to real blob links once the fork/branch is actually pushed.

---

## DRAFT PR BODY (candidate, to be pasted into the GitHub PR description verbatim or edited)

### Summary

Adds `SystemProfile.typec_ports: Option<Vec<TypecPort>>`, exposing USB-C alt-mode, cable
e-marker, and data/power role information read from `/sys/class/typec`. Reaches `--json`
output on every Linux run. Related to #121.

### Background

Issue #121 asks for visibility into what alt-modes a Type-C connector supports, and whether a
limitation is the cable, the port, or the host driver. None of `lsusb`/`lshw`/`inxi`/cyme show
that today, even though most of it lives under `/sys/class/typec`, split across sibling
directories instead of one place.

### Design history

The top-level shape here isn't new: it was first proposed in the issue on 2026-07-11
(`typec_ports: Option<Vec<TypecPort>>`, sibling to `buses`). The path to get there, and the
answer to the device-correlation question raised afterward, is what this PR actually spent
its design effort on.

**Why not correlate through the existing `Device` tree.** First pass was an optional field
bolted onto the existing
[`Device`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L1229),
correlated through the `/sys/bus/usb/devices/<dev>/typec` symlink cyme already resolves from
`syspath`. Reading
[`typec_link_ports()`](https://github.com/torvalds/linux/blob/master/drivers/usb/typec/port-mapper.c)
in the kernel source line by line shows that path is ACPI only: it bails out immediately with
no ACPI companion, matching on the shared `_PLD`. A
[2022 patch](https://www.spinics.net/lists/linux-usb/msg221626.html) added that exact guard to
stop a null-deref on Apple M1 (pure Device Tree, no ACPI), by bailing out cleanly rather than
adding Device Tree support. The Snapdragon X Elite hardware discussed in #121 boots mainline
via Device Tree too. Same kernel logic says that correlation shouldn't exist there either;
nobody in the issue has specifically checked for that symlink on the posted dumps, so this is
inferred from source, not independently observed.

Put together two diagrams early on, before writing any code, to get the correlation problem
straight in my own head, and sharing them here since they carry more of the reasoning than
prose alone would. Generated with AI tooling, PlantUML by hand isn't something I use day to
day, but I made sure they represent this exact scenario correctly: checked against the actual
struct definitions and the kernel source discussed above, not a generic template.

**> [DIAGRAM 2 GOES HERE, inline, right after this paragraph] <**
Correlation sequence: how `device_links` resolves on ACPI systems versus Device-Tree/UCSI
systems like the ones in this issue. Uploading the PNG into the GitHub PR body text box at
this exact point auto-generates a `user-attachments` URL and inserts
`![correlation-sequence](<generated-url>)` right here -- not appended at the bottom.

That finding ruled out nesting into the existing device tree and pointed at the top-level
`SystemProfile.typec_ports` shape instead, with device correlation treated as opportunistic,
never required. The hardware dumps posted in the issue on 2026-07-17 (alt-mode naming under
`portN.M` instead of a literal `altmode` dir, the `portN/device` symlink loop, data role and
power role turning out independent) all landed as detail that fits inside that shape without
changing it.

**The device-correlation question.** @tuna-f1sh's comment on 2026-07-21 confirmed `Option`
works for the whole thing and green-lit a prototype, but raised three candidate mechanisms for
linking a `Cable`/`Port` back to an existing
[`Device`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L1229):
extending
[`DeviceLocation`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L920),
extending
[`Path`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/usb/path.rs),
or having
[`Bus`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L339)
own `ports`/`cables` directly.

- **`Bus`-owned, ruled out.**
  [`Bus`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L339)
  doesn't carry a stored syspath, only a bus-number-derived one computed on demand
  ([`Bus::path()`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L510)).
  Without a resolvable device link there's no way to know which `Bus` a port even belongs to,
  without the exact correlation this design is meant to provide. On Device-Tree/UCSI hardware,
  which per the kernel reasoning above is what #121 is actually about, `device_links` comes
  back `None`; not an edge case, plausibly the normal case on that hardware.
- **`DeviceLocation`, ruled out.** Its
  [`FromStr` parsing](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L930),
  and the string form of its `Deserialize` impl, is a direct encoding of macOS
  `system_profiler`'s `LocationReg/DeviceNo` string. That specific path is not
  platform-neutral, even though the struct itself already gets populated directly by the Linux
  backends today (via
  [`location_id`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L1240)
  struct literals in `nusb.rs`/`libusb.rs`), not through that parser. Extending it would still
  mean growing a type whose string contract belongs to macOS.
- **`Path`, reused but narrow.**
  [`PortPath`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/usb/path.rs#L320)
  already exists as a small `{bus, ports}` struct that round-trips to and from the Linux sysfs
  device-name convention (`"1-1.3.4"`) via its own
  [`FromStr`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/usb/path.rs#L327),
  and `Device` already derives one through
  [`port_path()`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler/types.rs#L1546).
  Rather than extending `Path` itself, `TypecPort::device_links: Option<Vec<PortPath>>` *(new
  in this PR, not yet linkable -- see note at the top of this file)* reuses `PortPath` purely as
  the correlation key, resolved independently instead of embedded into the tree. Same
  id/index correlation pattern used by [`indextree`](https://docs.rs/indextree)/[`petgraph`](https://docs.rs/petgraph),
  and the same approach the kernel itself takes: `port-mapper.c` links pre-existing objects
  instead of embedding them.

**> [DIAGRAM 1 GOES HERE, inline, right after this bullet list] <**
Data model: where the three options above would have lived, next to what got built. Same
upload mechanic as Diagram 2 -- inserted at this exact point in the PR text box, not appended.

### What's implemented

- `SystemProfile.typec_ports: Option<Vec<TypecPort>>` *(new, not yet linkable)*, top-level,
  `#[serde(skip_serializing_if = "Option::is_none")]` so JSON stays byte-identical on
  platforms without typec support. `None` means the platform/kernel doesn't support it,
  `Some(vec![])` means it's supported and nothing's plugged in.
- Each `TypecPort` *(new)* carries `Option<Partner>`/`Option<Cable>` for identity, alt-modes,
  PD role and plug type, all optional per the ABI docs, plus `device_links` for the
  correlation above.
- `device_links: Option<Vec<PortPath>>` *(new field on the new type, reusing the existing
  linked `PortPath` above)*, not a single value: a hub/dock enumerates on the USB2 and USB3
  bus at the same time, so the kernel calls `typec_partner_link_device()` once per bus. A
  partner can end up with two device links, not one, and directory read order isn't
  guaranteed, so the field is sorted and deduped.
- Tested without any real Type-C hardware: injectable sysfs root, fixture directory trees
  built from the real attribute values posted in the issue.
- Wired into
  [`get_spusb_with_options()`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/profiler.rs#L878)
  *(existing function this PR modifies, line as of the same pinned commit)*, the single
  injection point independent of either USB backend (nusb/libusb), so it isn't duplicated
  into `nusb.rs`/`libusb.rs`.

### What's not implemented / out of scope for this PR

- Watch-mode re-enumeration. `typec_ports` is a static snapshot captured once per profile;
  hotplug events never refresh it. Documented as a scoped v1 limitation, not an oversight.
- Tree/table display rendering
  ([`src/display.rs`](https://github.com/tuna-f1sh/cyme/blob/a00589c108059b2b491dcfb1f1b94c827a043c0c/src/display.rs)).
  This PR only reaches `--json` output.
- Validation against real ACPI hardware. All fixtures so far are Device-Tree/UCSI style,
  where `device_links` is expected to always be `None`; the one path this can't exercise is
  ACPI symlink resolution succeeding end to end.

### Testing

- `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo fmt
  --check` all clean, including a cross-compiled check against `x86_64-pc-windows-msvc`
  matching CI's `RUSTFLAGS=-D warnings`.
- Two independent review passes on the implementation (general code review and a
  security-focused pass). The security pass flagged that an existing unbounded sysfs read
  (`read_attr`, new in this PR) became reachable on every default Linux invocation once this
  PR wires it in, where before it was inert code; fixed by capping the read at 4096 bytes,
  matching the kernel's own `PAGE_SIZE` bound on sysfs attributes, so it costs nothing on real
  hardware.

### Open questions for review

- Whether the `Bus`/`DeviceLocation`/`Path` reasoning above covers what those three options
  were meant to solve, or whether there's a case they were meant to catch that top-level plus
  `device_links` misses.
- An ACPI-based Type-C laptop dump would help validate the one correlation path none of the
  hardware discussed in #121 can exercise. A `find /sys/class/typec -maxdepth 5 -L -type f
  -exec sh -c 'echo "== $1 =="; cat "$1" 2>/dev/null' _ {} \;` from anyone with that hardware
  would do it.

---

## Things to double check before this goes anywhere

- Whether "Related to #121" should be "Closes #121" or "Fixes #121" -- probably not yet, since
  display/table rendering and ACPI validation are still open, so this reads more like a
  partial/WIP contribution than something that closes the issue outright.
- Whether to attach the `.puml` source too (in a collapsed `<details>` block) or just the
  rendered PNGs.
- The `get_spusb_with_options()` line link (`#L878`) points at the line as it exists on
  `main` today -- once this PR's own commits are pushed and opened, GitHub will show the diff
  at that function directly, so this link is mainly useful for a reviewer checking the
  starting point before the change.
- This still assumes a fork exists and the PR is being opened against `tuna-f1sh/cyme` --
  neither has happened. Posting this requires the fork step first, which is explicitly not
  happening without separate authorization.
