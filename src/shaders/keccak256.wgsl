// Keccak-256 selector search. WGSL has no u64, so lanes are (lo, hi) u32 pairs.

const MAX_MATCHES: u32 = 1024u;

struct U64 {
    lo: u32,
    hi: u32,
}

fn u64_xor(a: U64, b: U64) -> U64 {
    return U64(a.lo ^ b.lo, a.hi ^ b.hi);
}

fn u64_rotl(a: U64, n: u32) -> U64 {
    if n == 0u {
        return a;
    }
    if n == 32u {
        return U64(a.hi, a.lo);
    }
    if n < 32u {
        return U64((a.lo << n) | (a.hi >> (32u - n)), (a.hi << n) | (a.lo >> (32u - n)));
    }
    let s = n - 32u;
    return U64((a.hi << s) | (a.lo >> (32u - s)), (a.lo << s) | (a.hi >> (32u - s)));
}

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
    0x80000000u, 0x80000000u, 0x00000000u, 0x80000000u,
    0x80000000u, 0x80000000u, 0x00000000u, 0x80000000u,
);

const RHO = array<u32, 25>(
     0u,  1u, 62u, 28u, 27u,
    36u, 44u,  6u, 55u, 20u,
     3u, 10u, 43u, 25u, 39u,
    41u, 45u, 15u, 21u,  8u,
    18u,  2u, 61u, 56u, 14u,
);

struct CounterParams {
    selector: vec4<u32>,
    count: u32,
    len: u32,
    _pad: vec2<u32>,
    // Mixed-radix digits of the first name: start[p / 4][p % 4] is position p.
    start: array<vec4<u32>, 4>,
    // The 136-byte block as LE words with the name bytes zero: type suffix,
    // 0x01 pad after it and 0x80 in byte 135 are baked in on the CPU.
    tmpl: array<vec4<u32>, 9>,
}

struct WordParams {
    selector: vec4<u32>,
    count: u32,
    tail_len: u32,
    _pad: vec2<u32>,
    // Type suffix followed by the 0x01 pad, LE words from byte 0. Each thread
    // shifts it to the end of its own name.
    tail: array<vec4<u32>, 4>,
}

// Each entry point has its own bindings so each pipeline's auto-derived layout
// only sees what it uses.
@group(0) @binding(3) var<uniform> counter: CounterParams;
@group(0) @binding(4) var<storage, read_write> counter_matches: array<atomic<u32>>;
// One entry per dictionary word: (len, bytes 0-3, 4-7, 8-11) LE, zero padded.
@group(0) @binding(5) var<storage, read> words: array<vec4<u32>>;
@group(0) @binding(6) var<uniform> word_params: WordParams;
@group(0) @binding(7) var<storage, read_write> word_matches: array<atomic<u32>>;

var<private> state: array<U64, 25>;

fn keccak_f1600() {
    for (var round = 0u; round < 24u; round++) {
        var c: array<U64, 5>;
        for (var x = 0u; x < 5u; x++) {
            c[x] = u64_xor(u64_xor(u64_xor(u64_xor(
                state[x], state[x + 5u]), state[x + 10u]), state[x + 15u]), state[x + 20u]);
        }
        for (var x = 0u; x < 5u; x++) {
            let d = u64_xor(c[(x + 4u) % 5u], u64_rotl(c[(x + 1u) % 5u], 1u));
            for (var y = 0u; y < 25u; y += 5u) {
                state[x + y] = u64_xor(state[x + y], d);
            }
        }

        var b: array<U64, 25>;
        for (var x = 0u; x < 5u; x++) {
            for (var y = 0u; y < 5u; y++) {
                let src = x + 5u * y;
                let dst = y + 5u * ((2u * x + 3u * y) % 5u);
                b[dst] = u64_rotl(state[src], RHO[src]);
            }
        }

        for (var y = 0u; y < 25u; y += 5u) {
            for (var x = 0u; x < 5u; x++) {
                let b1 = b[y + (x + 1u) % 5u];
                let b2 = b[y + (x + 2u) % 5u];
                state[y + x] = U64(b[y + x].lo ^ (~b1.lo & b2.lo), b[y + x].hi ^ (~b1.hi & b2.hi));
            }
        }

        state[0] = u64_xor(state[0], U64(RC_LO[round], RC_HI[round]));
    }
}

fn selector_matches(wanted: vec4<u32>) -> bool {
    let sel = vec4<u32>(
        state[0].lo & 0xffu,
        (state[0].lo >> 8u) & 0xffu,
        (state[0].lo >> 16u) & 0xffu,
        (state[0].lo >> 24u) & 0xffu,
    );
    return all(sel == wanted);
}

// REST_CHARS order: a-z A-Z 0-9 _
fn rest_char(d: u32) -> u32 {
    return select(select(select(95u, 48u + d - 52u, d < 62u), 65u + d - 26u, d < 52u), 97u + d, d < 26u);
}

// FIRST_CHARS order: a-z A-Z _
fn first_char(d: u32) -> u32 {
    return select(select(95u, 65u + d - 26u, d < 52u), 97u + d, d < 26u);
}

// Adds the low base-63 digit of `carry` to a start digit and leaves the carry
// into the next (slower) position. Work::signature on the CPU mirrors this.
fn next_digit(start: u32, carry: ptr<function, u32>) -> u32 {
    let d = start + *carry % 63u;
    *carry = *carry / 63u + d / 63u;
    return rest_char(d % 63u);
}

