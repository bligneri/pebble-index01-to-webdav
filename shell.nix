# Dev shell: `nix-shell`, then `make build` / `make test`.
let
  # nixos-26.05, pinned for reproducible tool versions
  pkgs = import (builtins.fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/da6dc0e7b26060f2f71ce3ef7088fb56ee44fa36.tar.gz";
    sha256 = "sha256-MlvRfa1O5Cqc2/a+gG+l9TSuowEoCKCmrJ1d9oAVm0I=";
  }) { };
in
pkgs.mkShell {
  # mkShell's stdenv provides gcc as the cc linker
  packages = with pkgs; [ cargo rustc rustfmt clippy gnumake openssl kubeseal ];
}
