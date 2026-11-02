# List recipes.
default:
    @just --list

# Build a release binary and link it onto PATH (~/.local/bin and ~/.cargo/bin).
# `just install` and `just install axel` are the same. Links point at
# target/release/<bin>, so later release builds take effect at once.
# Running session hosts keep the old binary until they are restarted.
install bin="axel":
    #!/usr/bin/env bash
    set -euo pipefail
    case "{{bin}}" in
      axel) ;;
      *) echo "unknown binary '{{bin}}' (installable: axel)" >&2; exit 1 ;;
    esac
    cargo build --release -p "{{bin}}" --bin "{{bin}}"
    target="{{justfile_directory()}}/target/release/{{bin}}"
    mkdir -p ~/.local/bin ~/.cargo/bin
    ln -sfn "$target" ~/.local/bin/{{bin}}
    ln -sfn "$target" ~/.cargo/bin/{{bin}}
    echo "installed: $(command -v {{bin}}) -> $(readlink "$(command -v {{bin}})")"
    {{bin}} --version