// Thread gid.x hashes the name `start + gid.x` in a mixed radix counter with
// position len-1 fastest. All names in a dispatch share one length.
@compute @workgroup_size(256)
fn main_counter(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= counter.count {
        return;
    }
    let len = counter.len;
    var carry = idx;
    var nw0 = 0u;
    var nw1 = 0u;
    var nw2 = 0u;
    var nw3 = 0u;

    if 15u < len { nw3 |= next_digit(counter.start[3].w, &carry) << 24u; }
    if 14u < len { nw3 |= next_digit(counter.start[3].z, &carry) << 16u; }
    if 13u < len { nw3 |= next_digit(counter.start[3].y, &carry) << 8u; }
    if 12u < len { nw3 |= next_digit(counter.start[3].x, &carry); }
    if 11u < len { nw2 |= next_digit(counter.start[2].w, &carry) << 24u; }
    if 10u < len { nw2 |= next_digit(counter.start[2].z, &carry) << 16u; }
    if 9u < len { nw2 |= next_digit(counter.start[2].y, &carry) << 8u; }
    if 8u < len { nw2 |= next_digit(counter.start[2].x, &carry); }
    if 7u < len { nw1 |= next_digit(counter.start[1].w, &carry) << 24u; }
    if 6u < len { nw1 |= next_digit(counter.start[1].z, &carry) << 16u; }
    if 5u < len { nw1 |= next_digit(counter.start[1].y, &carry) << 8u; }
    if 4u < len { nw1 |= next_digit(counter.start[1].x, &carry); }
    if 3u < len { nw0 |= next_digit(counter.start[0].w, &carry) << 24u; }
    if 2u < len { nw0 |= next_digit(counter.start[0].z, &carry) << 16u; }
    if 1u < len { nw0 |= next_digit(counter.start[0].y, &carry) << 8u; }
    let d0 = counter.start[0].x + carry;
    if d0 >= 53u {
        return;
    }
    nw0 |= first_char(d0);

    for (var v = 0u; v < 9u; v++) {
        let t = counter.tmpl[v];
        state[2u * v] = U64(t.x, t.y);
        state[2u * v + 1u] = U64(t.z, t.w);
    }
    for (var i = 18u; i < 25u; i++) {
        state[i] = U64(0u, 0u);
    }
    state[0].lo ^= nw0;
    state[0].hi ^= nw1;
    state[1].lo ^= nw2;
    state[1].hi ^= nw3;

    keccak_f1600();

    if selector_matches(counter.selector) {
        let n = atomicAdd(&counter_matches[0], 1u);
        if n < MAX_MATCHES {
            atomicStore(&counter_matches[1u + n], idx);
        }
    }
}

// ORs the LE word `v` into `msg` at any byte offset `o`. WGSL masks shift
// amounts to 5 bits, so the spill into the next word is skipped when aligned.
fn place_word(msg: ptr<function, array<u32, 34>>, o: u32, v: u32) {
    let s = (o % 4u) * 8u;
    (*msg)[o / 4u] |= v << s;
    if s != 0u {
        (*msg)[o / 4u + 1u] |= v >> (32u - s);
    }
}

// Appends word `entry` at byte offset `o`. Words after the first get their
// first byte uppercased, which for lowercase ASCII is clearing bit 5.
fn place_dict_word(msg: ptr<function, array<u32, 34>>, o: u32, entry: vec4<u32>, upper: bool) -> u32 {
    place_word(msg, o, entry.y ^ select(0u, 0x20u, upper));
    place_word(msg, o + 4u, entry.z);
    place_word(msg, o + 8u, entry.w);
    return o + entry.x;
}

// Thread gid.x hashes name gid.x in WordNameIter order: every 1-word name,
// then every 2-word name (first index slowest), then every 3-word name, each
// followed by the type suffix in `word_params.tail`.
@compute @workgroup_size(256)
fn main_words(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if idx >= word_params.count {
        return;
    }
    let n = arrayLength(&words);
    var t = idx;
    var msg: array<u32, 34>;
    var len = 0u;
    if t < n {
        len = place_dict_word(&msg, 0u, words[t], false);
    } else {
        t -= n;
        if t < n * n {
            len = place_dict_word(&msg, 0u, words[t / n], false);
            len = place_dict_word(&msg, len, words[t % n], true);
        } else {
            t -= n * n;
            len = place_dict_word(&msg, 0u, words[t / (n * n)], false);
            len = place_dict_word(&msg, len, words[(t / n) % n], true);
            len = place_dict_word(&msg, len, words[t % n], true);
        }
    }
    let tail_words = (word_params.tail_len + 3u) / 4u;
    for (var k = 0u; k < tail_words; k++) {
        place_word(&msg, len + 4u * k, word_params.tail[k / 4u][k % 4u]);
    }
    msg[33] |= 0x80000000u;

    for (var w = 0u; w < 17u; w++) {
        state[w] = U64(msg[2u * w], msg[2u * w + 1u]);
    }
    for (var i = 17u; i < 25u; i++) {
        state[i] = U64(0u, 0u);
    }

    keccak_f1600();

    if selector_matches(word_params.selector) {
        let n = atomicAdd(&word_matches[0], 1u);
        if n < MAX_MATCHES {
            atomicStore(&word_matches[1u + n], idx);
        }
    }
}
