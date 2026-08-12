//! Fuzzy string matching, ported inline from `github.com/sahilm/fuzzy`
//! (used by the list component's `DefaultFilter`).
//!
//! Provides fuzzy string matching optimized for filenames and code symbols
//! in the style of Sublime Text, VSCode, IntelliJ IDEA et al.

use std::cmp::Ordering;

/// Match represents a matched string.
#[derive(Debug, Clone)]
pub struct Match {
    /// The matched string.
    pub str: String,
    /// The index of the matched string in the supplied slice.
    pub index: usize,
    /// The indexes of matched characters. Useful for highlighting matches.
    pub matched_indexes: Vec<usize>,
    /// Score used to rank matches.
    pub score: i32,
}

const FIRST_CHAR_MATCH_BONUS: i32 = 10;
const MATCH_FOLLOWING_SEPARATOR_BONUS: i32 = 20;
const CAMEL_CASE_MATCH_BONUS: i32 = 20;
const ADJACENT_MATCH_BONUS: i32 = 5;
const UNMATCHED_LEADING_CHAR_PENALTY: i32 = -5;
const MAX_UNMATCHED_LEADING_CHAR_PENALTY: i32 = -15;

const SEPARATORS: [char; 6] = ['/', '-', '_', '.', ' ', '\\'];

/// Find looks up pattern in data and returns matches in descending order of
/// match quality. Match quality is determined by a set of bonus and penalty
/// rules.
///
/// The following types of matches apply a bonus:
///
/// * The first character in the pattern matches the first character in the
///   match string.
/// * The matched character is camel cased.
/// * The matched character follows a separator such as an underscore
///   character.
/// * The matched character is adjacent to a previous match.
///
/// Penalties are applied for every character in the search string that
/// wasn't matched and all leading characters up to the first match.
///
/// Results are sorted by best match.
pub fn find(pattern: &str, data: &[String]) -> Vec<Match> {
    let mut matches = find_no_sort(pattern, data);
    matches.sort_by(|a, b| a.score.cmp(&b.score).reverse());
    matches
}

/// FindNoSort is an alternative Find implementation that does not sort
/// the results in the end.
pub fn find_no_sort(pattern: &str, data: &[String]) -> Vec<Match> {
    if pattern.is_empty() {
        return vec![];
    }
    let runes: Vec<char> = pattern.chars().collect();
    let mut matches: Vec<Match> = Vec::new();
    let mut matched_indexes: Option<Vec<usize>> = None;
    for (i, s) in data.iter().enumerate() {
        let mut match_ = Match {
            str: s.clone(),
            index: i,
            matched_indexes: matched_indexes
                .take()
                .unwrap_or_else(|| Vec::with_capacity(runes.len())),
            score: 0,
        };
        let mut pattern_index = 0usize;
        let mut best_score = -1i32;
        let mut matched_index: isize = -1;
        let mut curr_adjacent_match_bonus = 0i32;
        let mut last: char = '\0';
        let mut last_index = 0usize;
        let chars: Vec<char> = s.chars().collect();
        let mut j = 0usize;
        while j < chars.len() {
            let candidate = chars[j];
            if let Some(pc) = runes.get(pattern_index).copied() {
                if equal_fold(candidate, pc) {
                    let mut score = 0i32;
                    if j == 0 {
                        score += FIRST_CHAR_MATCH_BONUS;
                    }
                    if last.is_lowercase() && candidate.is_uppercase() {
                        score += CAMEL_CASE_MATCH_BONUS;
                    }
                    if j != 0 && is_separator(last) {
                        score += MATCH_FOLLOWING_SEPARATOR_BONUS;
                    }
                    if let Some(&last_match) = match_.matched_indexes.last() {
                        let bonus =
                            adjacent_char_bonus(last_index, last_match, curr_adjacent_match_bonus);
                        score += bonus;
                        // adjacent matches are incremental and keep
                        // increasing based on previous adjacent matches thus
                        // we need to maintain the current match bonus
                        curr_adjacent_match_bonus += bonus;
                    }
                    if score > best_score {
                        best_score = score;
                        matched_index = j as isize;
                    }
                }
            }
            let nextp = if pattern_index + 1 < runes.len() {
                Some(runes[pattern_index + 1])
            } else {
                None
            };
            let nextc = if j + 1 < chars.len() {
                Some(chars[j + 1])
            } else {
                None
            };
            // We apply the best score when we have the next match coming up
            // or when the search string has ended. Tracking when the next
            // match is coming up allows us to exhaustively find the best
            // match and not necessarily the first match.
            if ((nextp.is_some() && nextc.is_some() && equal_fold(nextp.unwrap(), nextc.unwrap()))
                || nextc.is_none())
                && matched_index > -1
            {
                if match_.matched_indexes.is_empty() {
                    let penalty = matched_index as i32 * UNMATCHED_LEADING_CHAR_PENALTY;
                    best_score += max(penalty, MAX_UNMATCHED_LEADING_CHAR_PENALTY);
                }
                match_.score += best_score;
                match_.matched_indexes.push(matched_index as usize);
                best_score = -1;
                pattern_index += 1;
            }
            last_index = j;
            last = candidate;
            j += 1;
        }
        // apply penalty for each unmatched character
        let penalty = match_.matched_indexes.len() as i32 - chars.len() as i32;
        match_.score += penalty;
        if match_.matched_indexes.len() == runes.len() {
            matches.push(match_);
            matched_indexes = None;
        } else {
            matched_indexes = Some(match_.matched_indexes.clone());
        }
    }
    matches
}

/// Taken from strings.EqualFold
fn equal_fold(tr: char, sr: char) -> bool {
    if tr == sr {
        return true;
    }
    if tr.to_lowercase().collect::<String>() == sr.to_lowercase().collect::<String>() {
        return true;
    }
    // ASCII fast path: uppercase vs lowercase pair.
    let tr_lower = tr.to_ascii_lowercase();
    let sr_lower = sr.to_ascii_lowercase();
    tr_lower == sr_lower && (tr.is_ascii_alphabetic() || sr.is_ascii_alphabetic())
}

fn adjacent_char_bonus(i: usize, last_match: usize, current_bonus: i32) -> i32 {
    if last_match == i {
        return current_bonus * 2 + ADJACENT_MATCH_BONUS;
    }
    0
}

fn is_separator(s: char) -> bool {
    SEPARATORS.contains(&s)
}

fn max(x: i32, y: i32) -> i32 {
    if x > y {
        x
    } else {
        y
    }
}

/// SortOrder is used by callers that sort matches themselves (matching Go's
/// `sort.Stable` on score descending).
pub fn score_cmp(a: &Match, b: &Match) -> Ordering {
    b.score.cmp(&a.score)
}
