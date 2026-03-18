DOCKER := $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)
ifeq ($(DOCKER),)
$(error "Neither podman nor docker found on PATH")
endif

.PHONY: all build-claude

all: build-claude

build-claude:
	$(DOCKER) build -f dockerfiles/Dockerfile.claude -t am-claude:latest .
