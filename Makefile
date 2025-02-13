SHELL        := /bin/bash
DIST_DIR     := dist

help:
	@printf "Usage: make COMMAND [OPTIONS]\n\nCommands:\n"
	@grep -E '^[a-z].*:' Makefile | sed -r 's/^([^:]+):(.*)/  \1/g'

clean:
	rm -rf $(DIST_DIR)/*

docker-build: clean
	mkdir -p dist
	docker build -f build.dockerfile . -t raptor-cage
	docker run --rm -it raptor-cage cat /builder/target/release/raptor-cage > dist/raptor-cage
	sha256sum dist/raptor-cage
	tar czf dist/raptor-cage.tgz -C dist raptor-cage
