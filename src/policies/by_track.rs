use std::collections::{HashMap, HashSet};

fn assign_tracks(tracks: &HashSet<usize>, connection_count: usize) -> HashMap<usize, usize> {
    let mut result = HashMap::new();

    let mut index = 0;
    for track in tracks {
        result.insert(*track, index);
        index = (index + 1) % connection_count;
    }

    result
}
