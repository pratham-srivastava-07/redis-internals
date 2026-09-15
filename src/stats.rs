use std::collections::HashMap;

pub struct Stat {
    pub keyspace_stat: [HashMap<String, i32>; 4],
}

impl Stat {
    pub fn new() -> Self {
        Self {
            keyspace_stat: std::array::from_fn(|_| HashMap::new()),
        }
    }

    pub fn update_stat_db(&mut self, num: usize, metric: String, value: i32) {
        self.keyspace_stat[num].insert(metric, value);
    }
}
