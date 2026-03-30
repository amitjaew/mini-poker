use crate::core::card::Card;

fn factorial(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}

pub fn combinations(cards: &[Card], k: usize) -> Vec<Vec<Card>> {
    let n = cards.len();
    if k > n {
        return vec![];
    }
    let n_comb = factorial(n) / (factorial(n - k) * factorial(k));
    let mut result = Vec::with_capacity(n_comb);
    let mut indices: Vec<usize> = (0..k).collect();

    loop {
        result.push(indices.iter().map(|&i| cards[i].clone()).collect());

        // Find the rightmost index that can be incremented
        let mut i = k;
        while i > 0 && indices[i - 1] == i - 1 + n - k {
            i -= 1;
        }
        if i == 0 {
            break;
        }
        indices[i - 1] += 1;
        for j in i..k {
            indices[j] = indices[j - 1] + 1;
        }
    }

    result
}

pub fn omaha_hands(hand: &[Card], community: &[Card]) -> Vec<Vec<Card>> {
    let hole_combos = combinations(hand, 2);
    let community_combos = combinations(community, 3);
    let mut result = Vec::with_capacity(hole_combos.len() * community_combos.len());
    for hole in &hole_combos {
        for comm in &community_combos {
            result.push(hole.iter().chain(comm.iter()).cloned().collect());
        }
    }
    result
}
