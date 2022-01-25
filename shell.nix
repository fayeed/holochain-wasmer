let
  holonixPath = (import ./nix/sources.nix).holonix;

  holonix = import (holonixPath) {
    include = {
      holochainBinaries = false;
    };
  };

in
holonix.pkgs.mkShell {
  inputsFrom = [ holonix.main ];
  shellHook = ''
    export RUST_BACKTRACE=full
    export WASMER_BACKTRACE=1
  '';

  packages = with holonix.pkgs; [
    nixpkgs-fmt
  ];
}
