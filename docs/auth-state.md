# Auth-state + Dioxus signals

Prefer `auth_state_changes` over polling. The native SDK restores the session; the callback fires for restore / sign-in / sign-out.

The callback may run off the UI thread. Pipe events into a channel and update a Signal from `spawn`:

```rust,ignore
use dioxus::prelude::*;
use dioxus_firebase::{auth_state_changes, User};

enum AuthStatus { Restoring, Guest, Ready(User) }

let mut status = use_signal(|| AuthStatus::Restoring);
let (tx, mut rx) = futures_channel::mpsc::unbounded();
let _sub = auth_state_changes(move |user| {
    let _ = tx.unbounded_send(user);
})?;
spawn(async move {
    use futures_util::StreamExt;
    while let Some(user) = rx.next().await {
        status.set(match user {
            Some(u) => AuthStatus::Ready(u),
            None => AuthStatus::Guest,
        });
    }
});
```

Keep the returned `AuthStateSubscription` alive (context / `use_hook`).

`User` shape and host wire encoding: [Wire protocol](wire-protocol.md).
