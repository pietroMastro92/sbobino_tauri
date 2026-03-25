use std::collections::HashMap;

use crate::TimedSegment;

const MIN_DUPLICATE_WORDS: usize = 4;
const MIN_DUPLICATE_CHARS: usize = 12;
const MAX_DUPLICATE_GAP_SECONDS: f32 = 1.5;
const MAX_CONTEXTUAL_TOKEN_DELTA_RATIO: f32 = 0.18;
const MIN_CONTEXTUAL_TOKEN_OVERLAP_RATIO: f32 = 0.58;
const MIN_CONTEXTUAL_BIGRAM_OVERLAP_RATIO: f32 = 0.42;
const MAX_CONTEXTUAL_NOVEL_TOKEN_RATIO: f32 = 0.12;
const MIN_CONTEXTUAL_TOKEN_ALLOWANCE: usize = 3;

pub fn minimize_transcript_repetitions(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut cleaned_lines = Vec::<String>::new();
    let mut previous_key: Option<String> = None;
    let mut pending_blank = false;

    for raw_line in normalized.lines() {
        let collapsed = collapse_whitespace(raw_line);
        if collapsed.is_empty() {
            pending_blank = !cleaned_lines.is_empty();
            previous_key = None;
            continue;
        }

        let key = duplicate_key(&collapsed);
        if is_substantive_duplicate_candidate(&collapsed)
            && previous_key.as_deref() == Some(key.as_str())
        {
            continue;
        }

        if pending_blank && !cleaned_lines.is_empty() {
            cleaned_lines.push(String::new());
            pending_blank = false;
        }

        cleaned_lines.push(collapsed);
        previous_key = Some(key);
    }

    cleaned_lines.join("\n").trim().to_string()
}

pub fn constrain_transcript_edit(source: &str, edited: &str) -> String {
    let normalized_source = minimize_transcript_repetitions(source);
    let normalized_edited = minimize_transcript_repetitions(edited);

    if normalized_source.trim().is_empty() {
        return normalized_edited;
    }

    if normalized_edited.trim().is_empty() {
        return normalized_source;
    }

    let source_tokens = tokenize_transcript_content(&normalized_source);
    let edited_tokens = tokenize_transcript_content(&normalized_edited);

    if is_token_subsequence(&source_tokens, &edited_tokens)
        || is_safe_contextual_transcript_edit(&source_tokens, &edited_tokens)
    {
        normalized_edited
    } else {
        normalized_source
    }
}

pub fn merge_optimized_transcript_sections(
    sections: &[String],
    min_overlap_tokens: usize,
) -> String {
    let mut merged = String::new();

    for section in sections {
        let cleaned = strip_section_markers(section);
        if cleaned.trim().is_empty() {
            continue;
        }

        if merged.trim().is_empty() {
            merged = cleaned;
            continue;
        }

        merged = merge_optimized_section_pair(&merged, &cleaned, min_overlap_tokens);
    }

    minimize_transcript_repetitions(&merged)
}

pub fn collapse_consecutive_repeated_segments(segments: &[TimedSegment]) -> Vec<TimedSegment> {
    let mut collapsed = Vec::<TimedSegment>::new();

    for segment in segments {
        let text = collapse_whitespace(&segment.text);
        if text.is_empty() {
            continue;
        }

        let mut next = segment.clone();
        next.text = text;

        if let Some(previous) = collapsed.last_mut() {
            if should_collapse_segment_pair(previous, &next) {
                previous.end_seconds =
                    merge_optional_seconds(previous.end_seconds, next.end_seconds);
                if previous.start_seconds.is_none() {
                    previous.start_seconds = next.start_seconds;
                }
                if previous.speaker_id.is_none() {
                    previous.speaker_id = next.speaker_id.clone();
                }
                if previous.speaker_label.is_none() {
                    previous.speaker_label = next.speaker_label.clone();
                }
                continue;
            }
        }

        collapsed.push(next);
    }

    collapsed
}

fn should_collapse_segment_pair(left: &TimedSegment, right: &TimedSegment) -> bool {
    if !is_substantive_duplicate_candidate(&left.text)
        || !is_substantive_duplicate_candidate(&right.text)
    {
        return false;
    }

    if duplicate_key(&left.text) != duplicate_key(&right.text) {
        return false;
    }

    if normalized_optional(left.speaker_id.as_deref())
        != normalized_optional(right.speaker_id.as_deref())
    {
        return false;
    }
    if normalized_optional(left.speaker_label.as_deref())
        != normalized_optional(right.speaker_label.as_deref())
    {
        return false;
    }

    match (left.end_seconds, right.start_seconds) {
        (Some(left_end), Some(right_start)) if left_end.is_finite() && right_start.is_finite() => {
            right_start <= left_end + MAX_DUPLICATE_GAP_SECONDS
        }
        _ => true,
    }
}

