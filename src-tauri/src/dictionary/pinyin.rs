//! Pinyin handling for CC-CEDICT's numeric tone notation.
//!
//! CC-CEDICT stores `ni3 hao3`; readers need `nǐ hǎo`. The importer stores three
//! forms so no conversion happens on the UI thread at render time:
//!
//! | column            | example     | use                        |
//! |-------------------|-------------|----------------------------|
//! | `pinyin`          | `ni3 hao3`  | lossless source fidelity   |
//! | `pinyin_display`  | `nǐ hǎo`    | sidebar / card display     |
//! | `pinyin_plain`    | `ni hao`    | tone-less search           |
//! | `pinyin_compact`  | `nihao`     | keyboard-friendly search   |

/// Display marks for each tone 1–4, indexed `[vowel][tone - 1]`.
///
/// Syllabic nasals (`m2`, `n4`, `hng5`) have no vowel; pinyin still assigns them
/// a tone mark on the nasal, so they get their own rows.
const TONE_MARKS: &[(char, [&str; 4])] = &[
    ('a', ["ā", "á", "ǎ", "à"]),
    ('e', ["ē", "é", "ě", "è"]),
    ('i', ["ī", "í", "ǐ", "ì"]),
    ('o', ["ō", "ó", "ǒ", "ò"]),
    ('u', ["ū", "ú", "ǔ", "ù"]),
    ('ü', ["ǖ", "ǘ", "ǚ", "ǜ"]),
    ('m', ["m̄", "ḿ", "m̌", "m̀"]),
    ('n', ["n̄", "ń", "ň", "ǹ"]),
];

