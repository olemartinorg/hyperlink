use std::path::PathBuf;
use std::sync::Arc;
use std::mem;

use bumpalo::Bump;
use bumpalo::collections::Vec as BumpVec;
use patricia_tree::PatriciaMap;

use crate::allocator::BumpaloPatriciaAllocator;
use crate::html::{Href, Link, UsedLink};

impl<'a> AsRef<[u8]> for Href<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub trait LinkCollector<P: Send>: Send {
        fn new() -> Self;
    fn ingest(&mut self, link: Link<'_, P>);
    fn merge(&mut self, other: Self);
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct OwnedUsedLink<P> {
    pub href: String,
    pub path: Arc<PathBuf>,
    pub paragraph: Option<P>,
}

/// Collects only used links for match-all-paragraphs command. Discards defined links.
pub struct UsedLinkCollector<P> {
    pub used_links: Vec<OwnedUsedLink<P>>,
}

impl<P: Send> LinkCollector<P> for UsedLinkCollector<P> {
        fn new() -> Self {
        UsedLinkCollector {
            used_links: Vec::new(),
        }
    }

    fn ingest(&mut self, link: Link<'_, P>) {
        if let Link::Uses(used_link) = link {
            self.used_links.push(OwnedUsedLink {
                href: used_link.href.0.to_owned(),
                path: used_link.path.to_owned(),
                paragraph: used_link.paragraph,
            });
        }
    }

    fn merge(&mut self, other: Self) {
        self.used_links.extend(other.used_links);
    }
}

#[derive(Debug)]
enum LinkState<'a, P: 'a> {
    /// We have observed a DefinedLink for this href
    Defined,
    /// We have not *yet* observed a DefinedLink and therefore need to keep track of all link
    /// usages for potential error reporting.
    Undefined(BumpVec<'a, (Arc<PathBuf>, Option<P>)>),
}

// LinkState's BumpVec is naturally !Send because it points to a Bump, which is !Sync. However we
// can guarantee that all LinkStates within the same Bump are owned by the same thread. When
// they're all only accessible by one thread, the Bump does not need to be sync.
unsafe impl<'a, P> Send for LinkState<'a, P> {}

impl<'a, P: Copy> LinkState<'a, P> {
    fn add_usage(&mut self, link: &UsedLink<P>) {
        if let LinkState::Undefined(ref mut links) = self {
            links.push((link.path.clone(), link.paragraph));
        }
    }

    fn update(&mut self, other: Self) {
        match self {
            LinkState::Defined => (),
            LinkState::Undefined(links) => match other {
                LinkState::Defined => *self = LinkState::Defined,
                LinkState::Undefined(links2) => links.extend(links2.into_iter()),
            },
        }
    }
}

/// Link collector used for actual link checking. Keeps track of broken links only.
pub struct BrokenLinkCollector<P: 'static> {
    links: PatriciaMap<LinkState<'static, P>, BumpaloPatriciaAllocator<'static>>,
    used_link_count: usize,

    #[allow(unused)]
    bump: Box<Bump>,
}

impl<P: Send + Copy + PartialEq + 'static> LinkCollector<P> for BrokenLinkCollector<P> {
    fn new() -> Self {
        let bump = Box::new(Bump::new());
        let bump_ref: &'static Bump = unsafe {
            mem::transmute::<&Bump, &'static Bump>(&bump)
        };

        BrokenLinkCollector {
            bump,
            links: PatriciaMap::new_in(BumpaloPatriciaAllocator(bump_ref)),
            used_link_count: 0,
        }
    }

    fn ingest(&mut self, link: Link<'_, P>) {
        match link {
            Link::Uses(used_link) => {
                self.used_link_count += 1;
                if let Some(state) = self.links.get_mut(&used_link.href) {
                    state.add_usage(&used_link);
                } else {
                    let mut state = LinkState::Undefined(BumpVec::new_in(self.get_bump_ref()));
                    state.add_usage(&used_link);
                    self.links.insert(used_link.href, state);
                }
            }
            Link::Defines(defined_link) => {
                self.links.insert(defined_link.href, LinkState::Defined);
            }
        }
    }

    fn merge(&mut self, other: Self) {
        // TODO: rebuild tree here to avoid rellocation?
        self.used_link_count += other.used_link_count;

        for (href, other_state) in other.links {
            if let Some(state) = self.links.get_mut(&href) {
                state.update(other_state);
            } else {
                self.links.insert(href, other_state);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct BrokenLink<P> {
    pub hard_404: bool,
    pub link: OwnedUsedLink<P>,
}

impl<P: Copy + PartialEq + 'static> BrokenLinkCollector<P> {
    #[inline]
    fn get_bump_ref(&self) -> &'static Bump {
        unsafe {
            mem::transmute::<&Bump, &'static Bump>(&self.bump)
        }
    }

    pub fn get_broken_links(&self, check_anchors: bool) -> impl Iterator<Item = BrokenLink<P>> {
        let mut broken_links = Vec::new();

        for (href, state) in self.links.iter() {
            if let LinkState::Undefined(links) = state {
                let href = unsafe { String::from_utf8_unchecked(href) };
                let hard_404 = if check_anchors {
                    !matches!(
                        self.links.get(&Href(&href).without_anchor()),
                        Some(&LinkState::Defined)
                    )
                } else {
                    true
                };

                for (path, paragraph) in links.iter() {
                    broken_links.push(BrokenLink {
                        hard_404,
                        link: OwnedUsedLink {
                            path: path.clone(),
                            paragraph: *paragraph,
                            href: href.clone(),
                        },
                    });
                }
            }
        }

        broken_links.into_iter()
    }

    pub fn used_links_count(&self) -> usize {
        self.used_link_count
    }
}
