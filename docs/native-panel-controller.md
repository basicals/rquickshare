# Native panel controller

2026-10-12. Mode A. Implementation stage after native-complete-view.md.
Accepted architecture: one daemon owns transfers; GTK is a thin native client.
No new wire API, authority, settings schema, product flow or fallback decision.

## Analysis and plan before code

NativeClient requires serialized mutable access and retains pending descriptor/
ordered mutation state across interrupted awaits. GTK widgets are confined to the
GLib main thread, while zbus/Tokio IO must run on its own runtime. A GTK callback
must not own an independent client or spawn unbounded per-button mutation tasks.
Use a toolkit-independent controller inside the existing native SDK: one owned
worker, bounded command queue, coalesced immutable view and explicit completions.
The eventual GTK adapter only submits explicit user actions and renders views.
Source: https://docs.gtk.org/gtk4/section-threading.html and
https://gtk-rs.org/gtk4-rs/stable/latest/book/main_event_loop.html.

## Invariants

- Inject a dedicated authenticated NativeClient, no global runtime/connection.
  Controller owns a single Tokio task and at most eight queued actions. Try-send
  gives explicit Busy/Closed instead of silently dropping or spawning work.
- Commands own already acquired OpenedSelection (never paths). Acquisition stays
  in the local chooser/CLI adapter. Files are sent in <=16-FD batches. Only an
  explicit Send command submits the committed selection to a discovered endpoint.
- Watch retains one latest immutable view plus busy/error state. GUI slowness
  cannot block the worker or daemon replies. No per-revision tasks/timer polling.
- Before each action refresh complete state; transfer controls match the requested
  ID/round against the new authoritative view. Ready complete metadata required
  for acceptance, including an exact already-admitted accept retry. Rejection and
  cancellation stay available; terminal state never fabricates acknowledgement.
- Publish busy before starting and complete only after reconciliation plus refresh.
  A cancelled completion receiver does not cancel admitted work or source handles.
  Pending SDK operations are reconciled before accepting new mutation work. If
  reconciliation fails, preserve the pending request, expose explicit error and
  allow a Recover command. Never start a replacement selection/send on uncertainty.
- Unsent selection ownership is the worker's connection. Choose only creates a
  committed selection; Discard explicitly releases it, and controller close
  releases unsent selections through joined SDK connection close. Sent transfers
  remain daemon-owned. Closing/hiding a panel is not Controller.close.
- Explicit controller close cancels waits/current IO, closes SDK and joins worker.
  Drop requests cancellation; no unowned cleanup task. Already admitted transfers
  survive; abandoned local preparation never authorizes implicit sending.
- Sticky daemon/transport failure disables actions and does not reconnect/rebind
  old selections. User error display uses typed redacted errors; no names/PIN/
  payload contents in Debug or diagnostics.

## Milestones / rollback

1. [ ] Implement bounded commands/worker/watch/completion and control validation.
2. [ ] Regression checks using actual private broker/coordinator/native client.
3. [ ] Run affected checks once, strict Clippy/format; record actual results, commit.
4. [ ] Connect GTK Rust shell to controller, prove isolated widget integration.

Rollback this client-only module before any consuming shell. No daemon/protocol
change. Risks: cancellation loses ordered state, unread GUI blocks IO, queue grows,
stale consent, caller assumes Choose sends, close unexpectedly cancels transfers.

## Scoped Mode A verification inventory

Six planned methods with tables where useful:
1. Choose batches >=17 real FDs; chosen selection visible, no transfer until Send;
   explicit send completion/receipt; disconnect survives for accepted transfer.
2. Discard and close release unsent ownership/source clones without deleting files.
3. Queue eight/exact/over, dropped completion receivers, task close/join; no sleeps.
4. Fresh matched ready consent/rejection/cancel, wrong round/unknown/terminal guard.
5. Coalesced changes; busy/completion/current view ordering; owner-loss fail-closed.
6. Recover pending interrupted mutation using existing SDK hooks/private broker;
   no duplicate send or selection after lost completion.
Primary Positive,N2,N4,N5,N7,N9. N1 typed invalid controls/endpoints; N3 existing
owner-loss; N6 epoch; N10 redacted errors. N8/N11/N12 no new clock/billing/storage
rules, inherited file handle failures remain. Do not mock domain/coordinator/SDK.
No Mode B exhaustive coverage or ten-run development gate.

## Actual observations

Pending implementation. All previous verification remains in its stage SSOT.
