Trooper 0.3.1
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
- Configuration file for keybindings

### Planned
- Visual mode for operating on multiple files at once
- VIM-like repeats of commands (4dd would cut 4 files at once for example)
- Changing the working directory of the shell when exiting trooper

## Installation
Install the binary package from [crates.io](https://crates.io/) using [Cargo](https://doc.rust-lang.org/cargo/) with:
```
cargo install trooper
```

## Configuration
Trooper will look for a config file located at `.config/trooper/config.ini` in your home directory. On Windows this is `%USERPROFILE%\.config\trooper\config.ini` with the equivalent on UNIX being `~/.config/trooper/config.ini`.

The config format is a simple ini format with `=` accepted as the only delimiter. It maps sequences of keystrokes to actions in the program. The default configuration is located in the `/assets` directory. It is this configuration which is overwritten by bindings in the user condfig file.

### Syntax
All keybindsings are located under the section denoted `[keybindings]` in the ini file.

Most keys are mapped simply by the character on the keyboard. Some special keys instead have to be escaped with the same syntax as in a Vim config. The escaped versions follow below:
```
<lt> (<)
<gt> (>)
<Space>
```

Most keys can also be mapped with the ctrl modifier active. This is similarly done as in a Vim config:
```
<C-w> (Ctrl+w)
```

## Dependencies
See `Cargo.toml`
