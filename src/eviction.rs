use rand::Rng;
use std::collections::HashMap;
use crate::cmd::Entry;
use std::time::Instant;

const SAMPLE_SIZE: usize = 20;
const EXPIRED_THRESHOLD: f64 = 0.25;

// for implementing LRU Cache eviction strategy
 struct Node {
     key: String,
     value: Entry,

     prev: Node,
     next: Node
 }


impl Node {
    fn new(&self, key: String, value: Entry) -> Self {
        Node {
            key,
            value
        }
    }
}

struct LRU {
    map: HashMap<usize, Node>,
    capacity: usize,
    head: Node,
    tail: Node
}


impl LRU {
    fn new(&mut self, capacity) {
        self.capacity = capacity;
        self.head.next = tail;
        self.tail.prev = head;
    }

    fn get(&mut self, key: String) -> usize {
        if !self.map.contains_key(&key) {
            return -1;
        }

        let node: Node = self.map.get(key);
        
        let _ = delete_node(&node);

        let _ = insert_after_head(&node);

        return node.value;
    }

    fn put(&mut self, key: String, value: Entry) {
        if self.map.contains_key(key) {
            let mut node: Node = self.map.get(key);

            node.value = value;

            let _ = delete_node(&node);

            let _ = insert_after_head(&node);

            return node.value;
        }

        let node: Node = Node::new(key, value);


    }
}

pub fn evict_keys(store: &mut HashMap<String, Entry>) {
    let now = Instant::now();
    
    let mut candidates: Vec<String> = store.iter().filter(|(_, e)| e.expires_at.is_some()).map(|(k, _)| k.clone()).collect();

    let mut rng = rand::thread_rng();

    loop {
        if candidates.is_empty() {
            break;
        }

        let mut expired = 0;

        let sample_n = SAMPLE_SIZE.min(candidates.len());

        for _ in 0..sample_n {
            let idx = rng.gen_range(0..candidates.len());
            let key = candidates.swap_remove(idx);

            if let Some(entry) = store.get(&key) {
                if matches!(entry.expires_at, Some(exp) if now >= exp) {
                    store.remove(&key);
                    expired += 1;
                }
            }

        }

        if (expired as f64) / (sample_n as f64) <= EXPIRED_THRESHOLD {
            break;
        }
    }

}

fn delete_node(node: &Node) {
    !todo()
}


fn insert_after_head(node: &Node) {
    !todo()
}
