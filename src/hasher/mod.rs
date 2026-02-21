pub mod scalar;
pub mod avx2;
pub mod avx512;
pub mod gpu;

pub type Selector = [u8; 4];

#[derive(Debug, Clone, Copy)]
pub enum SimdWidth {
    Scalar,
    Avx2,
    Avx512,
}

impl SimdWidth {
    pub fn lanes(&self) -> usize {
        match self { SimdWidth::Scalar => 1, SimdWidth::Avx2 => 4, SimdWidth::Avx512 => 8 }
    }
    pub fn name(&self) -> &str {
        match self { SimdWidth::Scalar => "scalar", SimdWidth::Avx2 => "AVX2 (4-way)", SimdWidth::Avx512 => "AVX-512 (8-way)" }
    }
}
