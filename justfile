# Lists available recipes.
default:
    @just --list

# Runs the sesame server locally.
server:
    cargo run -p sesame-server

# Lists secrets through the CLI.
list:
    cargo run -p sesame -- list

# Runs tests for the workspace.
test:
    cargo test

# Formats the workspace.
fmt:
    cargo fmt --all

# Builds release artifacts.
[linux]
build:
    cargo build --release -p sesame --target=x86_64-unknown-linux-musl

# Builds release artifacts.
[macos]
build:
    cargo build --release -p sesame --target=aarch64-apple-darwin

# Builds release artifacts.
[windows]
build:
    cargo build --release -p sesame --target=x86_64-pc-windows-msvc

# Publishes the CLI to Armory.
[linux]
publish: build
    armory publish --triple x86_64_linux

# Publishes the CLI to Armory.
[macos]
publish: build
    armory publish --triple aarch64_darwin

# Publishes the CLI to Armory.
[windows]
publish: build
    armory publish --triple x86_64_windows
