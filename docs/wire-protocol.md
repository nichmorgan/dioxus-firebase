# Wire protocol

Host maintainers: how `User` snapshots cross the Rust ↔ native boundary.

## User snapshot

`User` mirrors official Auth get-profile shape: top-level `uid` / `email` / `display_name` / `photo_url`, plus `provider_data: Vec<UserInfo>` (each row has `provider_id`, optional `uid` / `email` / `display_name` / `photo_url`).

Hosts pass Firebase top-level fields and every `providerData` row through as-is — no Google/Apple merge in the library. Apps that want a preferred store email (or similar) implement that themselves from `provider_data`.

## Encoding

Auth-state and OK-user replies use the `\x1f` unit separator:

`uid\x1femail\x1fdisplayName\x1fphotoURL\x1fN` then N times `providerId\x1fuid\x1femail\x1fdisplayName\x1fphotoURL`

Error replies use Firebase-style wire codes listed in [Errors and linking](errors-and-linking.md). See also `src/host_protocol.rs` in the crate source.
