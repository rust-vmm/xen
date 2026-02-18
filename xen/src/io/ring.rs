//! Xen paravirtualized ring buffer utilities (for PV Console and XenStore)
use core::{
    hint::{likely, unlikely},
    sync::atomic::{AtomicU32, Ordering},
};

use volatile::VolatilePtr;

#[derive(Clone, Copy, Debug)]
pub struct XenRing<'a> {
    pub ring: VolatilePtr<'a, [u8]>,
    pub cons: &'a AtomicU32,
    pub prod: &'a AtomicU32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XenRingError {
    /// Data is too large to fit in the ring buffer.
    TooLarge,
    /// Consumer is not ready for receiving the payload
    NotReady,
    /// Misbehaving ring index
    MisbehavingIndex,
}

#[inline(always)]
fn available(prod: usize, cons: usize, len: usize) -> usize {
    len - queued(prod, cons, len) - 1
}

#[inline(always)]
fn queued(prod: usize, cons: usize, len: usize) -> usize {
    if prod < cons {
        len - 1
    } else {
        (prod - cons).min(len)
    }
}

impl XenRing<'_> {
    pub fn capacity(&self) -> usize {
        self.ring.len() - 1
    }

    pub fn available(&self) -> usize {
        let cons = self.cons.load(Ordering::Acquire) as usize % self.ring.len();
        let prod = self.prod.load(Ordering::Acquire) as usize % self.ring.len();

        available(prod, cons, self.ring.len())
    }

    pub fn queued(&self) -> usize {
        let cons = self.cons.load(Ordering::Acquire) as usize % self.ring.len();
        let prod = self.prod.load(Ordering::Acquire) as usize % self.ring.len();

        queued(prod, cons, self.ring.len())
    }

    pub fn read(&mut self, buffer: &mut [u8], exact: bool) -> Result<usize, XenRingError> {
        // FIXME: Fix indexes (see write)

        if exact && buffer.len() >= self.ring.len() {
            return Err(XenRingError::TooLarge);
        }

        let cons = self.cons.load(Ordering::Acquire) as usize % self.ring.len();
        let prod = self.prod.load(Ordering::Acquire) as usize % self.ring.len();

        let queued = queued(prod, cons, self.ring.len());

        if unlikely(exact && queued < buffer.len()) {
            return Err(XenRingError::NotReady);
        }

        let to_read = queued.min(buffer.len());
        let dest_cons = (cons + to_read) % self.ring.len();

        let buffer = &mut buffer[..to_read];

        if likely(cons <= dest_cons) {
            self.ring
                .index(cons..dest_cons)
                .copy_into_slice(&mut buffer[..to_read]);
        } else {
            /*
             * Split the buffer in two parts, one that will be copied from
             * the end of the ring buffer, another from the beginning.
             *
             * [(parts.1)C    P(parts.0)]
             */

            let parts = buffer.split_at_mut(self.ring.len() - cons);
            self.ring.index(cons..).copy_into_slice(parts.0);
            self.ring.index(..dest_cons).copy_into_slice(parts.1);
        }

        self.cons
            .compare_exchange(
                cons as u32,
                dest_cons as u32,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .map_err(|_| XenRingError::MisbehavingIndex)?;

        Ok(to_read)
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<(), XenRingError> {
        if unlikely(buffer.len() >= self.ring.len()) {
            return Err(XenRingError::TooLarge);
        }

        let cons = self.cons.load(Ordering::Acquire) as usize;
        let prod = self.prod.load(Ordering::Acquire) as usize;

        let prod_idx = prod % self.ring.len();

        let dest_prod = prod + buffer.len();
        let dest_prod_idx = dest_prod % self.ring.len();

        if unlikely(available(prod, cons, self.ring.len()) < buffer.len()) {
            return Err(XenRingError::NotReady);
        }

        if likely(prod_idx <= dest_prod_idx) {
            self.ring
                .index(prod_idx..dest_prod_idx)
                .copy_from_slice(buffer);
        } else {
            /*
             * Split the buffer in two parts, one that will be copied at
             * the end of the ring buffer, another at the beginning.
             *
             * [(parts.1)C    P(parts.0)]
             */

            let parts = buffer.split_at(self.ring.len() - prod_idx);
            self.ring.index(prod_idx..).copy_from_slice(parts.0);
            self.ring.index(..dest_prod_idx).copy_from_slice(parts.1);
        }

        self.prod
            .compare_exchange(
                prod as u32,
                dest_prod as u32,
                Ordering::Release,
                Ordering::Relaxed,
            )
            .map_err(|_| XenRingError::MisbehavingIndex)?;

        Ok(())
    }
}
