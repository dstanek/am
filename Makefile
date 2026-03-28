DOCKER := $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null)
ifeq ($(DOCKER),)
$(error "Neither podman nor docker found on PATH")
endif

.PHONY: all build-claude build-copilot build-docs-image docs docs-serve

all: build-claude build-copilot

build-claude:
	$(DOCKER) build -f dockerfiles/Dockerfile.claude -t am-claude:latest .

build-copilot:
	$(DOCKER) build -f dockerfiles/Dockerfile.copilot -t am-copilot:latest .

build-docs-image:
	$(DOCKER) build -f dockerfiles/Dockerfile.docs -t am-docs:latest .

docs:
	$(DOCKER) run --rm -v "$(PWD):/docs" am-docs:latest mkdocs build

docs-serve:
	$(DOCKER) run --rm -v "$(PWD):/docs" -p 8000:8000 am-docs:latest mkdocs serve --dev-addr 0.0.0.0:8000
