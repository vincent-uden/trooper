Trooper 0.2.0
---

Trooper is a [tui](https://en.wikipedia.org/wiki/Text-based\_user\_interface) file manager with VIM key bindings inspired by the great [ranger](https://github.com/ranger/ranger).

![screenshot](https://raw.githubusercontent.com/vincent-uden/trooper/master/assets/2023-02-13_21-06.png)

## Features
The goal of trooper is to adhere to the unix philosophy. Do one thing and do it well, in this case that thing is managing files. Trooper is not supposed to edit files, preview files (might change my mind on this one), run files or anything which is does not aid in the goal of managing files and directories.

### Implemented
- Navigating the file system
- Copy, cut & paste files across simultaneous running instances of trooper
- Create bookmarks for quick access to directories
- Renaming files
- Persistence for bookmarks and files in the yank register
- Cross-platform support (Linux, Windows and probably Mac)

### Planned
- Visual mode for operating on multiple files at once
- VIM-like repeats of commands (4dd would cut 4 files at once for example)
- Changing the working directory of the shell when exiting trooper
- Configuration file for keybindings (currently only adjustable in the source code)
- A view of the currently bound keybindings

## Installation
Install the binary package from [crates.io](https://crates.io/) using [Cargo](https://doc.rust-lang.org/cargo/) with:
```
cargo install trooper
```

## Dependencies
See `Cargo.toml`
