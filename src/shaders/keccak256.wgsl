// Keccak-256 compute shader for Solidity selector brute-forcing.
// WGSL lacks native u64, so we emulate with (lo, hi) u32 pairs.

struct U64 {
    lo: u32,
    hi: u32,
}

fn u64_zero() -> U64 {
    return U64(0u, 0u);
}

fn u64_from_lo(lo: u32) -> U64 {
    return U64(lo, 0u);
}

fn u64_xor(a: U64, b: U64) -> U64 {
    return U64(a.lo ^ b.lo, a.hi ^ b.hi);
}

fn u64_and(a: U64, b: U64) -> U64 {
    return U64(a.lo & b.lo, a.hi & b.hi);
}

fn u64_not(a: U64) -> U64 {
    return U64(~a.lo, ~a.hi);
}

fn u64_rotl(a: U64, n: u32) -> U64 {
    let r = n % 64u;
    if r == 0u {
        return a;
    }
    if r == 32u {
        return U64(a.hi, a.lo);
    }
    if r < 32u {
        let lo = (a.lo << r) | (a.hi >> (32u - r));
        let hi = (a.hi << r) | (a.lo >> (32u - r));
        return U64(lo, hi);
    }
    let s = r - 32u;
    let lo = (a.hi << s) | (a.lo >> (32u - s));
    let hi = (a.lo << s) | (a.hi >> (32u - s));
    return U64(lo, hi);
}

// Keccak-f[1600] round constants (24 x U64)
const RC_LO = array<u32, 24>(
    0x00000001u, 0x00008082u, 0x0000808au, 0x80008000u,
    0x0000808bu, 0x80000001u, 0x80008081u, 0x00008009u,
    0x0000008au, 0x00000088u, 0x80008009u, 0x8000000au,
    0x8000808bu, 0x0000008bu, 0x00008089u, 0x00008003u,
    0x00008002u, 0x00000080u, 0x0000800au, 0x8000000au,
    0x80008081u, 0x00008080u, 0x80000001u, 0x80008008u,
);
const RC_HI = array<u32, 24>(
    0x00000000u, 0x00000000u, 0x80000000u, 0x80000000u,
    0x00000000u, 0x00000000u, 0x80000000u, 0x80000000u,
    0x00000000u, 0x00000000u, 0x00000000u, 0x00000000u,
    0x00000000u, 0x80000000u, 0x80000000u, 0x80000000u,
    0x80000000u, 0x00000000u, 0x80000000u, 0x80000000u,
    0x80000000u, 0x80000000u, 0x00000000u, 0x80000000u,
);

// Rotation offsets for rho step (lane [x][y] = ROT_OFFSETS[x + 5*y])
// Lane (0,0) is 0 (identity), included for indexing simplicity.
const ROT_OFFSETS = array<u32, 25>(
     0u,  1u, 62u, 28u, 27u,
    36u, 44u,  6u, 55u, 20u,
     3u, 10u, 43u, 25u, 39u,
    41u, 45u, 15u, 21u,  8u,
    18u,  2u, 61u, 56u, 14u,
);

struct Params {
    target_b0: u32,
    target_b1: u32,
    target_b2: u32,
    target_b3: u32,
    count: u32,
}

@group(0) @binding(0) var<storage, read> inputs: array<u32>;
@group(0) @binding(1) var<storage, read_write> matches: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: Params;

var<private> state: array<U64, 25>;

