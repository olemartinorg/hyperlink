use std::collections::BTreeMap;
use std::mem;

use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct Trie<T> {
    value: Option<T>,
    label: SmallVec<[u8; 12]>,
    lower_than: BTreeMap<usize, Trie<T>>,
    bigger_than: BTreeMap<usize, Trie<T>>,
}

impl<T> Default for Trie<T> {
    fn default() -> Self {
        Trie {
            value: None,
            label: SmallVec::new(),
            lower_than: BTreeMap::new(),
            bigger_than: BTreeMap::new(),
        }
    }
}

macro_rules! impl_get {
    ({ $($ref:tt)* }, $slf:ident, $key:ident, $get:ident, $as_ref:ident) => {{
        let key = $key.as_ref();

        if key.is_empty() {
            return $slf.value.$as_ref();
        }

        let mut diverge_at = 0;
        let mut is_lower_than = false;
        for (a, b) in key.iter().zip($slf.label.iter()) {
            if a != b {
                is_lower_than = a < b;
                break;
            }

            diverge_at += 1;
        }

        let children = if is_lower_than {
            $($ref)* $slf.lower_than
        } else {
            $($ref)* $slf.bigger_than
        };

        children.$get(&diverge_at)?.$get(&key[diverge_at..])
    }}
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, key: impl AsRef<[u8]>, value: T) -> Option<T> {
        let key = key.as_ref();

        if key.is_empty() {
            return mem::replace(&mut self.value, Some(value));
        }

        let mut diverge_at = 0;
        let mut is_lower_than = false;
        for (a, b) in key.iter().zip(self.label.iter()) {
            if a != b {
                is_lower_than = a < b;
                break;
            }

            diverge_at += 1;
        }

        let next_key = if diverge_at == self.label.len() {
            self.label.extend_from_slice(&key[diverge_at..]);
            diverge_at = key.len();
            b""
        } else {
            key.get(diverge_at..).unwrap_or(b"")
        };

        let children = if is_lower_than {
            &mut self.lower_than
        } else {
            &mut self.bigger_than
        };

        children
            .entry(diverge_at)
            .or_insert_with(Trie::new)
            .insert(next_key, value)
    }

    pub fn get(&self, key: impl AsRef<[u8]>) -> Option<&T> {
        impl_get!({ & }, self, key, get, as_ref)
    }

    pub fn get_mut(&mut self, key: impl AsRef<[u8]>) -> Option<&mut T> {
        impl_get!({ &mut }, self, key, get_mut, as_mut)
    }
}

impl<T> IntoIterator for Trie<T> {
    type IntoIter = IntoIter<T>;
    type Item = <IntoIter<T> as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            next_tries: vec![],
            current_trie: Some((vec![], self)),
            next_value: None,
        }
    }
}

pub struct IntoIter<T> {
    next_tries: Vec<(Vec<u8>, Trie<T>)>,
    current_trie: Option<(Vec<u8>, Trie<T>)>,
    next_value: Option<(Vec<u8>, T)>,
}

impl<T> Iterator for IntoIter<T> {
    type Item = (Vec<u8>, T);

    fn next(&mut self) -> Option<(Vec<u8>, T)> {
        loop {
            if let Some(value) = self.next_value.take() {
                return Some(value);
            } else if let Some((prefix, trie)) = self.current_trie.take() {
                if let Some(v) = trie.value {
                    self.next_value = Some((prefix.clone(), v));
                }

                for (diverged_at, child) in trie
                    .lower_than
                    .into_iter()
                    .chain(trie.bigger_than.into_iter())
                {
                    let mut k = prefix.clone();
                    k.extend(&trie.label[..diverged_at]);
                    self.next_tries.push((k, child));
                }
            } else if let Some(next_trie) = self.next_tries.pop() {
                self.current_trie = Some(next_trie);
            } else {
                return None;
            }
        }
    }
}

impl<K: AsRef<[u8]>, V> Extend<(K, V)> for Trie<V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        for (k, v) in iter.into_iter() {
            self.insert(k.as_ref(), v);
        }
    }
}

#[cfg(test)]
mod test {
    use std::fmt::Debug;

    use super::*;

    fn debug_tree_impl<T: Debug>(tree: &Trie<T>) -> Vec<String> {
        let mut rv = Vec::new();

        for (i, &c) in tree.label.iter().enumerate() {
            rv.push(format!("{}", c as char));

            if let Some(child) = tree.lower_than.get(&(i + 1)) {
                if let Some(value) = &child.value {
                    rv.push(format!(" -> {:?}", value));
                }
                for line in debug_tree_impl(child) {
                    rv.push(format!(" < {}", line));
                }
            }

            if let Some(child) = tree.bigger_than.get(&(i + 1)) {
                if let Some(value) = &child.value {
                    rv.push(format!(" -> {:?}", value));
                }
                for line in debug_tree_impl(child) {
                    rv.push(format!(" > {}", line));
                }
            }
        }

        for child in tree.lower_than.get(&0).into_iter().chain(tree.bigger_than.get(&0)) {
            let max_len = rv.iter().map(String::len).max().unwrap_or(0) + 4;
            for line in &mut rv {
                for _ in 0..(max_len - line.len()) {
                    line.push(' ');
                }
            }

            let child_rv = debug_tree_impl(child);

            if child_rv.len() > rv.len() {
                for _ in 0..(child_rv.len() - rv.len()) {
                    rv.push(" ".to_owned().repeat(max_len));
                }
            }

            for (line, child_line) in rv.iter_mut().zip(child_rv.iter()) {
                line.push_str(&child_line);
            }
        }

        rv
    }

    fn debug_tree<T: Debug>(tree: &Trie<T>) -> String {
        let mut rv = String::new();
        for line in debug_tree_impl(tree) {
            rv.push_str(line.trim_end());
            rv.push('\n');
        }

        rv
    }

    #[test]
    fn test_basic() {
        let mut map = Trie::new();

        map.insert(b"foo", "invalid");
        map.insert(b"foobar", "barbar");
        map.insert(b"fooxxx", "barbam");
        map.insert(b"fooaaa", "barbam");
        map.insert(b"foo", "bar");
        map.insert(b"blabar", "blabla");

        assert_eq!(map.get(b"foo"), Some(&"bar"));
        assert_eq!(map.get(b"foobar"), Some(&"barbar"));
        assert_eq!(map.get(b"blabar"), Some(&"blabla"));

     
        assert_eq!(debug_tree(&map), "\
f                  b
o                  l
o                  a
 < a               b
 < a               a
 < a               r
 <  -> \"barbam\"     -> \"blabla\"
 -> \"bar\"
 > x
 > x
 > x
 >  -> \"barbam\"
b
a
r
 -> \"barbar\"
");

        assert_eq!(
            map.into_iter().collect::<Vec<_>>(),
            vec![
                (b"foobar".to_vec(), "barbar"),
                (b"foo".to_vec(), "bar"),
                (b"fooxxx".to_vec(), "barbam"),
                (b"fooaaa".to_vec(), "barbam"),
                (b"blabar".to_vec(), "blabla"),
            ]
        );
    }
}
