# chaincash-server
ChainCash Payment Server - reference implementation in Rust


## Developing

### Auto Reload

When working on chain cash server development it can be useful to have auto-reload setup such that
whenever code changes the app is recompiled and restarted. Using `listenfd` we are able to migrate
connections from the old version of the app to the newly-compiled version.

Auto reload requires the following tools to be installed:

```sh
cargo install cargo-watch systemfd
```

Now the app can be ran with auto reload like so:

```sh
systemfd --no-pid -s http::8080 -- cargo watch -x run
```
