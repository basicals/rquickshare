# Inbound file completion invariant repair

2026-10-14. Mode A, direct dev. Guide /home/morozhenko/.codex/AGENTS.md and the
user's cross-project maintenance authorization apply. Found while analyzing the
next native received-file reveal slice after completing clipboard images.

## Analysis and plan before code

InboundRequest::process_offline_frame validates ReceivingFiles, known payload ID,
exact current offset and new_offset<=Introduction file size. However an empty LAST
chunk removes the destination from transferred_files without requiring the exact
advertised size. The final such chunk publishes Finished even for a truncated file.
The file handle check currently runs only for nonempty body writes. Success is
therefore weaker than the already enforced declared-size contract and must be fixed
before a future completed-file capability can attest actual receipt.

1. [ ] Require an opened destination for file payload processing; require exact
   transferred size before retiring an empty final chunk. Preserve existing data/
   offset/upper-bound checks and separate empty-final framing. Do not add resume,
   wire fields, deletion/cleanup policy or native reveal APIs in this repair.
2. [ ] Add one table-driven actual request/frame test (no mocked subject): zero/
   one/three-byte declared sizes, empty/partial/exact content, missing destination,
   invalid offsets, multiple file retirement. Rejected EOF must keep the file and
   metadata/ack state; exact complete EOF alone may finish. Keep real temporary
   destinations and owned duplex transport, no network or sleeps.
3. [ ] Run affected inbound checks once and strict core lint, record results,
   atomic fix commit. Rollback is isolated to the EOF/opened-destination checks and
   regression method. The intended advertised-size invariant predates this repair;
   rejecting false successful completion is a bug fix, not a new product choice.

## Supporting inventory

File frame branch: receiving/known-ID/offset/size validation, actual write, nonfinal
empty no-op, premature EOF rejection, exact EOF retirement, last-file Finished.
Primary N2; Positive/N1/N4/N6/N7/N9 also apply. Test final state, retained resources,
bytes and ack count. N3/N5/N8/N10/N11/N12 unchanged by this scoped logic repair.
No exhaustive Mode B, ten-run, installed device or Google acceptance claim.
EOF validation error follows existing manager failure/termination, never Finished.
EOF does not attest fsync durability or immutable received-file contents.
