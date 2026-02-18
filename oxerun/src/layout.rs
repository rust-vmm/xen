// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2022 Akira Moroo

use core::ops::Range;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub enum MemoryAttribute {
    Code,
    Data,
    Unusable,
    Mmio,
}

#[derive(Clone, Copy)]
pub struct MemoryDescriptor {
    #[allow(dead_code)]
    pub name: &'static str,
    pub range: fn() -> Range<usize>,
    pub attribute: MemoryAttribute,
}

impl MemoryDescriptor {
    pub const PAGE_SIZE: usize = 0x1000;

    pub fn range_start(&self) -> usize {
        let addr = (self.range)().start;
        assert!(addr.is_multiple_of(Self::PAGE_SIZE));
        addr
    }

    pub fn range_end(&self) -> usize {
        let addr = (self.range)().end;
        assert!(addr.is_multiple_of(Self::PAGE_SIZE));
        addr
    }

    pub fn page_count(&self) -> usize {
        (self.range_end() - self.range_start()) / Self::PAGE_SIZE
    }
}

pub type MemoryLayout<const NUM_MEM_DESCS: usize> = [MemoryDescriptor; NUM_MEM_DESCS];
