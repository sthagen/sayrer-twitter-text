// Copyright 2025 Robert Sayre
// Licensed under the Apache License, Version 2.0
// http://www.apache.org/licenses/LICENSE-2.0

pub mod autolinker;
pub mod entity;
pub mod extractor;
pub mod hit_highlighter;
pub mod nom_parser;
pub mod tlds;
pub mod validator;

#[cfg(feature = "ffi")]
pub mod ffi;

use extractor::{Extract, ValidatingExtractor};
use twitter_text_config::Configuration;
use twitter_text_config::Range;

// Re-export ParserBackend for convenience
pub use extractor::ParserBackend;

/// A struct that represents a parsed tweet containing the length of the tweet,
/// its validity, display ranges etc. The name mirrors Twitter's Java implementation.
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub struct TwitterTextParseResults {
    /// The weighted length is the number used to determine the tweet's length for the purposes of Twitter's limit of 280. Most characters count
    /// for 2 units, while a few ranges (like ASCII and Latin-1) count for 1. See [Twitter's blog post](https://blog.twitter.com/official/en_us/topics/product/2017/Giving-you-more-characters-to-express-yourself.html).
    pub weighted_length: i32,

    /// The weighted length expressed as a number relative to a limit of 1000.
    /// This value makes it easier to implement UI like Twitter's tweet-length meter.
    pub permillage: i32,

    /// Whether the tweet is valid: its weighted length must be under the configured limit, it must
    /// not be empty, and it must not contain invalid characters.
    pub is_valid: bool,

    /// The display range expressed in UTF-16.
    pub display_text_range: Range,

    /// The valid display range expressed in UTF-16. After the end of the valid range, clients
    /// typically stop highlighting entities, etc.
    pub valid_text_range: Range,
}

impl TwitterTextParseResults {
    /// A new TwitterTextParseResults struct with all fields supplied as arguments.
    pub fn new(
        weighted_length: i32,
        permillage: i32,
        is_valid: bool,
        display_text_range: Range,
        valid_text_range: Range,
    ) -> TwitterTextParseResults {
        TwitterTextParseResults {
            weighted_length,
            permillage,
            is_valid,
            display_text_range,
            valid_text_range,
        }
    }

    /// An invalid TwitterTextParseResults struct. This function produces the return value when
    /// empty text or invalid UTF-8 is supplied to parse().
    pub fn empty() -> TwitterTextParseResults {
        TwitterTextParseResults {
            weighted_length: 0,
            permillage: 0,
            is_valid: false,
            display_text_range: Range::empty(),
            valid_text_range: Range::empty(),
        }
    }
}

/// Produce a [TwitterTextParseResults] struct from a [str]. If extract_urls is true, the weighted
/// length will give all URLs the weight supplied in [Configuration](twitter_text_configuration::Configuration),
/// regardless of their length.
///
/// This function uses the default parser backend (Nom). Use [parse_with_parser_backend] to
/// specify a different parsing strategy.
///
/// This function will allocate an NFC-normalized copy of the input string. If the text is already
/// NFC-normalized, [ValidatingExtractor::new_with_nfc_input] will be more efficient.
pub fn parse(text: &str, config: &Configuration, extract_urls: bool) -> TwitterTextParseResults {
    parse_with_parser_backend(text, config, extract_urls, ParserBackend::default())
}

