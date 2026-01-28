# htop-rs

<img src="logo-small.png" alt="htop-rs">

A Rust port of [htop](https://htop.dev/), the beloved interactive process viewer.

## What is this?

htop-rs aims to be a **1:1 faithful recreation** of htop's user interface and functionality, implemented entirely in Rust. If you're familiar with htop, you should feel right at home.

## Why?

This project exists to explore Rust and experiment with new patterns and architectures—free from the technical debt that accumulates in any long-lived C codebase. It's a learning exercise, a playground for ideas, and hopefully a useful tool.

Goals:
- Match htop's UI pixel-for-pixel (or character-for-character)
- Maintain feature parity with C htop
- Explore Rust idioms for systems programming
- Experiment with architectural improvements

Non-goals:
- Adding features that diverge from htop's design philosophy

## Building

```bash
cargo build --release
```

### Dependencies

- Rust 1.93+

No external libraries required - htop-rs uses a pure Rust terminal implementation.

## Running

```bash
cargo run --release
```

Or after building:
```bash
./target/release/htop-rs
```

## Current Status

**Work in progress.** Many features work, but not everything is implemented yet.

Working:
- Process list with sorting and filtering
- Tree view
- CPU, Memory, Swap meters
- Multiple meter display modes (Bar, Text, LED)
- Setup screen (F2) with meter configuration
- Color themes
- Keyboard navigation
- Process actions (kill, nice, etc.)

Not yet implemented:
- Some column types show "TODO"
- Graph meter mode
- Some platform-specific features

## Platform Support

| Platform | Status |
|----------|--------|
| Linux    | Primary target, most complete |
| macOS    | Supported, some features limited |
| FreeBSD  | Planned (see TODO.md) |

## Contributing

Contributions are **greatly welcome**! Whether it's:

- Implementing missing features
- Fixing bugs
- Improving documentation
- Adding tests
- Platform-specific improvements

Feel free to open issues or submit pull requests.

## Vision

See [TODO.md](TODO.md) for unimplemented features, planned enhancements and architectural ideas.

## License

GPL-2.0-or-later (same as htop)

## Acknowledgments

- The [htop](https://htop.dev/) project and its maintainers
- [btop++](https://github.com/aristocratos/btop) for inspiration on conservative threading design
- Everyone who has contributed to htop over the years
