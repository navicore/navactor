use navactor::genes::gene::GeneType;
use std::collections::HashMap;

struct MockDirector {
    pub gene_path_map: HashMap<String, GeneType>,
}

impl MockDirector {
    fn find_gene_type(&mut self, path: &str) -> Option<GeneType> {
        let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_path = String::new();
        let mut gene_type = None;

        for component in &components {
            current_path.push('/');
            current_path.push_str(component);

            if let Some(gt) = self.gene_path_map.get(&current_path) {
                gene_type = Some(*gt);
            }
        }

        gene_type
    }
    fn new() -> Self {
        Self {
            gene_path_map: HashMap::new(),
        }
    }
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_lookup_gene_type() {
    let path = "/domain/building/1/floor/3/room/5";
    let mut director = MockDirector::new();
    director
        .gene_path_map
        .insert(String::from("/domain"), GeneType::Gauge);
    director
        .gene_path_map
        .insert(String::from("/domain/building"), GeneType::GaugeAndAccum);
    director
        .gene_path_map
        .insert(String::from("/domain/building/1"), GeneType::Accum);
    let gt = director.find_gene_type(path);
    assert!(gt.is_some());
    assert_eq!(gt.unwrap(), GeneType::Accum);
}

#[cfg_attr(feature = "cargo-clippy", allow(clippy::unwrap_used))]
#[test]
fn test_lookup_gene_type_short_path() {
    let path = "/domain/building";
    let mut director = MockDirector::new();
    director
        .gene_path_map
        .insert(String::from("/domain"), GeneType::Gauge);
    director
        .gene_path_map
        .insert(String::from("/domain/building"), GeneType::GaugeAndAccum);
    director
        .gene_path_map
        .insert(String::from("/domain/building/1"), GeneType::Accum);
    let gt = director.find_gene_type(path);
    assert!(gt.is_some());
    assert_eq!(gt.unwrap(), GeneType::GaugeAndAccum);
}
