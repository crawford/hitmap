{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    sccache
    wrangler
  ];
  shellHook = "export RUSTC_WRAPPER=sccache";
}
