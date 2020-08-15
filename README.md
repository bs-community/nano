# nano

Fast build tool for official Blessing Skin plugins, written in Rust.

## Special actions

When committing code to Blessing Skin plugins repository,
if the commit message matches some special formats,
you can trigger some actions automatically.

### Force update

If commit message matches:

```
force update: (plugin name)
```

A plugin will be forced to update, even its version isn't changed at that commit.
This is useful for expecting re-run a plugin build.

Example:

```
force update: yggdrasil-api
```

Plugin "yggdrasil-api" will be re-built.

## Build from source

1. Clone this repository with Git.
2. Run `cargo build` or `cargo build --release` for production use.

## License

MIT License

2020-present (c) The Blessing Skin Team