/// Produce a [TwitterTextParseResults] struct from a [str] using the specified parser backend.
///
/// If extract_urls is true, the weighted length will give all URLs the weight supplied in
/// [Configuration](twitter_text_configuration::Configuration), regardless of their length.
///
/// The `parser_backend` parameter controls how TLDs are validated:
/// - [ParserBackend::Pest]: Trust the Pest grammar's TLD matching (original behavior)
/// - [ParserBackend::External]: Use phf lookup for O(1) TLD validation
/// - [ParserBackend::Nom]: Nom parser with external TLD/emoji validation (default, fastest)
///
/// This function will allocate an NFC-normalized copy of the input string. If the text is already
/// NFC-normalized, [ValidatingExtractor::new_with_nfc_input_and_parser_backend] will be more efficient.
pub fn parse_with_parser_backend(
    text: &str,
    config: &Configuration,
    extract_urls: bool,
    parser_backend: ParserBackend,
) -> TwitterTextParseResults {
    let mut extractor = ValidatingExtractor::with_parser_backend(config, parser_backend);
    let input = extractor.prep_input(text);
    if extract_urls {
        extractor
            .extract_urls_with_indices(input.as_str())
            .parse_results
    } else {
        extractor.extract_scan(input.as_str()).parse_results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_length_mixed_unicode_and_emoji() {
        // Test case from conformance suite that requires v2 config
        // Text: "H🐱☺👨‍👩‍👧‍👦"
        // Expected weighted_length: 16
        let config = twitter_text_config::config_v2();
        let text = "H🐱☺👨‍👩‍👧‍👦";
        let result = parse(text, config, false);

        assert_eq!(
            result.weighted_length, 16,
            "Mixed single/double byte Unicode and emoji family counting is incorrect"
        );
        assert!(result.is_valid);
        assert_eq!(result.permillage, 57);
    }

    #[test]
    fn test_weighted_length_emoji_with_skin_tone_modifiers() {
        // Test case from conformance suite that requires v2 config
        // Text: "🙋🏽👨‍🎤"
        // Expected weighted_length: 9
        let config = twitter_text_config::config_v2();
        let text = "🙋🏽👨‍🎤";
        let result = parse(text, config, false);

        assert_eq!(
            result.weighted_length, 9,
            "Emoji with skin tone modifiers counting is incorrect"
        );
        assert!(result.is_valid);
        assert_eq!(result.permillage, 32);
    }

    #[test]
    fn test_weighted_length_mixed_unicode_and_emoji_v3() {
        // Same test as above but with v3 config (emoji parsing enabled)
        // Text: "H🐱☺👨‍👩‍👧‍👦"
        // With v3 config, emoji families are counted as single units
        let config = twitter_text_config::config_v3();
        let text = "H🐱☺👨‍👩‍👧‍👦";
        let result = parse(text, config, false);

        assert_eq!(
            result.weighted_length, 7,
            "V3: Mixed single/double byte Unicode and emoji family counting is incorrect"
        );
        assert!(result.is_valid);
        assert_eq!(result.permillage, 25);
    }

    #[test]
    fn test_weighted_length_emoji_with_skin_tone_modifiers_v3() {
        // Same test as above but with v3 config (emoji parsing enabled)
        // Text: "🙋🏽👨‍🎤"
        // With v3 config, emojis with modifiers are counted as single units
        let config = twitter_text_config::config_v3();
        let text = "🙋🏽👨‍🎤";
        let result = parse(text, config, false);

        assert_eq!(
            result.weighted_length, 4,
            "V3: Emoji with skin tone modifiers counting is incorrect"
        );
        assert!(result.is_valid);
        assert_eq!(result.permillage, 14);
    }

    /// Runic is an alphabetic script, so under v4 its code points weigh the same
    /// as Latin letters rather than the default weight used for logographic
    /// scripts. v3 keeps the upstream weighting.
    /// See https://github.com/twitter/twitter-text/issues/430
    #[test]
    fn test_weighted_length_runic_counts_as_one_in_v4() {
        // Elder Futhark, plus the runic word separators at the end of the block.
        let text = "ᚠᚢᚦᚨᚱᚲᚷᚹᚺᚾᛁᛃᛇᛈᛉᛊᛏᛒᛖᛗᛚᛜᛞᛟ᛫᛬᛭";
        assert_eq!(text.chars().count(), 27);

        let v4 = parse(text, twitter_text_config::config_v4(), false);
        assert_eq!(v4.weighted_length, 27, "runes should weigh one unit each");
        assert!(v4.is_valid);

        // v3 tracks upstream, where the Runic block takes the default weight.
        let v3 = parse(text, twitter_text_config::config_v3(), false);
        assert_eq!(v3.weighted_length, 54);

        // v4 has to be asked for: the default configuration is still v3.
        let default = twitter_text_config::default();
        assert_eq!(default.version, 3);
        assert_eq!(parse(text, default, false).weighted_length, 54);
    }

    /// A passage transliterated one-to-one from Latin into Futhark has the same
    /// number of code points, so it must also fit within the Tweet length limit.
    #[test]
    fn test_transliterated_futhark_passage_is_valid() {
        let latin = "the hazelnut and the walnut and the beechnut all fall from \
                     the trees in the autumn and the squirrels gather them and \
                     hide them beneath the roots where the snow will keep them \
                     safe until the hungry days of the late winter arrive";
        let futhark = "ᛏᚺᛖ ᚺᚨᛉᛖᛚᚾᚢᛏ ᚨᚾᛞ ᛏᚺᛖ ᚹᚨᛚᚾᚢᛏ ᚨᚾᛞ ᛏᚺᛖ ᛒᛖᛖᚲᚺᚾᚢᛏ ᚨᛚᛚ ᚠᚨᛚᛚ ᚠᚱᛟᛗ \
                       ᛏᚺᛖ ᛏᚱᛖᛖᛊ ᛁᚾ ᛏᚺᛖ ᚨᚢᛏᚢᛗᚾ ᚨᚾᛞ ᛏᚺᛖ ᛊᚲᚢᛁᚱᚱᛖᛚᛊ ᚷᚨᛏᚺᛖᚱ ᛏᚺᛖᛗ ᚨᚾᛞ \
                       ᚺᛁᛞᛖ ᛏᚺᛖᛗ ᛒᛖᚾᛖᚨᛏᚺ ᛏᚺᛖ ᚱᛟᛟᛏᛊ ᚹᚺᛖᚱᛖ ᛏᚺᛖ ᛊᚾᛟᚹ ᚹᛁᛚᛚ ᚲᛖᛖᛈ ᛏᚺᛖᛗ \
                       ᛊᚨᚠᛖ ᚢᚾᛏᛁᛚ ᛏᚺᛖ ᚺᚢᚾᚷᚱᚤ ᛞᚨᚤᛊ ᛟᚠ ᛏᚺᛖ ᛚᚨᛏᛖ ᚹᛁᚾᛏᛖᚱ ᚨᚱᚱᛁᚡᛖ";
        assert_eq!(latin.chars().count(), futhark.chars().count());

        let config = twitter_text_config::config_v4();
        let latin_result = parse(latin, config, false);
        let futhark_result = parse(futhark, config, false);

        assert!(latin_result.is_valid);
        assert_eq!(
            futhark_result.weighted_length, latin_result.weighted_length,
            "a one-to-one transliteration should not change the weighted length"
        );
        assert!(futhark_result.is_valid);

        // The same passage is still rejected under the default configuration,
        // which weighs each of the 185 runes as two and each of the 42 spaces
        // as one.
        let default_result = parse(futhark, twitter_text_config::default(), false);
        assert_eq!(default_result.weighted_length, 185 * 2 + 42);
        assert!(!default_result.is_valid);
    }

    /// The weighted length fast path may only answer for the code points the
    /// leading range actually covers, however narrow that range is.
    #[test]
    fn test_weighting_respects_a_narrow_leading_range() {
        // Weight 100 stops at U+007F, so U+0101 takes the default weight of 200.
        let config = Configuration::configuration_from_json(
            r#"{"version": 3, "maxWeightedTweetLength": 280, "scale": 100,
                "defaultWeight": 200, "transformedURLLength": 23,
                "ranges": [{"start": 0, "end": 127, "weight": 100}]}"#,
        );

        assert_eq!(parse("a", &config, false).weighted_length, 1);
        assert_eq!(parse("ā", &config, false).weighted_length, 2);
        assert_eq!(parse("aā", &config, false).weighted_length, 3);
    }

    /// A configuration whose ranges start above 0 has no fast path, so every
    /// code point has to be looked up in the range list rather than assumed to
    /// take the default weight.
    #[test]
    fn test_weighting_without_a_leading_range() {
        // Only U+1100..U+1101 are discounted; everything else weighs 200.
        let config = Configuration::configuration_from_json(
            r#"{"version": 3, "maxWeightedTweetLength": 280, "scale": 100,
                "defaultWeight": 200, "transformedURLLength": 23,
                "ranges": [{"start": 4352, "end": 4353, "weight": 100}]}"#,
        );

        assert_eq!(parse("a", &config, false).weighted_length, 2);
        assert_eq!(parse("ᄀ", &config, false).weighted_length, 1);
        assert_eq!(parse("aᄀ", &config, false).weighted_length, 3);
    }

    /// v1 defines no ranges at all, so every code point takes the default weight.
    #[test]
    fn test_weighting_with_no_ranges() {
        let config = twitter_text_config::config_v1();

        assert_eq!(parse("hello", config, false).weighted_length, 5);
        assert_eq!(parse("ᚠᚢᚦ漢字", config, false).weighted_length, 5);
    }
}