fn keccak_f1600() {
    for (var round = 0u; round < 24u; round = round + 1u) {
        // Theta
        var c: array<U64, 5>;
        for (var x = 0u; x < 5u; x = x + 1u) {
            c[x] = u64_xor(u64_xor(u64_xor(u64_xor(
                state[x], state[x + 5u]), state[x + 10u]), state[x + 15u]), state[x + 20u]);
        }
        var d: array<U64, 5>;
        for (var x = 0u; x < 5u; x = x + 1u) {
            d[x] = u64_xor(c[(x + 4u) % 5u], u64_rotl(c[(x + 1u) % 5u], 1u));
        }
        for (var i = 0u; i < 25u; i = i + 1u) {
            state[i] = u64_xor(state[i], d[i % 5u]);
        }

        // Rho and Pi combined
        var b: array<U64, 25>;
        for (var x = 0u; x < 5u; x = x + 1u) {
            for (var y = 0u; y < 5u; y = y + 1u) {
                let src = x + 5u * y;
                let dst = y + 5u * ((2u * x + 3u * y) % 5u);
                b[dst] = u64_rotl(state[src], ROT_OFFSETS[src]);
            }
        }

        // Chi
        for (var x = 0u; x < 5u; x = x + 1u) {
            for (var y = 0u; y < 5u; y = y + 1u) {
                let i = x + 5u * y;
                state[i] = u64_xor(b[i], u64_and(u64_not(b[((x + 1u) % 5u) + 5u * y]), b[((x + 2u) % 5u) + 5u * y]));
            }
        }

        // Iota
        state[0] = u64_xor(state[0], U64(RC_LO[round], RC_HI[round]));
    }
}

fn read_byte(slot_offset: u32, byte_idx: u32) -> u32 {
    let word_idx = slot_offset + 1u + (byte_idx / 4u);
    let word = inputs[word_idx];
    let shift = (byte_idx % 4u) * 8u;
    return (word >> shift) & 0xffu;
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= params.count {
        return;
    }

    // Each slot is 64 u32s (256 bytes): [u32 length][data as u32s...]
    let slot_offset = idx * 64u;
    let len = inputs[slot_offset];

    // Zero the state
    for (var i = 0u; i < 25u; i = i + 1u) {
        state[i] = u64_zero();
    }

    // Absorb: load input bytes into state as little-endian u64 lanes
    let full_words = len / 8u;
    for (var w = 0u; w < full_words; w = w + 1u) {
        var lo = 0u;
        var hi = 0u;
        for (var b = 0u; b < 4u; b = b + 1u) {
            lo = lo | (read_byte(slot_offset, w * 8u + b) << (b * 8u));
        }
        for (var b = 0u; b < 4u; b = b + 1u) {
            hi = hi | (read_byte(slot_offset, w * 8u + 4u + b) << (b * 8u));
        }
        state[w] = u64_xor(state[w], U64(lo, hi));
    }

    // Remaining bytes (partial u64 word)
    let remaining = len % 8u;
    if remaining > 0u {
        var lo = 0u;
        var hi = 0u;
        for (var b = 0u; b < min(remaining, 4u); b = b + 1u) {
            lo = lo | (read_byte(slot_offset, full_words * 8u + b) << (b * 8u));
        }
        if remaining > 4u {
            for (var b = 0u; b < remaining - 4u; b = b + 1u) {
                hi = hi | (read_byte(slot_offset, full_words * 8u + 4u + b) << (b * 8u));
            }
        }
        state[full_words] = u64_xor(state[full_words], U64(lo, hi));
    }

    // Keccak padding: domain separator 0x01 at position len, 0x80 at position rate-1 (=135)
    let pad_word = len / 8u;
    let pad_byte = len % 8u;
    if pad_byte < 4u {
        state[pad_word] = u64_xor(state[pad_word], U64(0x01u << (pad_byte * 8u), 0u));
    } else {
        state[pad_word] = u64_xor(state[pad_word], U64(0u, 0x01u << ((pad_byte - 4u) * 8u)));
    }
    // rate = 136 bytes = 17 u64 lanes. Last byte of rate block is byte 135 = lane 16, byte 7 (hi byte).
    // 0x80 << (7*8) = 0x80 << 56 = 0x80000000 in the hi word
    state[16] = u64_xor(state[16], U64(0u, 0x80000000u));

    // Permute
    keccak_f1600();

    // Extract first 4 bytes of output (from state[0].lo, little-endian)
    let b0 = state[0].lo & 0xffu;
    let b1 = (state[0].lo >> 8u) & 0xffu;
    let b2 = (state[0].lo >> 16u) & 0xffu;
    let b3 = (state[0].lo >> 24u) & 0xffu;

    // Compare with target
    if b0 == params.target_b0 && b1 == params.target_b1 && b2 == params.target_b2 && b3 == params.target_b3 {
        let slot = atomicAdd(&matches[0], 1u);
        // Write match index starting at matches[1]
        if slot < 1024u {
            atomicStore(&matches[1u + slot], idx);
        }
    }
}
