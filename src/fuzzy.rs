// Copyright (c) 2026 Faris Alfarhan
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

pub fn ratio(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let b2j = char_positions(&b);
    let mut blocks = Vec::new();
    matching_blocks(&a, &b, 0, a.len(), 0, b.len(), &b2j, &mut blocks);
    blocks.sort();
    let matches: usize = blocks.iter().map(|(_, _, size)| size).sum();
    2.0 * matches as f64 / (a.len() + b.len()) as f64
}

pub fn get_close_matches<'a>(
    word: &str,
    choices: &'a [String],
    n: usize,
    cutoff: f64,
) -> Vec<&'a str> {
    let mut scored: Vec<(f64, &'a str)> = Vec::new();
    for choice in choices {
        let r = ratio(word, choice);
        if r >= cutoff {
            scored.push((r, choice));
        }
    }
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(n);
    scored.into_iter().map(|(_, s)| s).collect()
}

fn char_positions(b: &[char]) -> HashMap<char, Vec<usize>> {
    let mut map: HashMap<char, Vec<usize>> = HashMap::new();
    for (idx, &c) in b.iter().enumerate() {
        map.entry(c).or_default().push(idx);
    }
    map
}

fn matching_blocks(
    a: &[char],
    b: &[char],
    alo: usize,
    ahi: usize,
    blo: usize,
    bhi: usize,
    b2j: &HashMap<char, Vec<usize>>,
    out: &mut Vec<(usize, usize, usize)>,
) {
    let (i, j, k) = find_longest_match(a, alo, ahi, blo, bhi, b2j);
    if k == 0 {
        return;
    }
    if alo < i && blo < j {
        matching_blocks(a, b, alo, i, blo, j, b2j, out);
    }
    out.push((i, j, k));
    if i + k < ahi && j + k < bhi {
        matching_blocks(a, b, i + k, ahi, j + k, bhi, b2j, out);
    }
}

fn find_longest_match(
    a: &[char],
    alo: usize,
    ahi: usize,
    blo: usize,
    bhi: usize,
    b2j: &HashMap<char, Vec<usize>>,
) -> (usize, usize, usize) {
    let mut j2len: HashMap<usize, usize> = HashMap::new();
    let mut best = (alo, blo, 0usize);
    for i in alo..ahi {
        let mut new_j2len: HashMap<usize, usize> = HashMap::new();
        if let Some(positions) = b2j.get(&a[i]) {
            for &j in positions {
                if j < blo || j >= bhi {
                    continue;
                }
                let k = j2len.get(&j.wrapping_sub(1)).copied().unwrap_or(0) + 1;
                new_j2len.insert(j, k);
                if k > best.2 {
                    best = (i + 1 - k, j + 1 - k, k);
                }
            }
        }
        j2len = new_j2len;
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_identical_strings() {
        assert_eq!(ratio("Al-Faatiha", "Al-Faatiha"), 1.0);
    }

    #[test]
    fn ratio_empty_strings() {
        assert_eq!(ratio("", ""), 1.0);
        assert_eq!(ratio("abc", ""), 0.0);
    }

    #[test]
    fn ratio_partial_match() {
        let r = ratio("rahman", "Ar-Rahman");
        assert!(r > 0.5 && r < 1.0);
    }

    #[test]
    fn close_matches_ranking() {
        let names: Vec<String> = vec![
            "Al-Faatiha".into(),
            "Al-Baqarah".into(),
            "Aal-E-Imran".into(),
            "An-Nisa".into(),
            "Ar-Rahman".into(),
        ];
        let matches = get_close_matches("rahman", &names, 5, 0.5);
        assert_eq!(matches[0], "Ar-Rahman");
    }

    #[test]
    fn close_matches_cutoff() {
        let names: Vec<String> = vec!["Al-Faatiha".into()];
        let matches = get_close_matches("xyzzy", &names, 5, 0.5);
        assert!(matches.is_empty());
    }

    #[test]
    fn close_matches_limits_n() {
        let names: Vec<String> = vec![
            "Ar-Rahman".into(),
            "Al-Mulk".into(),
            "Ya-Sin".into(),
            "Al-Insan".into(),
        ];
        let matches = get_close_matches("rahman", &names, 2, 0.3);
        assert!(matches.len() <= 2);
    }
}