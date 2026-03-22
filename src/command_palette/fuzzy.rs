// Fuzzy string matching for command palette

/// Score a candidate string against a query using fuzzy matching.
/// Returns None if the query doesn't match, or Some(score) where higher is better.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let candidate_lower: Vec<char> = candidate.to_lowercase().chars().collect();
    let candidate_chars: Vec<char> = candidate.chars().collect();

    // Check if all query chars exist in candidate (in order)
    let mut qi = 0;
    for &cc in &candidate_lower {
        if qi < query_lower.len() && cc == query_lower[qi] {
            qi += 1;
        }
    }
    if qi < query_lower.len() {
        return None; // Not all query chars found
    }

    // Score the match
    let mut score: i32 = 0;
    qi = 0;
    let mut prev_match = false;
    let mut prev_was_separator = true; // Start counts as separator

    for (ci, &cc) in candidate_lower.iter().enumerate() {
        if qi < query_lower.len() && cc == query_lower[qi] {
            score += 1;

            // Bonus: consecutive matches
            if prev_match {
                score += 5;
            }

            // Bonus: word boundary
            if prev_was_separator {
                score += 10;
            }

            // Bonus: start of string
            if ci == 0 {
                score += 15;
            }

            // Bonus: exact case match
            if qi < query.len() && candidate_chars[ci] == query.chars().nth(qi).unwrap_or(' ') {
                score += 1;
            }

            qi += 1;
            prev_match = true;
        } else {
            if prev_match {
                score -= 1;
            }
            prev_match = false;
        }

        prev_was_separator = matches!(cc, ' ' | '/' | '-' | '_' | ':' | '\\');
    }

    Some(score)
}
