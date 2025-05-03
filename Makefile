# Compile the Rust project in debug mode
build:
	cargo build

# Compile the project in release mode (optimized)
release:
	cargo build --release

# Run the application
run:
	cargo run

# Run with release build
run-release:
	cargo run --release

# Run unit tests
test:
	cargo test

# Format the code
fmt:
	cargo fmt

# Lint the code using clippy
lint:
	cargo clippy

# Clean the target directory
clean:
	cargo clean

# Watch for file changes and recompile (requires cargo-watch)
watch:
	cargo watch -x run
