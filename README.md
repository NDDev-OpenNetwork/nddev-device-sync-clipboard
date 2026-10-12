# NDDev OpenNetwork · clipboard

AGPL-3.0-only NDS adapter. [NDDev OpenNetwork](https://nddev.ai).

Browse native archive pages and fetch one explicitly selected representation, capped at 1 MiB. The archive remains provider-owned.

`contracts/module.json` is the canonical compiled-in runtime descriptor;
`module.yaml` identifies its repository package. Caller-owned local/process
capability handles share a bounded agent admission budget. Native responses,
private paths and content are never Debug or telemetry payloads.

Run `just check`, then `just provider-check` with the explicit `NDS_TEST_*`
provider paths used by CI. Acceptance exercises the real pinned native owner
in isolated temporary state. Linux provider acceptance and portable source
checks are distinct; an absent provider is unavailable, not a successful stub.
There is no backup/recovery implementation or private estate configuration.

The browser adapter reserves a ClipboardItem Promise synchronously during an
explicit user gesture, then accepts the Rust consumer's bounded text. No secret
is returned through presentation DTOs. Copy requires a secure context and the
browser's permission; it never reads or archives the system clipboard implicitly.
Browser copy cannot exclude OS history or revoke a write already handed to the
browser. A single pending request owns a 15-second response deadline; cancellation
before supplying text rejects the item. Native clipboard policy stays native.
