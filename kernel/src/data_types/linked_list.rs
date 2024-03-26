use core::marker::PhantomData;

#[derive(Copy, Clone)]
pub struct LinkedList {
    head: *mut usize,
}

pub struct ListNode {
    prev: *mut usize,
    curr: *mut usize,
}

unsafe impl Send for LinkedList {}

impl LinkedList {
    pub const fn new() -> Self {
        LinkedList {
            head: core::ptr::null_mut(),
        }
    }

    pub fn push(&mut self, node: *mut usize) {
        unsafe {
            *node = self.head as usize;
            self.head = node;
        }
    }

    pub fn pop(&mut self) -> Option<*mut usize> {
        match self.is_empty() {
            true => None,
            false => {
                // Advance head pointer
                let item = self.head;
                self.head = unsafe { *item as *mut usize };
                Some(item)
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    pub fn iter(&self) -> Iter {
        Iter {
            current: self.head,
            list: PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            prev: &mut self.head as *mut *mut usize as *mut usize,
            curr: self.head,
            list: PhantomData,
        }
    }
}

pub struct Iter<'a> {
    current: *mut usize,
    list: PhantomData<&'a LinkedList>,
}

impl Iterator for Iter<'_> {
    type Item = *mut usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_null() {
            None
        } else {
            let item = self.current;
            let next = unsafe { *item as *mut usize };
            self.current = next;
            Some(item)
        }
    }
}

pub struct IterMut<'a> {
    list: PhantomData<&'a mut LinkedList>,
    prev: *mut usize,
    curr: *mut usize,
}

impl<'a> Iterator for IterMut<'a> {
    type Item = ListNode;

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr.is_null() {
            None
        } else {
            let res = ListNode {
                prev: self.prev,
                curr: self.curr,
            };
            self.prev = self.curr;
            self.curr = unsafe { *self.curr as *mut usize };
            Some(res)
        }
    }
}

impl ListNode {
    pub fn pop(self) -> *mut usize {
        unsafe {
            *(self.prev) = *(self.curr);
        }
        self.curr
    }

    pub fn value(&self) -> *mut usize {
        self.curr
    }
}
