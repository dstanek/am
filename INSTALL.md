# Installing am

## Shell script (Linux & macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/dstanek/agent-manager/main/install.sh | sh
```

Installs to `~/.local/bin` by default. Override with `AM_INSTALL_DIR`:

```sh
curl -fsSL https://raw.githubusercontent.com/dstanek/agent-manager/main/install.sh | AM_INSTALL_DIR=/usr/local/bin sh
```

## Homebrew (macOS & Linux)

```sh
brew tap dstanek/am
brew install am
```

## Debian / Ubuntu (.deb)

Download the `.deb` for your architecture from the [latest release](https://github.com/dstanek/agent-manager/releases/latest), then:

```sh
sudo dpkg -i am-x86_64-unknown-linux-gnu.deb      # x86_64
sudo dpkg -i am-aarch64-unknown-linux-gnu.deb     # aarch64
```

## Red Hat / Fedora (.rpm)

Download the `.rpm` for your architecture from the [latest release](https://github.com/dstanek/agent-manager/releases/latest), then:

```sh
sudo rpm -i am-x86_64-unknown-linux-gnu.rpm       # x86_64
sudo rpm -i am-aarch64-unknown-linux-gnu.rpm      # aarch64
```

## Windows

Download `am-x86_64-pc-windows-msvc.zip` from the [latest release](https://github.com/dstanek/agent-manager/releases/latest), extract it, and place `am.exe` somewhere on your `PATH`.

## Build from source

Requires [Rust](https://rustup.rs) 1.70+.

```sh
git clone https://github.com/dstanek/agent-manager.git
cd am
cargo build --release
# binary is at target/release/am
```
