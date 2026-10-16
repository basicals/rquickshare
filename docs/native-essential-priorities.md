# Essential native sharing priorities

2026-10-16. Mode A, documentation-only scope change on direct dev.
Guide: /home/morozhenko/.codex/AGENTS.md, sections3.1/11/12/13.
The user explicitly excluded previews and requested only necessary work, ordered
by actual need. This supersedes the broader execution order, not the historical
implementation/verification evidence. Full Google feature parity remains a
long-term inventory, not a gate for a useful ordinary file-sharing release.

## Analysis and rationale

Native11 already implements manual file/folder/text sharing, per-peer consent,
multi-recipient daemon ownership, tray/card, settings, desktop ingress and local
packaging. Its offline checks do not establish installed KDE/Wayland or current
Android/Google Windows acceptance. The immediate gap is evidence of reliable daily
use and fixes revealed by it, rather than more modeled protocol features.

Do not relist implemented features as missing. Existing security and interruption
handling must be inspected/verified before adding mechanisms. Basic sharing must
work without accounts, automatic acceptance or remembered trust. Graceful failure
and a safe explicit retry are essential; byte-offset resume is a later convenience.

## Documentation change plan

1. [x] E1: inspect the current state/audit/roadmap; record the user's scope change,
   essential order, acceptance criteria and excluded expansion here before edits.
2. [x] E2: point current-state/audit/roadmap to this active order; retire preview
   implementation from active next-work lists while retaining dated evidence.
3. [x] E3: verify affected links/whitespace once, commit the scope change atomically
   and refresh mutable chat handoff documents. No runtime/build/network checks.

Risk: an old comprehensive roadmap may be mistaken for the active work queue.
Mitigation: this document owns current priority; wider capability inventories and
dated completed slices remain reference material. Rollback is this documentation
commit; no binary, dependency, wire, settings, key or installed-service change.

## Active order

| Priority | Necessary result | Remaining action / completion evidence |
|---|---|---|
| 1 | Reliable ordinary sharing with current Android and Google Quick Share for Windows | Validate discovery and explicit send/receive of files, several files, folders and text against the current native candidate. Verify recipient identity/code, remote consent, destination and file contents, cancellation and independent member results. Fix interoperability defects found; offline fixtures alone do not close this item. |
| 2 | Safe, understandable failure and explicit retry | Audit existing behavior for network loss, sleep, peer/app exit, full disk, denied access, conflicting filenames and partial writes. No corrupted/partial file presented as complete, unintended overwrite, repeated send or leaked source authority. Failures expose a useful next action; retry is explicit. Add only focused regressions for concrete gaps. |
| 3 | A usable near-clock tray/card in the installed desktop | Verify actual KDE/Wayland placement, Full/Compact, incoming consent notification/card, progress/cancel, lock/DND/focus and reconnect/recovery. Show where received files went and permit opening their location. Existing UI is implemented; fix observed usability/lifecycle defects rather than redesigning again. |
| 4 | Everyday entry points without manual repair | Verify installed file-manager Send, explicit recipient selection, optional login start, one daemon/one tray client and clean shutdown/restart. Current assets exist; acceptance must prove launch ownership and no hidden automatic send. Broader file-manager support is added only for an actual target desktop. |
| 5 | A straightforward installation and update path | Confirm package dependencies/target OS, install/upgrade/removal and settings recovery. Then finish the selected signed APT path with exact release approval and publisher/provenance gates. Existing unsigned artifacts/tooling are preparation, not a published update channel. |

Priorities are ordered by product need, not implementation convenience. Work that
does not need the user's machine can proceed within the same priority: inspect
concrete error paths, make scoped fixes and focused checks, and prepare a short
installed/peer acceptance procedure. Missing physical peers must not turn into
an excuse to expand unrelated features or claim compatibility without evidence.

## Outside the essential release scope

- Previews: excluded by the user. No further codec research, peer preview IPC/card
  binding, outgoing thumbnails or synthetic preview polish is planned. Existing
  bounded opaque preview routing remains protocol input handling/security; do not
  remove it merely because thumbnails are unnecessary. Removing the already
  shipped decoder/helper/dependencies would be a separate concrete cleanup, not
  an implicit change in this documentation task.
- Byte-offset resume/dedup: deferred until reliable failure/cleanup/explicit retry
  and ordinary transfer acceptance are established; not an essential first-release
  gate. Reconsider if large interrupted transfers prove a real daily-use problem.
- Google accounts/Contacts/Your devices, remembered trust/auto-accept, QR, Apple
  transport, streams/Remote Copy, app and Wi-Fi payload completeness: deferred.
  No need to implement them to close ordinary nearby file sharing.
- Persistent history, broad presentation redesign and expanded packaging/platform
  targets: deferred; fix concrete accessibility/readability/recovery defects within
  the essential flows instead of building a separate polish project.

This prioritization authorizes no installation, account access, key operation or
publication. Existing consent, authenticated IPC and file-safety boundaries remain.

## Observed verification and handoff

All103 affected local documentation links resolve. Whitespace verification
passed before the atomic documentation commit. No Rust/UI tests, builds, physical
peer checks, network requests or installed-service changes were needed for this
priority-only change. Mutable current-state/roadmap/audit/product/preview/priority
copies and handoff status are refreshed; immutable native11 binaries, source ZIP,
checksums, unsigned repository and release request remain at their prior measured
checkpoint. All owned inspection/edit/check commands exited: Terminal closed.

## First reliability review — 2026-10-16

[Receive publication](native-receive-reliability.md) found premature final-name
creation and no incomplete-file cleanup authority by inspecting the existing
handler/root/session/receipt lifecycle. The user selected per-file atomic publication and retained completed members.
Implementation and focused verification are recorded in the receive contract;
source/package acceptance is distinct from physical-peer signoff. [Short essential acceptance procedure](native-essential-acceptance.md)
is prepared; no installation, physical action or test repetition was performed.
