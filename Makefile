PROJECT := demo-project
FIREBASE := npx -y firebase-tools
DEFAULT_ANDROID_APPLICATION_ID := com.example.demo

# A process already bound to 9099 has accounts. Stop it so the next start is empty.
# No --import and no --export-on-exit: nothing is read from or written to disk.
define stop_auth_emulator
pids=$$(lsof -ti tcp:9099 || true); \
if [ -n "$$pids" ]; then \
	kill $$pids; \
	sleep 1; \
fi
endef

.PHONY: emulator maestro sync-bundle-id android-demo ios-demo

# Generate gitignored Dioxus.toml from Dioxus.toml.example + ANDROID_APPLICATION_ID
# (default com.example.demo). Node is already required for npx (firebase-tools).
sync-bundle-id:
	@node scripts/sync-bundle-id.js

# Sync package from .env into local Dioxus.toml, then serve the demo
# (live Google needs ANDROID_APPLICATION_ID set).
android-demo: sync-bundle-id
	dx serve --example demo --platform android

ios-demo: sync-bundle-id
	dx serve --example demo --platform ios

emulator:
	@$(stop_auth_emulator)
	$(FIREBASE) emulators:start --only auth --project $(PROJECT)

# Flows use appId: $${ANDROID_APPLICATION_ID}; never hardcode a package in .maestro/.
# Requires Maestro CLI (CI: curl -fsSL https://get.maestro.mobile.dev | bash).
maestro:
	@$(stop_auth_emulator)
	@export PATH="$$HOME/.maestro/bin:$$PATH"; \
	app_id="$${ANDROID_APPLICATION_ID:-}"; \
	if [ -z "$$app_id" ] && [ -f .env ]; then \
		app_id=$$(grep -E '^[[:space:]]*ANDROID_APPLICATION_ID=' .env | tail -1 | sed 's/^[[:space:]]*ANDROID_APPLICATION_ID=//' | sed "s/^['\"]//;s/['\"]$$//" | tr -d '\r'); \
	fi; \
	app_id=$$(echo -n "$$app_id" | xargs); \
	app_id="$${app_id:-$(DEFAULT_ANDROID_APPLICATION_ID)}"; \
	if ! command -v maestro >/dev/null 2>&1; then \
		echo "maestro: command not found. Install with: curl -fsSL https://get.maestro.mobile.dev | bash" >&2; \
		echo "Then ensure $$HOME/.maestro/bin is on PATH (this target prepends it)." >&2; \
		exit 127; \
	fi; \
	$(FIREBASE) emulators:exec --only auth --project $(PROJECT) -- "maestro test -e ANDROID_APPLICATION_ID=$$app_id $(MAESTRO_FLAGS) .maestro"