fn merge_optional_seconds(left: Option<f32>, right: Option<f32>) -> Option<f32> {
    match (left, right) {
        (Some(a), Some(b)) if a.is_finite() && b.is_finite() => Some(a.max(b)),
        (Some(a), _) if a.is_finite() => Some(a),
        (_, Some(b)) if b.is_finite() => Some(b),
        _ => None,
    }
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_section_markers(value: &str) -> String {
    value
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed.starts_with("[Section ") && trimmed.ends_with(']'))
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn tokenize_transcript_content(value: &str) -> Vec<String> {
    value
        .split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|token| {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_lowercase())
            }
        })
        .collect()
}

fn is_token_subsequence(source: &[String], candidate: &[String]) -> bool {
    if candidate.is_empty() {
        return true;
    }

    let mut source_index = 0_usize;
    for token in candidate {
        while source_index < source.len() && source[source_index] != *token {
            source_index += 1;
        }
        if source_index == source.len() {
            return false;
        }
        source_index += 1;
    }

    true
}

fn is_safe_contextual_transcript_edit(source: &[String], candidate: &[String]) -> bool {
    if source.is_empty() || candidate.is_empty() {
        return false;
    }

    if candidate.len() > source.len() && is_token_subsequence(candidate, source) {
        return false;
    }

    let allowed_token_delta = ((source.len() as f32) * MAX_CONTEXTUAL_TOKEN_DELTA_RATIO).ceil()
        as usize
        + MIN_CONTEXTUAL_TOKEN_ALLOWANCE;
    let token_delta = source.len().abs_diff(candidate.len());
    if token_delta > allowed_token_delta {
        return false;
    }

    let token_overlap = multiset_overlap_count(source, candidate);
    let min_overlap_source =
        ((source.len() as f32) * MIN_CONTEXTUAL_TOKEN_OVERLAP_RATIO).ceil() as usize;
    let min_overlap_candidate =
        ((candidate.len() as f32) * MIN_CONTEXTUAL_TOKEN_OVERLAP_RATIO).ceil() as usize;
    if token_overlap < min_overlap_source || token_overlap < min_overlap_candidate {
        return false;
    }

    let max_novel_tokens = ((candidate.len() as f32) * MAX_CONTEXTUAL_NOVEL_TOKEN_RATIO).ceil()
        as usize
        + MIN_CONTEXTUAL_TOKEN_ALLOWANCE;
    let novel_tokens = candidate.len().saturating_sub(token_overlap);
    if novel_tokens > max_novel_tokens {
        return false;
    }

    let source_bigrams = build_token_ngrams(source, 2);
    let candidate_bigrams = build_token_ngrams(candidate, 2);
    if !source_bigrams.is_empty() && !candidate_bigrams.is_empty() {
        let bigram_overlap = multiset_overlap_count(&source_bigrams, &candidate_bigrams);
        let min_bigram_overlap =
            ((source_bigrams.len() as f32) * MIN_CONTEXTUAL_BIGRAM_OVERLAP_RATIO).ceil() as usize;
        if bigram_overlap < min_bigram_overlap {
            return false;
        }
    }

    true
}

fn multiset_overlap_count(source: &[String], candidate: &[String]) -> usize {
    let mut counts = HashMap::<&str, usize>::new();
    for token in source {
        *counts.entry(token.as_str()).or_insert(0) += 1;
    }

    let mut overlap = 0_usize;
    for token in candidate {
        if let Some(count) = counts.get_mut(token.as_str()) {
            if *count > 0 {
                *count -= 1;
                overlap += 1;
            }
        }
    }

    overlap
}

fn build_token_ngrams(tokens: &[String], size: usize) -> Vec<String> {
    if size == 0 || tokens.len() < size {
        return Vec::new();
    }

    tokens
        .windows(size)
        .map(|window| window.join("\u{1f}"))
        .collect()
}

fn tokenize_with_spans(value: &str) -> Vec<(String, usize, usize)> {
    let mut output = Vec::<(String, usize, usize)>::new();
    let mut active_start: Option<usize> = None;

    for (index, ch) in value.char_indices() {
        if ch.is_alphanumeric() {
            if active_start.is_none() {
                active_start = Some(index);
            }
        } else if let Some(start) = active_start.take() {
            output.push((value[start..index].to_lowercase(), start, index));
        }
    }

    if let Some(start) = active_start {
        output.push((value[start..].to_lowercase(), start, value.len()));
    }

    output
}

fn merge_optimized_section_pair(left: &str, right: &str, min_overlap_tokens: usize) -> String {
    let left_trimmed = left.trim();
    let right_trimmed = right.trim();
    if left_trimmed.is_empty() {
        return right_trimmed.to_string();
    }
    if right_trimmed.is_empty() {
        return left_trimmed.to_string();
    }

    let left_tokens = tokenize_with_spans(left_trimmed);
    let right_tokens = tokenize_with_spans(right_trimmed);
    let overlap_limit = left_tokens.len().min(right_tokens.len());

    for overlap in (min_overlap_tokens..=overlap_limit).rev() {
        let left_slice = &left_tokens[left_tokens.len() - overlap..];
        let right_slice = &right_tokens[..overlap];

        if left_slice
            .iter()
            .map(|(token, _, _)| token)
            .eq(right_slice.iter().map(|(token, _, _)| token))
        {
            if overlap == right_tokens.len() {
                return left_trimmed.to_string();
            }

            let suffix_start = right_tokens[overlap].1;
            let suffix = right_trimmed[suffix_start..].trim_start();
            if suffix.is_empty() {
                return left_trimmed.to_string();
            }

            let separator = if left_trimmed.ends_with(char::is_whitespace) {
                ""
            } else {
                " "
            };
            return format!("{left_trimmed}{separator}{suffix}")
                .trim()
                .to_string();
        }
    }

    format!("{left_trimmed}\n\n{right_trimmed}")
}

