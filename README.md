# ChainCash Server

[![ci](https://github.com/ChainCashLabs/chaincash-rs/actions/workflows/ci.yaml/badge.svg)](https://github.com/ChainCashLabs/chaincash-rs/actions/workflows/ci.yaml) [![audit](https://github.com/ChainCashLabs/chaincash-rs/actions/workflows/audit.yaml/badge.svg)](https://github.com/ChainCashLabs/chaincash-rs/actions/workflows/audit.yaml)
[![Discord badge][]][Discord link] [![Telegram](https://img.shields.io/badge/Telegram-2CA5E0?style=flat-squeare&logo=telegram&logoColor=white)](https://t.me/+xIwo9PNJdtdhMzZl)

## Running

We aren't yet shipping prebuilt `ChainCash` binaries so currently you must build from source yourself using `cargo`.

The easiest way to build and run currently is like so:

```sh
cargo run -- run
```

This will start the `ChainCash` server and initialize the database, etc.

## Configuration

`ChainCash` is configured using `TOML` config files stored in `$CWD/config`.

The default settings can be viewed in [`./config/default.toml`](./config/default.toml).

Default config values can be overriden by creating your own config file at [`./config/local.toml`] and supplying custom values.

## Predicates

### Predicate Configuration

Predicates are configured using `TOML` based config files. The structure of these config files are as follows:

```toml
type = "{predicateType}" # the type of predicate
{...args} # `args` is unique to the predicate type and are listed below
```

To enable a predicate config we specify it in our main `ChainCash` configuration file under the `acceptance` section:

```toml
[acceptance]
predicates = ["path/to/my/predicate1.toml", "path/to/my/predicate2.toml"]
```

If any of the predicates listed in the `predicates` field evaluate to `true` for a given note then the note will be considered acceptable.

Example of a predicate config file can be seen [here](./config/predicates/example.toml).

### Viewing Configured Predicates

With the `ChainCash` server running locally we can perform an API request to get our predicate configuration.

```sh
curl http://localhost:8080/api/v1/acceptance
```

### Predicate Types

Currently the following predicates are supported:

#### Whitelist

A `whitelist` predicate evaluates to `true` if any of the suppplied agents are the current owner of the note.

If the owner is known to us and trusted we can accept the note without any other consideration.

For example, this could be configured like so:

```toml
type = "whitelist"
agents = ["PK1", "OWNER2"]
```

#### Collateral

A `collateral` predicate evaluates to `true` if the note is backed by at least the `percent` supplied.

For example, we could require the note be over-collaterized if we really don't trust the owner, or under collaterized if we have _some_ trust in the holder.

Configuration of a note that requires at least 100% collateral.

```toml
type = "collateral"
percent = 100
```


#### Or

An `or` predicate evaluates to `true` if any of the conditions supplied evalute to `true`.

For example, if we want to express that a note is accepted if the note is over collaterized or the owner is trusted by us we could do the following:

```toml
# any of the conditions below match
type = "or"
conditions = [
    # the owner of the note is either PK1 or PK2
    {type = "whitelist", agents = ["PK1", "PK2"]},
    # the note has at least 100% collateral
    {type = "collateral", percent = 100}
]
```

[Discord badge]: https://img.shields.io/discord/668903786361651200?logo=discord&style=social
[Discord link]: https://discord.gg/ergo-platform-668903786361651200
