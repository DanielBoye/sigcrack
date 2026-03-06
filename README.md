# sigcrack

Brute-force 4-byte Solidity function selector collisions on CPU (scalar, AVX2, AVX-512) or GPU (wgpu).

## Install

```
cargo install --path .
```

Needs a Rust toolchain and a C compiler.

## Usage

```
sigcrack 0xa9059cbb
sigcrack "transfer(address,uint256)"
sigcrack 0xa9059cbb -n 5 -m readable
sigcrack 0xa9059cbb --cpu --no-tui
sigcrack --list-devices
```

```
Options:
  -n, --count <N>          Collisions to find [default: 1]
  -m, --mode <MODE>        Name style: random or readable [default: random]
      --cpu                CPU only
      --gpu                GPU only, fail if none is found
      --list-devices       Show detected hardware and exit
      --no-tui             Plain output instead of the TUI
  -t, --threads <N>        CPU threads, 0 = physical cores [default: 0]
  -p, --max-params <N>     Max parameters per signature, 0 to 6 [default: 4]
      --simd <SIMD>        auto, avx512, avx2 or scalar [default: auto]
      --gpu-device <ID>    GPU index from --list-devices [default: 0]
```

A GPU is used when one is found, otherwise all physical cores with the widest
available SIMD. The TUI stays open after the requested number of collisions
until you press `q`.

## License

MIT
