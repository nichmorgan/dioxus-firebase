# Auth emulator

Optional: `use_auth_emulator(host, port)` after init. Apps can keep using staging cloud in debug and adopt the emulator later.

## Hosts

| Target | Host | Notes |
| -- | -- | -- |
| Android emulator | `10.0.2.2` | Emulator alias for the host machine; no `adb reverse` |
| USB Android device | `127.0.0.1` | Needs `adb reverse` (re-run after reconnect) |
| iOS simulator | `127.0.0.1` | Shares the host network |

```bash
adb reverse tcp:9099 tcp:9099
```

The demo uses `10.0.2.2` on an Android emulator and `127.0.0.1` on a physical Android device or the iOS simulator.

## Starting the emulator

Start a stateless Auth emulator on the host (`0.0.0.0:9099` in [`firebase.json`](../firebase.json)). `make emulator` stops anything already listening on port 9099, then starts a new process with an empty account store. It does not import or export accounts:

```bash
make emulator
```

Accounts created while that process is running stay until it exits. The next `make emulator` does not load them.

## Cleartext

Dioxus CLI's generated network-security config only allows cleartext HTTP to `127.0.0.1`. Debug builds of an app that embeds this plugin also allow `10.0.2.2` (and `localhost`). Release builds are unchanged.

Federated Google against the emulator is limited — see [Google sign-in](google-sign-in.md). Demo / Maestro wiring: [Demo and Maestro](demo-and-maestro.md).