fn duplicate_key(value: &str) -> String {
    collapse_whitespace(value)
        .split_whitespace()
        .map(|token| {
            token
                .trim_matches(|ch: char| {
                    ch.is_whitespace()
                        || matches!(
                            ch,
                            '.' | ','
                                | ';'
                                | ':'
                                | '!'
                                | '?'
                                | '"'
                                | '\''
                                | '`'
                                | '('
                                | ')'
                                | '['
                                | ']'
                                | '{'
                                | '}'
                                | '“'
                                | '”'
                                | '‘'
                                | '’'
                        )
                })
                .to_lowercase()
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_substantive_duplicate_candidate(value: &str) -> bool {
    let word_count = value.split_whitespace().count();
    let char_count = value.chars().count();
    word_count >= MIN_DUPLICATE_WORDS || char_count >= MIN_DUPLICATE_CHARS
}

fn normalized_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(|candidate| candidate.to_lowercase())
}

#[cfg(test)]
mod tests {
    use crate::TimedSegment;

    use super::{
        collapse_consecutive_repeated_segments, constrain_transcript_edit,
        merge_optimized_transcript_sections, minimize_transcript_repetitions,
    };

    #[test]
    fn removes_consecutive_duplicate_lines_with_small_variations() {
        let input = "So, the idea is to propose significant changes.\nso the idea is to propose significant changes!\nFinal line.";
        let cleaned = minimize_transcript_repetitions(input);
        assert_eq!(
            cleaned,
            "So, the idea is to propose significant changes.\nFinal line."
        );
    }

    #[test]
    fn keeps_short_legitimate_consecutive_lines() {
        let input = "Yes\nYes\nNo";
        let cleaned = minimize_transcript_repetitions(input);
        assert_eq!(cleaned, "Yes\nYes\nNo");
    }

    #[test]
    fn collapses_consecutive_duplicate_segments() {
        let segments = vec![
            TimedSegment {
                text: "Repeated sentence.".to_string(),
                start_seconds: Some(0.0),
                end_seconds: Some(1.0),
                ..TimedSegment::default()
            },
            TimedSegment {
                text: " repeated sentence ".to_string(),
                start_seconds: Some(1.02),
                end_seconds: Some(2.1),
                ..TimedSegment::default()
            },
            TimedSegment {
                text: "Final line".to_string(),
                start_seconds: Some(2.2),
                end_seconds: Some(3.0),
                ..TimedSegment::default()
            },
        ];

        let collapsed = collapse_consecutive_repeated_segments(&segments);
        assert_eq!(collapsed.len(), 2);
        assert_eq!(collapsed[0].text, "Repeated sentence.");
        assert_eq!(collapsed[0].end_seconds, Some(2.1));
        assert_eq!(collapsed[1].text, "Final line");
    }

    #[test]
    fn constrain_transcript_edit_keeps_punctuation_only_edits() {
        let source = "hello world this is a test";
        let edited = "Hello world, this is a test.";

        assert_eq!(
            constrain_transcript_edit(source, edited),
            "Hello world, this is a test."
        );
    }

    #[test]
    fn constrain_transcript_edit_rejects_added_content() {
        let source = "hello world this is a test";
        let edited = "Hello world, this is a test. Added conclusion here.";

        assert_eq!(constrain_transcript_edit(source, edited), source);
    }

    #[test]
    fn constrain_transcript_edit_allows_contextual_term_fixes() {
        let source = "ho usato la libreria keras tuner e scikit larn per preparare le pipeline e i modelli di deep lurning";
        let edited = "Ho usato la libreria Keras Tuner e scikit-learn per preparare le pipeline e i modelli di deep learning.";

        assert_eq!(constrain_transcript_edit(source, edited), edited);
    }

    #[test]
    fn constrain_transcript_edit_rejects_large_rewrites() {
        let source = "the candidate describes the research workflow with python keras and remote sensing data";
        let edited =
            "The speaker presents a polished summary of the project, its outcomes, and the future roadmap.";

        assert_eq!(constrain_transcript_edit(source, edited), source);
    }

    #[test]
    fn merge_optimized_sections_strips_section_labels_and_overlap() {
        let sections = vec![
            "[Section 1]\nHello world this is a test and we continue".to_string(),
            "[Section 2]\nthis is a test and we continue with another sentence".to_string(),
        ];

        let merged = merge_optimized_transcript_sections(&sections, 4);
        assert!(!merged.contains("[Section"));
        assert_eq!(
            merged,
            "Hello world this is a test and we continue with another sentence"
        );
    }
}
