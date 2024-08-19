use std::{borrow::Borrow, mem::ManuallyDrop};

use bumpalo_herd::Herd;
use parking_lot::Mutex;

/// A object pool to reuse Herds through generations
pub struct HerdPool(Mutex<Vec<Herd>>);

pub struct RentedHerd<'hp> {
    herd: ManuallyDrop<Herd>,
    owner: &'hp HerdPool,
}

impl HerdPool {
    pub fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    /// Rent a herd from the pool. If the pool is empty, a new herd will be created.
    pub fn rent(&self) -> RentedHerd {
        let mut arena = self.0.lock();
        let herd = arena.pop().unwrap_or_default();
        RentedHerd {
            herd: ManuallyDrop::new(herd),
            owner: &self,
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

        self.owner.0.lock().push(member);
    }
}
