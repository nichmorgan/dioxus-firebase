PROJECT := demo-project
FIREBASE := npx -y firebase-tools

# A process already bound to 9099 has accounts. Stop it so the next start is empty.
# No --import and no --export-on-exit: nothing is read from or written to disk.
define stop_auth_emulator
pids=$$(lsof -ti tcp:9099 || true); \
if [ -n "$$pids" ]; then \
	kill $$pids; \
	sleep 1; \
fi
endef

.PHONY: emulator maestro

emulator:
	@$(stop_auth_emulator)
	$(FIREBASE) emulators:start --only auth --project $(PROJECT)

maestro:
	@$(stop_auth_emulator)
	$(FIREBASE) emulators:exec --only auth --project $(PROJECT) -- "maestro test $(MAESTRO_FLAGS) .maestro"
