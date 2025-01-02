<div align="center">
  <h1>
    raptor-cage
  </h1>
  <p>
    Run Linux games in a secure sandbox, various native and non-native titles are supported.
  </p>
</div>

## ⬇️ Installation

TODO

## 💡 Usage

### Command Line

```bash
# TODO
```

### F.A.Q.

* How to enable MangoHud?  
  Use the `-e MANGOHUD=1` parameter for games that use DXVK and VK3D, other games (OpenGL and WineD3D) may require to prepend `mangohud` before the binary (e.g., `mangohud wine game.exe`).
* What is the difference with Bubblewrap?  
  Bubblewrap (bwrap) is used under the hood by raptor-cage, the bwrap command could be used directly too, however it would require careful configuration of dozens of flags.

## ⚙️ Development

### Maintenance

```bash
# Check for dependency vulnerabilities.
cargo audit

# Check for updates.
cargo upgrade --dry-run
```

### TODOs

#### General

* Some games (like HC2) create a detached sub-process, since we are using `--die-with-parent`, said games will not run when executed directly (with `-b` parameter, executing a shell and launching manually still works); so we need to think in a way to detect child processes and wait for them, or at least add a flag to enable this feature. Disabling `--die-with-parent` is another option, but that would undermine security a bit and leave lingering wine processes all over the place. Maybe add a `--lead-process=NAME_EXE:TIMEOUT` to wait for another process inside the sandbox.
* Allow to pass runner and prefix path relative to the user's Bottles configuration directory so we avoid passing long paths.
* Implement bash autocompletion, should be able to autocomplete prefix and runner names based on the ones detected under Bottles.
* If application binary `-b` does not end with `.exe`, do not prepend `wine`, it's possible that the user wants to run a custom command like `mangohud` or a native Linux game.
* Add `integrate` sub-command to create integrations e.g., `.desktop` shortcut, entry on Heroic launcher.
* Add `list` sub-command to list Bottles' runners and prefixes.
* Wayland support, see https://www.phoronix.com/news/Wine-9.22-Released and https://wiki.archlinux.org/title/Wine#Wayland.
* Add `kill` sub-command to terminate all processes in a sandbox, need to connect to existing bwrap container.

#### Packaging

* AUR package, also add `mangohud` as optional dependency.
* Setup GitHub Actions to automatically publish AUR package.
* Normal binary release.
* cURL install script.
* Create deb package.
* Make a reusable lib version (`Cargo.lock` needs to be ignored, see https://doc.rust-lang.org/cargo/guide/cargo-toml-vs-cargo-lock.html).

#### Maybe

* Simple GUI delivered as Flatpak that builds the needed commands based on the selected options, and creates `.desktop` shortcuts.
* Investigate a way to use `--new-session` while allowing the user to read the output, without relying on seccomp, probably an easy fix could be to create an HTTP server where the output can be seen.
