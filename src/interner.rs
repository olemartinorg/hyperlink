use bumpalo::Bump;
use std::collections::HashSet;
use std::sync::Mutex;

/// A super-basic string interning helper based on arenas.
#[derive(Default)]
pub struct StringInterner {
    arena: Bump,
    map: Mutex<HashSet<&'static str>>,
}

impl StringInterner {
    pub fn intern_string<'a, 'b>(&'a self, s: &'b str) -> &'a str {
        let mut map = self.map.lock().unwrap();
        match map.get(&*s) {
            Some(rv) => rv,
            None => {
                let rv = self.arena.alloc_str(s);
                map.insert(unsafe {
                    // SAFETY: Our hashset's keys have a "static" lifetime which is fine for as
                    // long as we don't destructure the StringInterner and drop the arena without
                    // dropping the map, and generally for as long as we don't leak static
                    // lifetimes to the outside (such as when returning a &'static str)
                    //
                    // this next line just transmutes the 'a lifetime to 'static
                    &*(rv as *mut str as *const str)
                });
                rv
            }
        }
    }

    pub fn get_arena(&self) -> &Bump {
        &self.arena
    }
}
