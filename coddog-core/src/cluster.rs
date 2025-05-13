use crate::*;

#[derive(Debug)]
pub struct Cluster<'a> {
    pub syms: Vec<&'a Symbol>,
}

impl Cluster<'_> {
    pub fn size(&self) -> usize {
        self.syms.len()
    }
}

pub fn get_clusters(symbols: &[Symbol], threshold: f32, min_len: usize) -> Vec<Cluster> {
    let mut clusters: Vec<Cluster> = Vec::new();

    symbols
        .iter()
        .filter(|s: &&Symbol| s.insns.len() >= min_len)
        .for_each(|symbol| {
            let mut cluster_match = false;

            for cluster in &mut clusters {
                let cluster_score = diff_symbols(symbol, cluster.syms[0], threshold);
                if cluster_score > threshold {
                    cluster_match = true;
                    cluster.syms.push(symbol);
                    break;
                }
            }

            // Add this symbol to a new cluster if it didn't match any existing clusters
            if !cluster_match {
                clusters.push(Cluster { syms: vec![symbol] });
            }
        });

    // Sort clusters by size
    clusters.sort_by_key(|c| std::cmp::Reverse(c.size()));

    clusters
}

pub fn do_cluster(symbols: &[Symbol], threshold: f32, min_len: usize) {
    let clusters = get_clusters(symbols, threshold, min_len);

    // Print clusters
    for cluster in clusters.iter().filter(|c| c.size() > 1) {
        println!(
            "Cluster {} has {} symbols",
            cluster.syms[0].name,
            cluster.size()
        );
    }
}
