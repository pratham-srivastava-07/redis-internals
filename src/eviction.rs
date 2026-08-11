use rand::Rng;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Instant;

use crate::cmd::Entry;

const SAMPLE_SIZE: usize = 20;
const EXPIRED_THRESHOLD: f64 = 0.25;

// ---------------------------------------------------------------------------
// Active expiration: sample TTL keys, delete expired ones, repeat while the
// sample is >25% expired. (This is Redis's activeExpireCycle.)
// ---------------------------------------------------------------------------
pub fn evict_keys(store: &mut HashMap<String, Entry>) {
    let now = Instant::now();

    // Only keys that actually carry a TTL are candidates for expiry.
    let mut candidates: Vec<String> = store
        .iter()
        .filter(|(_, e)| e.expires_at.is_some())
        .map(|(k, _)| k.clone())
        .collect();

    let mut rng = rand::thread_rng();

    loop {
        if candidates.is_empty() {
            break;
        }

        let mut expired = 0;
        let sample_n = SAMPLE_SIZE.min(candidates.len());

        for _ in 0..sample_n {
            // swap_remove keeps sampling O(1) and never picks the same key twice
            let idx = rng.gen_range(0..candidates.len());
            let key = candidates.swap_remove(idx);

            if let Some(entry) = store.get(&key) {
                if matches!(entry.expires_at, Some(exp) if now >= exp) {
                    store.remove(&key);
                    expired += 1;
                }
            }
        }

        // Sample mostly clean? Stop. Still dirty? There are probably more.
        if (expired as f64) / (sample_n as f64) <= EXPIRED_THRESHOLD {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// LRU eviction index.
//
// A doubly linked list of keys ordered by recency, plus a HashMap for O(1)
// lookup. The node right after `head` is the most-recently-used; the node
// right before `tail` is the least-recently-used (the eviction victim).
//
// `head` and `tail` are sentinel nodes (dummy keys) so we never special-case
// the empty-list / first / last positions.
//
// `next` links own the chain (strong Rc). `prev` links are Weak to avoid a
// reference cycle that would leak memory.
// ---------------------------------------------------------------------------

type Link = Rc<RefCell<Node>>;

struct Node {
    key: String,
    prev: Option<Weak<RefCell<Node>>>,
    next: Option<Link>,
}

impl Node {
    fn new(key: String) -> Link {
        Rc::new(RefCell::new(Node {
            key,
            prev: None,
            next: None,
        }))
    }
}

pub struct Lru {
    map: HashMap<String, Link>,
    capacity: usize,
    head: Link, // MRU side
    tail: Link, // LRU side
}

impl Lru {
    pub fn new(capacity: usize) -> Self {
        let head = Node::new(String::new());
        let tail = Node::new(String::new());

        // wire the two sentinels together: head <-> tail
        head.borrow_mut().next = Some(Rc::clone(&tail));
        tail.borrow_mut().prev = Some(Rc::downgrade(&head));

        Lru {
            map: HashMap::new(),
            capacity,
            head,
            tail,
        }
    }

    /// Record that `key` was just used. If inserting a brand-new key pushes us
    /// over capacity, the least-recently-used key is evicted and returned so
    /// the caller can remove it from the real store.
    pub fn touch(&mut self, key: &str) -> Option<String> {
        // already tracked -> just move it to the front (MRU)
        if let Some(node) = self.map.get(key).cloned() {
            Self::unlink(&node);
            self.push_front(&node);
            return None;
        }

        // new key -> create, index, and place at the front
        let node = Node::new(key.to_string());
        self.map.insert(key.to_string(), Rc::clone(&node));
        self.push_front(&node);

        if self.map.len() > self.capacity {
            return self.evict();
        }
        None
    }

    /// Remove a key from the index (e.g. it was DEL'd or expired elsewhere).
    pub fn remove(&mut self, key: &str) {
        if let Some(node) = self.map.remove(key) {
            Self::unlink(&node);
        }
    }

    /// Drop the least-recently-used key and return it.
    fn evict(&mut self) -> Option<String> {
        let lru = self.tail.borrow().prev.clone()?.upgrade()?;

        // if the node before tail is head, the list is empty
        if Rc::ptr_eq(&lru, &self.head) {
            return None;
        }

        Self::unlink(&lru);
        let key = lru.borrow().key.clone();
        self.map.remove(&key);
        Some(key)
    }

    // --- linked-list plumbing -------------------------------------------------

    /// Splice `node` in right after `head` (the MRU position).
    fn push_front(&self, node: &Link) {
        let first = self.head.borrow().next.clone().expect("head always has next");

        node.borrow_mut().prev = Some(Rc::downgrade(&self.head));
        node.borrow_mut().next = Some(Rc::clone(&first));

        self.head.borrow_mut().next = Some(Rc::clone(node));
        first.borrow_mut().prev = Some(Rc::downgrade(node));
    }

    /// Detach `node` from its neighbours, stitching them together.
    fn unlink(node: &Link) {
        let prev = node
            .borrow()
            .prev
            .clone()
            .and_then(|w| w.upgrade())
            .expect("real nodes always have a prev");
        let next = node.borrow().next.clone().expect("real nodes always have a next");

        prev.borrow_mut().next = Some(Rc::clone(&next));
        next.borrow_mut().prev = Some(Rc::downgrade(&prev));

        node.borrow_mut().prev = None;
        node.borrow_mut().next = None;
    }
}
