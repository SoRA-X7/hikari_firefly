use std::{
    borrow::Borrow,
    mem::ManuallyDrop,
    sync::{Arc, Weak},
};

use bumpalo_herd::Herd;
use parking_lot::Mutex;

/// A object pool to reuse Herds through generations
#[derive(Clone)]
pub struct HerdPool(Arc<HerdPoolInner>);

pub struct HerdPoolInner {
    arena: Mutex<Vec<Herd>>,
}

pub struct RentedHerd<'hp> {
    herd: ManuallyDrop<Herd>,
    owner: &'hp HerdPoolInner,
}

impl HerdPool {
    pub fn new() -> Self {
        Self(Arc::new(HerdPoolInner {
            arena: Mutex::new(Vec::new()),
        }))
    }

    /// Rent a herd from the pool. If the pool is empty, a new herd will be created.
    pub fn rent(&self) -> RentedHerd {
        let mut arena = self.0.arena.lock();
        let herd = arena.pop().unwrap_or_default();
        RentedHerd {
            herd: ManuallyDrop::new(herd),
            owner: &self.0,
        }
    }
}

impl Borrow<Herd> for RentedHerd<'_> {
    fn borrow(&self) -> &Herd {
        &self.herd
    }
}

impl Drop for RentedHerd<'_> {
    fn drop(&mut self) {
        self.herd.reset();

        // We will reuse the herd, so we don't want to drop it right now
        // Safety: the herd will be dropped by the pool
        let member = unsafe { ManuallyDrop::take(&mut self.herd) };

        self.owner.arena.lock().push(member);
    }
}
