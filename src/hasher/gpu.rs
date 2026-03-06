use std::sync::mpsc;

use wgpu::util::DeviceExt;

use super::{Selector, RATE};
use crate::namegen::{
    camel_case, FIRST_CHARS, MAX_CHARS, MAX_WORDS, MAX_WORD_NAME_LEN, REST_CHARS,
};

const SHADER: &str = include_str!("../shaders/keccak256.wgsl");
const MAX_MATCHES: usize = 1024;
const WORKGROUP_SIZE: u32 = 256;
/// The default `max_compute_workgroups_per_dimension`. Every dispatch,
/// counter or word, must fit under it.
const MAX_WORKGROUPS: u32 = 65_535;
/// Threads per counter dispatch.
pub const DISPATCH: u32 = 1 << 22;
const _: () = assert!(DISPATCH.div_ceil(WORKGROUP_SIZE) <= MAX_WORKGROUPS);
/// Dispatches in flight at once.
pub const SLOTS: usize = 2;
/// Bytes of type suffix plus the 0x01 pad the word shader's uniform holds.
const MAX_TAIL_LEN: usize = 64;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CounterParams {
    selector: [u32; 4],
    count: u32,
    len: u32,
    _pad: [u32; 2],
    start: [u32; 16],
    tmpl: [u32; 36],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct WordParams {
    selector: [u32; 4],
    count: u32,
    tail_len: u32,
    _pad: [u32; 2],
    tail: [u32; MAX_TAIL_LEN / 4],
}

/// One counter dispatch: `count` names of `len` characters starting at the
/// mixed-radix digits `start` (FIRST_CHARS index, then REST_CHARS indices,
/// position len-1 fastest), each followed by `suffix`. This is
/// `CharNameIter::new(0, 1)`'s order within one length class.
pub struct CharWork {
    len: usize,
    start: [u32; MAX_CHARS],
    pub count: u32,
    suffix: Vec<u8>,
}

impl CharWork {
    /// Names of exactly `len` characters.
    pub fn class_size(len: usize) -> u128 {
        FIRST_CHARS.len() as u128 * (REST_CHARS.len() as u128).pow(len as u32 - 1)
    }

    pub fn new(len: usize, mut consumed: u128, count: u32, suffix: &[u8]) -> Self {
        let radix = REST_CHARS.len() as u128;
        let mut start = [0u32; MAX_CHARS];
        for p in (1..len).rev() {
            start[p] = (consumed % radix) as u32;
            consumed /= radix;
        }
        start[0] = consumed as u32;
        Self {
            len,
            start,
            count,
            suffix: suffix.to_vec(),
        }
    }

    fn template(&self) -> [u32; 36] {
        let mut msg = [0u8; RATE];
        let end = self.len + self.suffix.len();
        msg[self.len..end].copy_from_slice(&self.suffix);
        msg[end] = 0x01;
        msg[RATE - 1] |= 0x80;
        let mut words = [0u32; 36];
        for (i, &b) in msg.iter().enumerate() {
            words[i / 4] |= u32::from(b) << (8 * (i % 4));
        }
        words
    }

    /// The signature thread `idx` hashed. Mirrors the shader's digit
    /// arithmetic exactly. `None` means the index lies past the end of the
    /// length class, which the shader never reports.
    pub fn signature(&self, idx: u32) -> Option<Vec<u8>> {
        let radix = REST_CHARS.len() as u32;
        let mut name = vec![0u8; self.len];
        let mut carry = idx;
        for p in (1..self.len).rev() {
            let d = self.start[p] + carry % radix;
            carry = carry / radix + d / radix;
            name[p] = REST_CHARS[(d % radix) as usize];
        }
        name[0] = *FIRST_CHARS.get((self.start[0] + carry) as usize)?;
        name.extend_from_slice(&self.suffix);
        Some(name)
    }
}

/// One word dispatch: every camelCase name of 1..=MAX_WORDS dictionary words
/// in `WordNameIter` order, each followed by `suffix`.
pub struct WordWork {
    words: Vec<&'static str>,
    pub count: u32,
    suffix: Vec<u8>,
}

impl WordWork {
    pub fn new(words: &[&'static str], suffix: &[u8]) -> Self {
        let count = (1..=MAX_WORDS)
            .map(|k| words.len().pow(k as u32))
            .sum::<usize>() as u32;
        Self {
            words: words.to_vec(),
            count,
            suffix: suffix.to_vec(),
        }
    }

    /// The longest name in the dispatch.
    pub fn max_name_len(&self) -> usize {
        MAX_WORDS * self.words.iter().map(|w| w.len()).max().unwrap_or(0)
    }

    fn tail(&self) -> [u32; MAX_TAIL_LEN / 4] {
        let mut tail = [0u32; MAX_TAIL_LEN / 4];
        for (i, &b) in self.suffix.iter().chain(&[0x01]).enumerate() {
            tail[i / 4] |= u32::from(b) << (8 * (i % 4));
        }
        tail
    }

    /// The signature thread `idx` hashed. Mirrors the shader's index
    /// decoding exactly. `None` means the index lies past the end of the
    /// dispatch, which the shader never reports.
    pub fn signature(&self, idx: u32) -> Option<Vec<u8>> {
        let n = self.words.len();
        let mut rest = idx as usize;
        let mut size = 1;
        for k in 1..=MAX_WORDS {
            size *= n;
            if rest >= size {
                rest -= size;
                continue;
            }
            let mut idxs = [0usize; MAX_WORDS];
            for p in (0..k).rev() {
                idxs[p] = rest % n;
                rest /= n;
            }
            let mut name = [0u8; MAX_WORD_NAME_LEN];
            let len = camel_case(&self.words, &idxs[..k], &mut name);
            let mut sig = name[..len].to_vec();
            sig.extend_from_slice(&self.suffix);
            return Some(sig);
        }
        None
    }
}

pub enum Work {
    Chars(CharWork),
    Words(WordWork),
}

impl Work {
    pub fn count(&self) -> u32 {
        match self {
            Work::Chars(w) => w.count,
            Work::Words(w) => w.count,
        }
    }

    pub fn signature(&self, idx: u32) -> Option<Vec<u8>> {
        match self {
            Work::Chars(w) => w.signature(idx),
            Work::Words(w) => w.signature(idx),
        }
    }
}

struct Slot {
    counter_params: wgpu::Buffer,
    word_params: wgpu::Buffer,
    output: wgpu::Buffer,
    staging: wgpu::Buffer,
    counter_bind_group: wgpu::BindGroup,
    word_bind_group: wgpu::BindGroup,
}

/// A submitted dispatch whose results have not been read yet.
pub struct Pending {
    pub work: Work,
    index: wgpu::SubmissionIndex,
    rx: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
}

pub struct GpuHasher {
    device: wgpu::Device,
    queue: wgpu::Queue,
    counter_pipeline: wgpu::ComputePipeline,
    word_pipeline: wgpu::ComputePipeline,
    slots: Vec<Slot>,
}

fn entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

/// The shader's word table: (len, bytes 0-3, 4-7, 8-11) little-endian.
fn word_table(words: &[&str]) -> Vec<[u32; 4]> {
    words
        .iter()
        .map(|w| {
            let mut entry = [w.len() as u32, 0, 0, 0];
            for (i, &b) in w.as_bytes().iter().enumerate() {
                entry[1 + i / 4] |= u32::from(b) << (8 * (i % 4));
            }
            entry
        })
        .collect()
}

impl GpuHasher {
    pub fn new(adapter: &wgpu::Adapter) -> Result<Self, String> {
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("sigcrack"),
            ..Default::default()
        };
        let (device, queue) = pollster::block_on(adapter.request_device(&descriptor, None))
            .map_err(|e| e.to_string())?;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("keccak256"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = |label, entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: None,
                module: &shader,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let counter_pipeline = pipeline("counter", "main_counter");
        let word_pipeline = pipeline("words", "main_words");

        let word_table = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("words"),
            contents: bytemuck::cast_slice(&word_table(&crate::namegen::words())),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let output_size = ((1 + MAX_MATCHES) * 4) as u64;
        let uniform = |label, size: usize| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };

        // Each slot owns its uniforms so a submit never clobbers the params of
        // the dispatch still in flight.
        let slots = (0..SLOTS)
            .map(|_| {
                let counter_params = uniform("counter params", size_of::<CounterParams>());
                let word_params = uniform("word params", size_of::<WordParams>());
                let output = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("output"),
                    size: output_size,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let staging = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("staging"),
                    size: output_size,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let counter_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("counter"),
                    layout: &counter_pipeline.get_bind_group_layout(0),
                    entries: &[entry(3, &counter_params), entry(4, &output)],
                });
                let word_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("words"),
                    layout: &word_pipeline.get_bind_group_layout(0),
                    entries: &[
                        entry(5, &word_table),
                        entry(6, &word_params),
                        entry(7, &output),
                    ],
                });
                Slot {
                    counter_params,
                    word_params,
                    output,
                    staging,
                    counter_bind_group,
                    word_bind_group,
                }
            })
            .collect();

        Ok(Self {
            device,
            queue,
            counter_pipeline,
            word_pipeline,
            slots,
        })
    }

    /// Queues `work` on `slot` and returns without waiting. The slot must not
    /// have a dispatch pending.
    pub fn submit(&self, slot: usize, work: Work, target: Selector) -> Pending {
        let s = &self.slots[slot];
        let selector = target.map(u32::from);
        let (pipeline, bind_group) = match &work {
            Work::Chars(w) => {
                let params = CounterParams {
                    selector,
                    count: w.count,
                    len: w.len as u32,
                    _pad: [0; 2],
                    start: w.start,
                    tmpl: w.template(),
                };
                self.queue
                    .write_buffer(&s.counter_params, 0, bytemuck::bytes_of(&params));
                (&self.counter_pipeline, &s.counter_bind_group)
            }
            Work::Words(w) => {
                let params = WordParams {
                    selector,
                    count: w.count,
                    tail_len: w.suffix.len() as u32 + 1,
                    _pad: [0; 2],
                    tail: w.tail(),
                };
                self.queue
                    .write_buffer(&s.word_params, 0, bytemuck::bytes_of(&params));
                (&self.word_pipeline, &s.word_bind_group)
            }
        };

        let mut encoder = self.device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&s.output, 0, None);
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups(work.count().div_ceil(WORKGROUP_SIZE), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&s.output, 0, &s.staging, 0, s.staging.size());
        let index = self.queue.submit([encoder.finish()]);

        let (tx, rx) = mpsc::channel();
        s.staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
        Pending { work, index, rx }
    }

    /// Waits for `pending` and returns the matching thread indices in
    /// ascending order, so the first reported collision does not depend on
    /// the order the GPU appended them.
    pub fn complete(&self, slot: usize, pending: &Pending) -> Result<Vec<u32>, String> {
        let s = &self.slots[slot];
        // Waiting for this exact submission keeps the other slot's dispatch
        // running; `Maintain::Wait` would wait for the newest one instead.
        self.device
            .poll(wgpu::Maintain::wait_for(pending.index.clone()));
        pending
            .rx
            .recv()
            .map_err(|_| "gpu readback callback dropped".to_string())?
            .map_err(|e| format!("gpu buffer map failed: {e}"))?;

        let mut matches = {
            let data = s.staging.slice(..).get_mapped_range();
            let out: &[u32] = bytemuck::cast_slice(&data);
            let n = (out[0] as usize).min(MAX_MATCHES);
            out[1..=n].to_vec()
        };
        s.staging.unmap();
        matches.sort_unstable();
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hasher::scalar::keccak256_selector;
    use crate::namegen::{words, CharNameIter, WordNameIter};

    #[test]
    fn work_signature_matches_char_iter() {
        for len in 1..=3usize {
            let before: u128 = (1..len).map(CharWork::class_size).sum();
            let size = CharWork::class_size(len);
            let mut cases = vec![(0u128, 0u32), (0, 7), (size / 2, 7), (size - 1, 0)];
            if len > 1 {
                cases.extend([(62, 1), (0, 500), (size - 501, 500)]);
            }
            if len > 2 {
                cases.extend([(0, 4000), (size - 4001, 4000)]);
            }
            for (consumed, idx) in cases {
                let mut it = CharNameIter::new(0, 1);
                for _ in 0..(before + consumed + u128::from(idx)) {
                    assert!(it.advance());
                }
                let mut expected = it.current().to_vec();
                expected.extend_from_slice(b"()");
                let work = CharWork::new(len, consumed, 1, b"()");
                assert_eq!(
                    work.signature(idx),
                    Some(expected),
                    "len {len} {consumed}+{idx}"
                );
            }
            assert_eq!(CharWork::new(len, size - 1, 2, b"()").signature(1), None);
        }
    }

    #[test]
    fn word_signature_matches_word_iter() {
        let words = words();
        let n = words.len() as u32;
        let work = WordWork::new(&words, b"(uint256)");
        assert_eq!(work.count, n + n * n + n * n * n);
        assert!(work.count.div_ceil(WORKGROUP_SIZE) <= MAX_WORKGROUPS);
        let mut it = WordNameIter::new(words);
        let mut pos = 0;
        for idx in [
            0,
            n - 1,
            n,
            n + 1,
            n + n * n - 1,
            n + n * n,
            n + n * n + 1,
            work.count - 1,
        ] {
            while pos < idx {
                assert!(it.advance());
                pos += 1;
            }
            let mut expected = it.current().to_vec();
            expected.extend_from_slice(b"(uint256)");
            assert_eq!(work.signature(idx), Some(expected), "{idx}");
        }
        assert!(!it.advance());
        assert_eq!(work.signature(work.count), None);
    }

    #[test]
    fn gpu_words_finds_names() {
        let Some(gpu) = crate::gpu::detect().into_iter().next() else {
            return;
        };
        let hasher = GpuHasher::new(&gpu.adapter).unwrap();
        let words = words();
        let last = words.len() - 1;
        let name = |idx: &[usize]| {
            let mut buf = [0u8; MAX_WORD_NAME_LEN];
            let len = camel_case(&words, idx, &mut buf);
            buf[..len].to_vec()
        };
        let cases: [(Vec<u8>, &[u8]); 3] = [
            (name(&[0]), b"()"),
            (name(&[0, 1]), b"(uint256)"),
            (name(&[last, 0, last]), b"(address,bytes32,uint8)"),
        ];
        for (name, suffix) in cases {
            let mut input = name.clone();
            input.extend_from_slice(suffix);
            let target = keccak256_selector(&input);
            let work = Work::Words(WordWork::new(&words, suffix));
            let pending = hasher.submit(0, work, target);
            let found: Vec<Vec<u8>> = hasher
                .complete(0, &pending)
                .unwrap()
                .into_iter()
                .map(|idx| pending.work.signature(idx).unwrap())
                .collect();
            assert!(found.contains(&input), "{input:?} not found");
            for f in &found {
                assert_eq!(keccak256_selector(f), target, "{f:?}");
            }
        }
    }

    #[test]
    fn gpu_counter_finds_names() {
        let Some(gpu) = crate::gpu::detect().into_iter().next() else {
            return;
        };
        let hasher = GpuHasher::new(&gpu.adapter).unwrap();
        let inputs: [&[u8]; 3] = [b"ab()", b"Zz9_()", b"_9Zaz()"];
        // Other names may share the selector by chance, so every reported
        // match is checked and the search stops once the input shows up.
        for input in inputs {
            let target = keccak256_selector(input);
            let mut found = Vec::new();
            'search: for len in 1..=MAX_CHARS {
                let total = CharWork::class_size(len);
                let mut consumed = 0u128;
                while consumed < total {
                    let count = (total - consumed).min(u128::from(DISPATCH)) as u32;
                    let work = CharWork::new(len, consumed, count, b"()");
                    consumed += u128::from(count);
                    let pending = hasher.submit(0, Work::Chars(work), target);
                    for idx in hasher.complete(0, &pending).unwrap() {
                        found.push(pending.work.signature(idx).unwrap());
                    }
                    if found.iter().any(|f| f == input) {
                        break 'search;
                    }
                }
            }
            assert!(found.iter().any(|f| f == input), "{input:?} not found");
            for f in &found {
                assert_eq!(keccak256_selector(f), target, "{f:?}");
            }
        }
    }
}
