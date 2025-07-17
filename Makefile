build: build-frontend build-bpf build-rust

build-frontend:
	cd frontend && npm install && npm run build

build-bpf:
	make -C bpf

build-rust:
	cd collector && cargo build --release

clean:
	make -C bpf clean
	cd collector && cargo clean
	cd frontend && rm -rf .next node_modules dist

install:
	sudo apt update
	sudo apt-get install -y --no-install-recommends \
        libelf1 libelf-dev zlib1g-dev \
        make clang llvm
	# Install Node.js if not present
	@command -v node >/dev/null 2>&1 || { \
		echo "Installing Node.js..."; \
		curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -; \
		sudo apt-get install -y nodejs; \
	}
	# Install Rust if not present
	@command -v cargo >/dev/null 2>&1 || { \
		echo "Installing Rust..."; \
		curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y; \
		source ~/.cargo/env; \
	}

test:
	make -C bpf test
	cd collector && cargo test
	cd frontend && npm run build

.PHONY: build build-frontend build-bpf build-rust clean install test