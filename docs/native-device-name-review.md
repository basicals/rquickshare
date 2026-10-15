# Native custom device name: ownership and application review

2026-10-15. Mode A, direct dev. Analysis only; user choice pending. This is the next
everyday parity item after the implemented native shell/settings/packaging slices.
Read the agent guide, current-state.md, product-direction.md, native-settings.md
and actual core/settings/transport code. No source/schema/API/runtime change yet.

## Actual readers and invariants

- Native Values/StartupSettings and Settings1 edits have visibility, port, directory
  and card mode, but no custom name. GTK cannot edit or report such a value.
- core_lib/src/hdl/mdns.rs builds ServiceInfo once; it currently uses the OS hostname
  both for the DNS hostname and the advertised human-readable endpoint name.
- core_lib/src/lib.rs independently reads the hostname to build BLE/GATT receiver
  endpoint data at transport startup.
- core_lib/src/hdl/outbound.rs reads the hostname twice for ConnectionRequest's
  endpoint_name and serialized RemoteDeviceInfo. A future name edit must not split
  those two fields or change an already admitted session's identity midway.
- mDNS name encoding truncates at255 UTF-8 bytes; BLE receiver data has a smaller
  bounded name budget. A shared validated name must honor the tightest actual
  transport budget, instead of silently advertising different truncated names.
- Existing settings track saved/configured/applied state, replay-safe edits and
  an explicit controlled idle runtime restart without killing the daemon process.
  Receive-root snapshots already show the owner-local admission model.

Preserve DNS host identity, endpoint IDs, PIN/crypto and trust behavior. A display
name is not a trusted identity. No process-global mutable name, direct GTK writes,
account enrollment or protocol-field changes. Preserve legacy/default hostname
behavior when no override exists. Existing transfers keep their admitted name.

## Significant lifecycle alternatives

| Option | Behavior | Advantages | Costs / risks |
|---|---|---|---|
| 1 — daemon-owned live name updates (recommended) | Save one validated override at the existing settings owner. Publish a revisioned owner-local identity to discovery workers and snapshot it for each new session. Report saved and actual applied acknowledgement separately. | Meets the existing no-restart roadmap goal; consistent owner and session identity; changing a presentation setting does not cycle unrelated transport resources or interrupt transfers. | Requires coordinated mDNS/BLE/GATT update acknowledgements and explicit unavailable/partially applied states; consumers can fail independently. Needs a scoped identity refactor and persistence/IPC migration, with adversarial concurrency checks. |
| 2 — apply at controlled idle runtime restart | Persist the override at the same owner, mark restart required, and reuse the existing explicit idle restart to construct all discovery/session owners with one new identity. | One runtime generation has one startup identity; reuses existing saved/applied/restart ownership and avoids a new live discovery coordination state machine. | Changes the documented no-restart UX goal: renaming waits for idle and explicit apply; discovery briefly restarts and acquires a new runtime generation. Risk of a confusing saved-but-old-name state unless GTK labels it clearly. |

Recommend1 for product behavior and architectural ownership, not speed: display
name is a dynamic local presentation policy and should not require restarting
independent receive/send transports. All writers/consumers must use one owner-local
revisioned identity with explicit acknowledgement and per-session snapshots;
merely replacing hostname calls with a shared mutable string is insufficient.
Option2 is coherent if the user explicitly accepts the changed application UX.

## Plan after selection, before production code

1. Freeze exact validation/default/reset, settings schema migration/backup and
   typed IPC compatibility. Source currently rejects unknown fields/schema; do
   not silently append fields without a deliberate version/migration contract.
2. Freeze the selected live-acknowledgement or idle-generation lifecycle, error/
   persistence reporting, discovery visibility interactions and admitted-session
   snapshot points. Document why each existing hostname reader changes or stays.
3. Inventory name byte/Unicode boundaries, defaults, migration/corruption, exact/
   stale retries, reader failures, hidden/visible transitions, concurrent edits/
   sends, cancellation, settings write failures and no-PII diagnostics before tests.
4. Implement owner-local identity, transport consumers, settings/service/client,
   then secondary GTK editor in independently reversible stages. No unrelated
   protocol cleanup, trust-store work or native/legacy shell retirement.
5. Run appropriate affected checks once, record actual results and atomic commits.
   Real Google names/mDNS/BlueZ/KDE checks remain separate acceptance.

Rollback GUI/client changes before persistence/transport consumers; retain exact
pre-migration private backup. Do not overwrite newer incompatible saved state on
rollback. No deployment, installation, live transport or user-settings edits are
needed to prepare the scoped implementation.
