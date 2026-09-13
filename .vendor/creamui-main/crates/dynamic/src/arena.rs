//! Keeps widget-callback closures (boxed once per rebuilt frame, since a
//! fresh widget tree — and fresh closures for its click/change handlers —
//! is built every repaint) alive exactly as long as the C ABI might still
//! call into them, then frees them.
//!
//! A widget tree built this frame stays the *live*, event-addressable scene
//! on the other side of the ABI for the entire span until the *next*
//! frame's tree replaces it — including while one of its own callbacks is
//! still on the call stack triggering that very rebuild (click -> signal
//! write -> reactive rebuild, all synchronous, all before that callback
//! invocation returns). So this always keeps the current and previous
//! frame's closures and only ever frees anything older, which is exactly
//! what's guaranteed to no longer be reachable from the ABI side by the
//! time a non-reentrant rebuild finishes (widget trees are only ever built
//! from the top-level render loop, never from within a callback that's
//! itself still being constructed, so at most two generations are ever
//! live at once).

use std::any::Any;
use std::cell::RefCell;
use std::collections::VecDeque;

pub(crate) struct ClosureArena {
    generations: RefCell<VecDeque<Vec<Box<dyn Any>>>>,
}

impl ClosureArena {
    pub(crate) fn new() -> Self {
        ClosureArena {
            generations: RefCell::new(VecDeque::new()),
        }
    }

    pub(crate) fn begin_frame(&self) {
        self.generations.borrow_mut().push_back(Vec::new());
    }

    pub(crate) fn end_frame(&self) {
        let mut generations = self.generations.borrow_mut();
        while generations.len() > 2 {
            generations.pop_front();
        }
    }

    /// Boxes `value`, keeps it alive in the current frame's generation, and
    /// returns a stable raw pointer to it for use as FFI `userdata`.
    pub(crate) fn keep<T: 'static>(&self, value: T) -> *mut T {
        let ptr = Box::into_raw(Box::new(value));
        let mut generations = self.generations.borrow_mut();
        let current = generations
            .back_mut()
            .expect("ClosureArena::keep called outside begin_frame/end_frame");
        current.push(unsafe { Box::from_raw(ptr) } as Box<dyn Any>);
        ptr
    }
}
