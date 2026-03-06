use hecs::World;

pub trait WorldExt {
    fn component_stats(&self) -> std::collections::HashMap<String, usize>;
}

impl WorldExt for World {
    fn component_stats(&self) -> std::collections::HashMap<String, usize> {
        let mut stats = std::collections::HashMap::new();
        stats.insert("entities".to_string(), self.iter().count());
        stats
    }
}
