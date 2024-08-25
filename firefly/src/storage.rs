use std::{ops::DerefMut, time::Duration};

use parking_lot::{Mutex, MutexGuard};
use rand::prelude::*;

/// Represents a rack that contains multiple shelves.
#[derive(Debug)]
pub struct Rack<T> {
    shelves: Vec<Shelf<T>>,
}

/// Represents a shelf that holds data of type `T`.
#[derive(Debug)]
pub struct Shelf<T> {
    data: Mutex<Vec<T>>,
}

/// Represents an index that points to a specific location in the rack.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Index {
    pub shelf: usize,
    pub slot: usize,
}

/// Represents a range of indices within a shelf.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct IndexRange {
    pub shelf: usize,
    pub start: usize,
    pub end: usize,
}

/// Represents a reference to a shelf's data.
#[derive(Debug)]
pub struct ShelfRef<'a, T> {
    data: MutexGuard<'a, Vec<T>>,
    pub shelf: usize,
}

impl<T> Rack<T> {
    /// Creates a new rack with the specified number of shelves.
    pub fn new(num_shelves: u32) -> Self {
        Self {
            shelves: (0..num_shelves).into_iter().map(|_| Shelf::new()).collect(),
        }
    }

    /// Allocates a single item to a random shelf and returns its index.
    pub fn alloc(&self, item: T) -> Index {
        // Use random number generator for now
        let shelf = thread_rng().gen_range(0..self.shelves.len());

        Index {
            shelf: shelf,
            slot: self.shelves[shelf].alloc(item),
        }
    }

    /// Rents a random shelf and returns a reference to its data.
    pub fn rent_shelf(&self) -> ShelfRef<'_, T> {
        let shelf = thread_rng().gen_range(0..self.shelves.len());
        ShelfRef {
            data: self.shelves[shelf].rent(),
            shelf,
        }
    }

    /// Returns a mutable reference to the item at the specified index.
    pub fn get(&self, index: Index) -> impl DerefMut<Target = T> + '_ {
        let shelf = &self.shelves[index.shelf];
        let data = shelf
            .data
            .try_lock_for(Duration::from_secs(1))
            .expect(format!("Rack::get deadlock {:?}", index).as_str());
        MutexGuard::map(data, |data| &mut data[index.slot])
    }

    /// Returns a mutable reference to the items within the specified range.
    pub fn get_range(&self, range: IndexRange) -> impl DerefMut<Target = [T]> + '_ {
        let shelf = &self.shelves[range.shelf];
        let data = shelf
            .data
            .try_lock_for(Duration::from_secs(1))
            .expect(format!("Rack::get_range deadlock {:?}", range).as_str());
        MutexGuard::map(data, |data| &mut data[range.start..range.end])
    }

    pub fn len(&self) -> usize {
        self.shelves
            .iter()
            .map(|shelf| shelf.data.lock().len())
            .sum()
    }
}

impl<T> Shelf<T> {
    /// Creates a new shelf with an empty data vector.
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Vec::new()),
        }
    }

    /// Allocates an item to the shelf and returns its slot index.
    pub fn alloc(&self, item: T) -> usize {
        let mut data = self
            .data
            .try_lock_for(Duration::from_secs(1))
            .expect("Shelf::alloc deadlock");
        data.push(item);
        data.len() - 1
    }

    /// Rents the data vector of the shelf.
    pub fn rent(&self) -> MutexGuard<'_, Vec<T>> {
        self.data
            .try_lock_for(Duration::from_secs(1))
            .expect("Shelf::rent deadlock")
    }
}

impl<T> ShelfRef<'_, T> {
    /// Appends an item to the shelf's data vector and returns its index.
    pub fn append(&mut self, item: T) -> Index {
        self.data.push(item);
        Index {
            shelf: self.shelf,
            slot: self.data.len() - 1,
        }
    }

    /// Appends a vector of items to the shelf's data vector and returns the range of indices.
    pub fn append_vec(&mut self, vec: Vec<T>) -> IndexRange {
        let len = vec.len();
        self.data.extend(vec);

        IndexRange {
            shelf: self.shelf,
            start: self.data.len() - len,
            end: self.data.len(),
        }
    }

    /// Modifies the item at the specified index using the provided function.
    pub fn modify(&mut self, index: Index, f: impl FnOnce(&mut T)) {
        assert!(index.shelf == self.shelf);
        f(&mut self.data[index.slot]);
    }

    pub fn get(&self, index: Index) -> &T {
        assert!(index.shelf == self.shelf);
        &self.data[index.slot]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc() {
        let rack: Rack<u32> = Rack::new(3);
        let index = rack.alloc(42);
        let data = rack.get(index);
        assert_eq!(*data, 42);
    }

    #[test]
    fn test_rent_shelf() {
        let rack: Rack<u32> = Rack::new(3);
        let shelf_ref = rack.rent_shelf();
        let data = shelf_ref.data;
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_append() {
        let rack: Rack<u32> = Rack::new(3);
        let mut shelf_ref = rack.rent_shelf();
        let index = shelf_ref.append(42);
        drop(shelf_ref);

        let data = rack.get(index);
        assert_eq!(*data, 42);
    }

    #[test]
    fn test_append_vec() {
        let rack: Rack<u32> = Rack::new(3);
        let mut shelf_ref = rack.rent_shelf();
        let vec = vec![1, 2, 3];
        let range = shelf_ref.append_vec(vec);
        drop(shelf_ref);

        let data = rack.get_range(range);
        assert_eq!(*data, [1, 2, 3]);
    }
}
