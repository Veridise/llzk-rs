use std::{
    alloc::{alloc, dealloc, Layout, LayoutError},
    any::TypeId,
    ops::{Deref, Range},
    ptr,
};

#[derive(Copy, Clone, Debug)]
pub struct Index(usize);

impl<T: Into<usize>> From<T> for Index {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Deref for Index {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct Item {
    ptr: *mut u8,
    id: TypeId,
}

const PAGE_SIZE: usize = 1024 * 4;

struct Page {
    ptr: *mut u8,
    layout: Layout,
}

impl Page {
    fn new() -> Result<Self, LayoutError> {
        let layout = Layout::array::<u8>(PAGE_SIZE)?;
        let ptr = unsafe { alloc(layout) };
        Ok(Self { layout, ptr })
    }

    fn end(&self) -> *mut u8 {
        unsafe { self.ptr.byte_add(self.layout.size()) }
    }

    fn range(&self) -> Range<*mut u8> {
        self.ptr..self.end()
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr, self.layout) }
    }
}

pub struct BumpArena {
    items: Vec<Item>,
    pages: Vec<Page>,
    cursor: *mut u8,
}

unsafe impl Send for BumpArena {}

const ITEM_DEFAULT_CAP: usize = 10;

impl BumpArena {
    pub fn new() -> Self {
        let page = Page::new().expect("page allocation");
        let cursor = page.ptr;
        Self {
            items: Vec::with_capacity(ITEM_DEFAULT_CAP),
            pages: vec![page],
            cursor,
        }
    }

    fn current_page(&self) -> &Page {
        self.pages.last().unwrap()
    }

    /// Returns the free space in the current page
    fn free_space(&self) -> usize {
        let page = self.current_page();
        assert!(
            page.range().contains(&self.cursor),
            "cursor is out of page range"
        );
        unsafe { page.end().offset_from_unsigned(self.cursor) }
    }

    fn object_fits(&self, layout: Layout) -> bool {
        self.free_space() >= layout.size()
    }

    fn new_page(&mut self) {
        let page = Page::new().expect("page allocation");
        self.cursor = page.ptr;
        self.pages.push(page);
    }

    fn allocate_object<T: 'static>(&mut self) -> Item {
        let layout = Layout::new::<T>();
        if !self.object_fits(layout) {
            self.new_page();
        }
        let ptr = self.cursor;
        self.cursor = unsafe { self.cursor.byte_add(layout.size()) };
        let id = TypeId::of::<T>();
        Item { ptr, id }
    }

    pub fn insert<T: 'static>(&mut self, value: T) -> Index {
        let item = self.allocate_object::<T>();
        unsafe { ptr::write(item.ptr as *mut T, value) }
        let index = self.items.len();
        self.items.push(item);

        index.into()
    }

    pub fn get<T: 'static>(&self, idx: &Index) -> &T {
        let item = &self.items[idx.0];
        assert_eq!(TypeId::of::<T>(), item.id);
        unsafe { (item.ptr as *const T).as_ref().unwrap() }
    }

    pub fn try_get<T: 'static>(&self, idx: &Index) -> Option<&T> {
        let item = self.items.get(idx.0)?;
        if TypeId::of::<T>() != item.id {
            return None;
        }
        Some(unsafe { (item.ptr as *const T).as_ref().unwrap() })
    }

    #[allow(dead_code)]
    pub fn contains<T: 'static>(&self, idx: &Index) -> bool {
        self.items.len() > idx.0 && self.items[idx.0].id == TypeId::of::<T>()
    }
}

impl Default for BumpArena {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_retrieve() {
        let mut arena = BumpArena::new();
        let idx0 = arena.insert(60usize);
        let idx1 = arena.insert(80u32);

        let val0: usize = *arena.get(&idx0);
        let val1: u32 = *arena.get(&idx1);

        assert_eq!(val0, 60usize);
        assert_eq!(val1, 80u32);
    }

    #[test]
    fn test_insert_and_retrieve_wrong_type() {
        let mut arena = BumpArena::new();
        let idx1 = arena.insert(80u32);

        let val0: Option<&usize> = arena.try_get(&idx1);

        assert_eq!(val0, None);
    }
}
