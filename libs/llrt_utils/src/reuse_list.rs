use std::fmt::Debug;

#[derive(Default, Clone)]
pub struct ReuseList<T> {
    items: Vec<Option<T>>,
    slots: Vec<usize>,
    last_slot_idx: usize,
    size: usize,
    slot_size: usize,
}

impl<T: Debug> Debug for ReuseList<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReuseList")
            .field("items", &self.items)
            .field("slots", &self.slots)
            .finish()
    }
}

impl<T> ReuseList<T> {
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    //create a with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            items: Vec::with_capacity(capacity),
            slots: Vec::with_capacity(capacity >> 2),
            last_slot_idx: 0,
            size: 0,
            slot_size: 0,
        }
    }

    pub fn append(&mut self, item: T) -> usize {
        if self.slot_size > 0 {
            //reuse empty slot if valid
            let slot = self.slots[self.last_slot_idx - 1];
            if slot > 0 {
                self.items[slot - 1] = Some(item);
                self.slots[self.last_slot_idx - 1] = 0;
                if self.last_slot_idx > 1 {
                    self.last_slot_idx -= 1;
                }

                self.size += 1;
                return slot - 1;
            }
        }
        //no valid empty slots, append to end
        self.items.push(Some(item));
        self.size += 1;
        self.items.len() - 1
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }

        let item = self.items[index].take();
        if item.is_some() {
            if self.slot_size > 0 && self.slots[self.last_slot_idx - 1] == 0 {
                self.slots[self.last_slot_idx - 1] = index + 1;
            } else {
                self.slots.push(index + 1);
                self.last_slot_idx += 1;
                self.slot_size += 1;
            }
            self.size -= 1;
        }
        item
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.items.len() {
            None
        } else {
            self.items[index].as_ref()
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.items.len() {
            None
        } else {
            self.items[index].as_mut()
        }
    }

    pub fn capacity(&self) -> usize {
        self.items.capacity()
    }

    pub fn len(&self) -> usize {
        self.size
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.items.iter().filter_map(|x| x.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.items.iter_mut().filter_map(|x| x.as_mut())
    }

    //implement clear
    pub fn clear(&mut self) {
        self.items.clear();
        self.slots.clear();
        self.last_slot_idx = 0;
        self.size = 0;
        self.slot_size = 0;
    }

    pub fn optimize(&mut self) {
        let mut new_items = Vec::with_capacity(self.size);

        for item in self.items.iter_mut() {
            let a = item.take();
            if a.is_some() {
                new_items.push(a);
            }
        }
        self.items = new_items;
        self.slots.clear();
        self.last_slot_idx = 0;
        self.slot_size = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let list: ReuseList<i32> = ReuseList::new();
        assert_eq!(list.len(), 0);
        assert_eq!(list.capacity(), 0);
        assert_eq!(list.items.len(), 0);
        assert_eq!(list.slots.len(), 0);
    }

    #[test]
    fn test_with_capacity() {
        let list: ReuseList<i32> = ReuseList::with_capacity(10);
        assert_eq!(list.len(), 0);
        assert_eq!(list.capacity(), 10);
        assert_eq!(list.items.len(), 0);
        assert_eq!(list.slots.len(), 0);
    }

    #[test]
    fn test_append() {
        let mut list = ReuseList::new();
        assert_eq!(list.append(1), 0);
        assert_eq!(list.append(2), 1);
        assert_eq!(list.append(3), 2);
        assert_eq!(list.len(), 3);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![1, 2, 3]);
        assert_eq!(list.items, vec![Some(1), Some(2), Some(3)]);
        assert_eq!(list.slots, vec![]);
    }

    #[test]
    fn test_remove() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);
        list.append(3);

        assert_eq!(list.remove(1), Some(2));
        assert_eq!(list.len(), 2);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![1, 3]);
        assert_eq!(list.items, vec![Some(1), None, Some(3)]);
        assert_eq!(list.slots, vec![2]);

        assert_eq!(list.remove(5), None);
    }

    #[test]
    fn test_reuse_slots() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);
        list.append(3);

        list.remove(1); // Remove 2
        assert_eq!(list.append(4), 1); // Should reuse index 1

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![1, 4, 3]);
        assert_eq!(list.items, vec![Some(1), Some(4), Some(3)]);
        assert_eq!(list.slots, vec![0]);
    }

    #[test]
    fn test_get() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);

        assert_eq!(list.get(0), Some(&1));
        assert_eq!(list.get(1), Some(&2));
        assert_eq!(list.get(2), None);
        assert_eq!(list.items, vec![Some(1), Some(2)]);
        assert_eq!(list.slots, vec![]);
    }

    #[test]
    fn test_get_mut() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);

        if let Some(value) = list.get_mut(0) {
            *value = 10;
        }

        assert_eq!(list.get(0), Some(&10));
        assert_eq!(list.items, vec![Some(10), Some(2)]);
        assert_eq!(list.slots, vec![]);
    }

    #[test]
    fn test_iter() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);
        list.append(3);
        list.remove(1);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![1, 3]);
        assert_eq!(list.items, vec![Some(1), None, Some(3)]);
        assert_eq!(list.slots, vec![2]);
    }

    #[test]
    fn test_iter_mut() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);
        list.append(3);

        for item in list.iter_mut() {
            *item *= 2;
        }

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![2, 4, 6]);
        assert_eq!(list.items, vec![Some(2), Some(4), Some(6)]);
        assert_eq!(list.slots, vec![]);
    }

    #[test]
    fn test_multiple_removes() {
        let mut list = ReuseList::new();
        for i in 0..5 {
            list.append(i);
        }

        list.remove(1);
        list.remove(3);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![0, 2, 4]);
        assert_eq!(list.items, vec![Some(0), None, Some(2), None, Some(4)]);
        assert_eq!(list.slots, vec![2, 4]);

        // Test reuse of both slots
        list.append(10);
        list.append(11);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![0, 11, 2, 10, 4]);
        assert_eq!(
            list.items,
            vec![Some(0), Some(11), Some(2), Some(10), Some(4)]
        );
        assert_eq!(list.slots, vec![0, 0]);

        list.remove(0);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![11, 2, 10, 4]);
        assert_eq!(list.items, vec![None, Some(11), Some(2), Some(10), Some(4)]);
        assert_eq!(list.slots, vec![1, 0]);

        list.append(20);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![20, 11, 2, 10, 4]);
        assert_eq!(
            list.items,
            vec![Some(20), Some(11), Some(2), Some(10), Some(4)]
        );
        assert_eq!(list.slots, vec![0, 0]);

        //remove all items
        list.clear();
    }

    #[test]
    fn test_optimize() {
        let mut list = ReuseList::new();
        list.append(1);
        list.append(2);
        list.append(3);
        list.remove(1);

        assert_eq!(list.items, vec![Some(1), None, Some(3)]);
        assert_eq!(list.slots, vec![2]);

        list.optimize();

        assert_eq!(list.items, vec![Some(1), Some(3)]);
        assert_eq!(list.slots, vec![]);
        assert_eq!(list.last_slot_idx, 0);
        assert_eq!(list.slot_size, 0);

        let items: Vec<i32> = list.iter().cloned().collect();
        assert_eq!(items, vec![1, 3]);
    }
}
