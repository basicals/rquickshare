# Same-PC signing recovery destination review

2026-10-15 selected: existing Google Drive ArcVault at the user-provided mounted
secrets path. R2 options below are history; current plan: native-signing-backup.md.

2026-10-14. Mode A. User confirmed local Pinentry; real local key initialization
and non-release challenge verification succeeded in native-local-signing.md.
No extra machine/VM/token is needed or proposed. Current key material exists only
on the existing Linux workstation, so loss of that disk remains a recovery risk.

## Decision before external key-backup storage

1. Recommended with current available infrastructure: locally encrypted recovery
   archive in a separate private R2 bucket, without domain/r2.dev/Worker bindings.
   Benefits: recovery survives workstation loss and is outside the release
   publisher's bucket-scoped credential/lifecycle. Uses the existing Cloudflare
   account, not another computer. Costs: one storage bucket, scoped credentials,
   local encryption/password entry, readback and restore verification. Risks:
   losing the archive password or Cloudflare account, deletion of the backup, or
   compromise of the workstation while encryption is unlocked. Upload ciphertext
   only; no password/key-decryption credential in R2, Git, CI or chat.
2. Locally encrypted archive in an existing user-chosen backup location outside
   this workstation (existing cloud drive/external storage). Benefits: reuse an
   established backup/account and potentially separate provider ownership. Costs:
   exact destination/access and retention must be supplied, restore/readback must
   be verified. Risks: losing password/storage access, sync deletion, accidental
   sharing or a destination that is actually on the same failed disk. Do not invent
   a mount/folder or require purchase of a new device.

First option is recommended given the user's existing Cloudflare/R2 setup and
clear separation from distribution credentials. Neither option makes a same-PC
primary offline. Encrypt locally in both; Cloudflare server-side at-rest encryption
alone is not a substitute for client-side protection of signing recovery material.

## Scope after selection

Document exact source inventory, local encryption/secret input/restore boundary
and selected destination before implementation. Prepare a protected archive of
necessary authority/subkey/revocation/public identity material, exclude sockets,
logs and unrelated GPG homes. Verify decryption/restoration and primary/subkey
identity in a separately owned temporary profile; never overwrite active custody.
Only then upload ciphertext with fixed bounded/checksummed immutable keys and
positive readback, verify destination remains private/no Worker binding, and
record recoverability. Leave the active profiles and existing archive route intact.
No unattended signing/background schedule or password stored with ciphertext.

No recovery archive/key upload/new bucket was created before this decision. The
existing publisher also still needs its own bucket-scoped credential and explicit
review of the canonical release request before actual package signing/promotion.
Key possession verification is not that release approval. Native install/upgrade/
Google peer acceptance remain separately open. Stop only the dependent backup/
publication work under AGENTS3/3.1 pending the destination decision.

## Primary references inspected

https://developers.cloudflare.com/r2/buckets/public-buckets/
https://developers.cloudflare.com/r2/reference/data-security/
Cloudflare skill reapplied for this review. Public access is an explicit bucket
setting; private backup must have neither public domain nor public development URL.
The earlier /r2/data-security/ URL failed and was replaced by the official reference
link from the current R2 documentation, not treated as verified content.