/// Convert a whole pinyin field (`ni3 hao3`) to its tone-marked form (`nǐ hǎo`).
pub fn numeric_to_display(raw: &str) -> String {
    raw.split_whitespace()
        .filter(|syllable| !syllable.is_empty())
        .map(syllable_to_display)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Convert a pinyin field to a tone-less, lowercase, space-separated form
/// (`ni hao`). Punctuation-only tokens such as the `·` in foreign names are
/// dropped because they are noise for matching.
pub fn numeric_to_plain(raw: &str) -> String {
    raw.split_whitespace()
        .map(|token| {
            let without_tone = strip_tone_digit(token);
            without_tone
                .replace("u:", "u")
                .replace("U:", "u")
                .to_lowercase()
        })
        .filter(|token| token.chars().any(|c| c.is_alphanumeric()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Further compress a plain form for keyboard search: `ni hao` -> `nihao`.
pub fn compact(plain: &str) -> String {
    plain
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Normalise arbitrary user input for pinyin matching.
///
/// Handles numeric tones (`ni3hao3`), tone marks (`nǐ hǎo`), `u:`/`ü` and stray
/// punctuation, returning `(spaced, compact)` forms.
pub fn normalize_for_search(input: &str) -> (String, String) {
    let folded = fold_tone_marks(input);
    let plain = folded
        .split_whitespace()
        .map(|token| {
            // Drop every tone digit, not just a trailing one, so both `ni3 hao3`
            // and keyboard-mashed `ni3hao3` normalise to `ni hao` / `nihao`.
            token
                .chars()
                .filter(|c| !c.is_ascii_digit())
                .collect::<String>()
                .replace("u:", "u")
                .replace('ü', "u")
        })
        .filter(|token| token.chars().any(|c| c.is_alphanumeric()))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();

    let compact = plain
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>();

    (plain, compact)
}

fn strip_tone_digit(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next_back() {
        Some(last) if last.is_ascii_digit() => chars.as_str().to_string(),
        _ => token.to_string(),
    }
}

fn syllable_to_display(token: &str) -> String {
    let (body, tone) = match token.chars().next_back() {
        Some(last) if ('1'..='5').contains(&last) => (
            token[..token.len() - last.len_utf8()].to_string(),
            last.to_digit(10).unwrap_or(0) as u8,
        ),
        // Acronyms and unromanised letters (`[A]`, `[C J B]`) have no tone.
        _ => (token.to_string(), 0),
    };

    // `u:` is CC-CEDICT's ASCII spelling of ü.
    let body = body.replace("u:", "ü").replace("U:", "Ü");

    // Tone 5 / no tone: no mark is written.
    if tone == 0 || tone > 4 {
        return body;
    }

    let chars: Vec<char> = body.chars().collect();
    let folded: Vec<char> = chars.iter().map(|c| fold_char(*c)).collect();

    let Some(position) = mark_position(&folded) else {
        // Nothing markable (rare); return the readable form rather than drop data.
        return body;
    };

    let base = folded[position];
    let Some((_, marks)) = TONE_MARKS.iter().find(|(vowel, _)| *vowel == base) else {
        return body;
    };
    let mark = marks[(tone - 1) as usize];

    let replacement = if chars[position].is_uppercase() {
        mark.chars()
            .flat_map(|c| c.to_uppercase())
            .collect::<String>()
    } else {
        mark.to_string()
    };

    let mut out = String::with_capacity(body.len() + 2);
    for (index, ch) in chars.iter().enumerate() {
        if index == position {
            out.push_str(&replacement);
        } else {
            out.push(*ch);
        }
    }
    out
}

/// Index of the vowel that receives the tone mark.
///
/// Standard Mandarin rule: `a` wins, then `e`, then the `o` of `ou`, otherwise
/// the final vowel (`iu` -> u, `ui` -> i, `üe` -> e…).
fn mark_position(folded: &[char]) -> Option<usize> {
    if let Some(index) = folded.iter().position(|c| *c == 'a') {
        return Some(index);
    }
    if let Some(index) = folded.iter().position(|c| *c == 'e') {
        return Some(index);
    }
    if let Some(index) = folded.windows(2).position(|pair| pair == ['o', 'u']) {
        return Some(index);
    }
    folded
        .iter()
        .rposition(|c| matches!(c, 'i' | 'o' | 'u' | 'ü'))
        // Syllabic nasals (m2, n4) carry the tone on the nasal itself.
        .or_else(|| folded.iter().rposition(|c| matches!(c, 'm' | 'n')))
}

fn fold_char(c: char) -> char {
    match c {
        'Ü' => 'ü',
        _ => c.to_ascii_lowercase(),
    }
}

/// Map accented pinyin vowels back to their base letters so users can paste
/// `nǐ hǎo` into the search box. Combining marks (used for the syllabic nasals
/// `m̄`, `m̌`, `m̀`, `n̄`) are dropped rather than matched as characters.
fn fold_tone_marks(input: &str) -> String {
    input
        .chars()
        .filter_map(|c| match c {
            'ā' | 'á' | 'ǎ' | 'à' | 'a' => Some('a'),
            'ē' | 'é' | 'ě' | 'è' | 'e' => Some('e'),
            'ī' | 'í' | 'ǐ' | 'ì' | 'i' => Some('i'),
            'ō' | 'ó' | 'ǒ' | 'ò' | 'o' => Some('o'),
            'ū' | 'ú' | 'ǔ' | 'ù' | 'u' => Some('u'),
            'ǖ' | 'ǘ' | 'ǚ' | 'ǜ' | 'ü' | 'v' => Some('u'),
            'ń' | 'ň' | 'ǹ' | 'n' => Some('n'),
            'ḿ' | 'm' => Some('m'),
            '\u{0304}' | '\u{030C}' | '\u{0300}' => None,
            other => Some(other),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_common_syllables() {
        assert_eq!(numeric_to_display("ni3 hao3"), "nǐ hǎo");
        assert_eq!(numeric_to_display("xie4 xie5"), "xiè xie");
        assert_eq!(numeric_to_display("er2"), "ér");
        assert_eq!(numeric_to_display("yi1"), "yī");
    }

    #[test]
    fn applies_standard_vowel_priority() {
        // a > e > ou > last vowel
        assert_eq!(numeric_to_display("zhuang4"), "zhuàng");
        assert_eq!(numeric_to_display("xue2"), "xué");
        assert_eq!(numeric_to_display("liu2"), "liú");
        assert_eq!(numeric_to_display("gui4"), "guì");
        assert_eq!(numeric_to_display("dou4"), "dòu");
        assert_eq!(numeric_to_display("zuo4"), "zuò");
    }

    #[test]
    fn handles_umlaut_notation() {
        assert_eq!(numeric_to_display("nu:3"), "nǚ");
        assert_eq!(numeric_to_display("lu:4"), "lǜ");
        assert_eq!(numeric_to_display("Lu:4"), "Lǜ");
        assert_eq!(numeric_to_display("nu:e4"), "nüè");
    }

    #[test]
    fn preserves_case_and_untoneable_tokens() {
        assert_eq!(numeric_to_display("Zhong1 guo2"), "Zhōng guó");
        assert_eq!(numeric_to_display("A"), "A");
        assert_eq!(numeric_to_display("A cai4"), "A cài");
        assert_eq!(numeric_to_display("A · Di4"), "A · Dì");
        assert_eq!(
            numeric_to_display("yi1 bu4 zuo4 , er4 bu4 xiu1"),
            "yī bù zuò , èr bù xiū"
        );
    }

    #[test]
    fn marks_syllabic_nasals() {
        assert_eq!(numeric_to_display("m2"), "ḿ");
        assert_eq!(numeric_to_display("n4"), "ǹ");
        assert_eq!(numeric_to_display("hng5"), "hng");
    }

    #[test]
    fn normalises_for_search() {
        assert_eq!(
            normalize_for_search("ni3 hao3"),
            ("ni hao".into(), "nihao".into())
        );
        assert_eq!(
            normalize_for_search("nǐ hǎo"),
            ("ni hao".into(), "nihao".into())
        );
        assert_eq!(
            normalize_for_search("Nihao"),
            ("nihao".into(), "nihao".into())
        );
        assert_eq!(normalize_for_search("lu:4"), ("lu".into(), "lu".into()));
        assert_eq!(normalize_for_search("  "), (String::new(), String::new()));
    }

    #[test]
    fn builds_plain_and_compact_forms() {
        let plain = numeric_to_plain("ni3 hao3");
        assert_eq!(plain, "ni hao");
        assert_eq!(compact(&plain), "nihao");

        // Middle dots and commas are dropped; they are not searchable.
        assert_eq!(
            numeric_to_plain("Ya4 li4 shan1 da4 · Du4 bu4 qie1 ke4"),
            "ya li shan da du bu qie ke"
        );
        assert_eq!(numeric_to_plain("lu:4"), "lu");
        assert_eq!(compact("lu"), "lu");
    }
}
