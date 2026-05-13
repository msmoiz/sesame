# Sesame

Sesame is a simple secret manager. It has two main components: a server that
hosts secrets and a CLI that can be used to publish, list, and retrieve them.
Secrets are simple key-value pairs where the value can be any arbitrary string.

To see general information about the supported commands and options, run the
following command:

```shell
sesame --help
```

To publish and retrieve a secret:

```shell
> sesame publish github-token ghp_123
> sesame list
github-token
> sesame get github-token
ghp_123
```

## Installation

The CLI is published to Armory. If you use Armory, install it with:

```shell
armory install sesame
```

You can also build it locally with Cargo:

```shell
cd cli && cargo install --path .
```

CLI configuration lives in `${HOME}/.sesame/config.toml`.

## Configuration

The CLI reads configuration from environment variables and
`${HOME}/.sesame/config.toml`. Environment variables take precedence.

Supported settings:

- `url`: The server URL (`SESAME_URL`)
- `password`: The server password (`SESAME_PASSWORD`)

Example config:

```toml
url = "https://secrets.msmoiz.com"
password = "super-secret"
```

The server reads configuration from environment variables and `sesame.toml` in
the current working directory. Environment variables take precedence.

Supported settings:

- `address`: The bind address (`SESAME_ADDRESS`)
- `db_path`: The path to the local fjall database (`SESAME_DB_PATH`)
- `password`: The API password (`SESAME_PASSWORD`)
- `flush_interval_secs`: The persist interval in seconds
  (`SESAME_FLUSH_INTERVAL_SECS`)

Example config:

```toml
address = "127.0.0.1:3000"
db_path = "sesame.db"
password = "super-secret"
flush_interval_secs = 30
```

## Server

The server is implemented with Axum and persists secrets using fjall. Data is
stored locally in `sesame.db` and is flushed to disk at regular intervals.

To start the server locally:

```shell
SESAME_PASSWORD=super-secret cargo run -p sesame-server
```

The API exposes the following endpoints:

- `POST /health`
- `POST /publish-secret`
- `POST /list-secrets`
- `POST /get-secret`

Each request must include the configured password in the
`x-sesame-password` header.

## Development

Common workflows are defined in the [justfile](./justfile).

```shell
just develop
just test
just fmt
```

Release metadata for Armory publication lives in `armory.toml`, and GitHub
Actions are configured to build, test, and publish the CLI on tagged commits.
