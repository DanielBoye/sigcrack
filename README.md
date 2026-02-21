# sigcrack

sigcrack is a Rust-based tool designed to find Solidity function signature collisions. This tool is useful for colliding 4-byte Keccak256 hashes, particularly in the context of Ethereum smart contracts where function signatures can collide through brute force.

## Install

Requires Rust 1.70+ and a C compiler. You can install Rust by following the instructions [here](https://www.rust-lang.org/tools/install).

```bash
# Install from source
cargo install --path .
```

## Usage

```bash
# Find a collision for a 4-byte selector
sigcrack "0xa9059cbb"

# Find a collision for a known function signature
sigcrack "transfer(address,uint256)"

# Find 5 collisions
sigcrack "0xa9059cbb" -n 5

# Use readable function names
sigcrack "0xa9059cbb" -m readable

# Force CPU-only (skip GPU)
sigcrack "0xa9059cbb" --cpu

# Print results to stdout instead of the TUI
sigcrack "0xa9059cbb" --no-tui

# Show detected hardware
sigcrack --list-devices
```

### Default Behavior

Running `sigcrack <selector>` with no flags will:

1. Auto-detect GPU via Vulkan/Metal/DX12 — if available, use it
2. If no GPU, fall back to CPU with the best available SIMD (AVX-512 > AVX2 > scalar)
3. Use all physical CPU cores
4. Generate random function names with up to 4 parameters
5. Display a live AFL++-style TUI dashboard
6. Keep running after finding a collision until you press `q`

### Options

```
Usage: sigcrack [OPTIONS] [TARGET]

Arguments:
  [TARGET]  Selector or signature, e.g. "transfer(address,uint256)" or "0xa9059cbb"

Options:
  -n, --count <N>           Collisions to find [default: 1]
  -m, --mode <MODE>         Name style: random, readable [default: random]
      --cpu                 Force CPU-only (skip GPU)
      --gpu                 Force GPU-only (skip CPU)
      --list-devices        Show detected hardware and exit
      --no-tui              Print to stdout instead of TUI
  -t, --threads <N>         CPU worker threads [0 = physical cores] [default: 0]
  -p, --max-params <N>      Max parameters per generated signature [default: 4]
      --simd <MODE>         Force SIMD: auto, avx512, avx2, scalar [default: auto]
      --gpu-device <ID>     GPU device index for multi-GPU systems [default: 0]
  -h, --help                Print help
  -V, --version             Print version
```

| Flag | Description |
|------|-------------|
| `-n, --count` | How many collisions to find before workers stop. The TUI stays open until you quit. |
| `-m, --mode` | `random` generates names like `s_jRi`, `readable` generates camelCase names like `getBalanceOf`. |
| `--cpu` | Skip GPU detection and use CPU SIMD workers only. |
| `--gpu` | Require GPU — exit with an error if no GPU is available. |
| `-t, --threads` | Override CPU thread count. Set to `0` (default) to auto-detect physical cores. |
| `-p, --max-params` | Controls how many parameters generated signatures can have (0-N). Higher values mean more type combinations but slower iteration per name. |
| `--simd` | Force a specific SIMD backend. `auto` picks the best available. Useful for benchmarking. |
| `--gpu-device` | Select which GPU to use by index. Run `--list-devices` to see available devices. |

## Time Complexity Analysis

Solidity function selectors are the first 4 bytes of the Keccak256 hash of the function signature (e.g., `keccak256("transfer(address,uint256)")` yields `0xa9059cbb`). Since selectors are 4 bytes (32 bits), the total selector space is 2^32 = **4,294,967,296** possible values.

### Expected Hashes to First Collision

By the birthday problem, the probability of finding a collision for a specific target selector after `n` random trials is:

```
P(hit after n trials) = 1 - (1 - 1/2^32)^n
```

| Probability | Hashes Required |
|-------------|-----------------|
| 50%         | ~2.98 billion   |
| 75%         | ~5.96 billion   |
| 90%         | ~9.89 billion   |
| 95%         | ~12.87 billion  |
| 99%         | ~19.73 billion  |

The **expected** number of hashes for a single collision is **2^32 = ~4.29 billion**.

### Estimated Wall Time

Performance depends on hardware and SIMD capability:

| Backend        | Approx. Rate     | Expected Time (1 collision) |
|----------------|-------------------|-----------------------------|
| Scalar (1 core)| ~2-3 M/sec       | ~25-35 minutes              |
| AVX2 (6 cores) | ~10-15 M/sec     | ~5-7 minutes                |
| AVX-512 (8c)   | ~30-50 M/sec     | ~1.5-2.5 minutes            |
| GPU (RTX 3060) | ~100-500 M/sec   | ~10-45 seconds              |

These are approximate. Actual performance varies with CPU model, clock speed, and GPU compute capability. Use `--no-tui` mode to see the live hash rate for your system.

### Finding Multiple Collisions

Each additional collision requires roughly another 2^32 hashes on average, since each trial is independent. Finding `n` collisions requires approximately `n * 2^32` hashes.

## Uninstall

```bash
# If installed via cargo install
cargo uninstall sigcrack

# If built from source, just delete the repo
rm -rf /path/to/sigcrack
```

No files are written outside the project directory.
