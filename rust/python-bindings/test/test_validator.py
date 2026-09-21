import os
import pytest
import sys
import twitter_text
import yaml

def test_ctor():
    validator = twitter_text.Validator()
    assert validator is not None

def test_accessors():
    validator = twitter_text.Validator()

    assert validator.get_max_tweet_length() == 280

    assert validator.get_short_url_length() == 23
    validator.set_short_url_length(42)
    assert validator.get_short_url_length() == 42

    assert validator.get_short_url_length_https() == 23
    validator.set_short_url_length_https(43)
    assert validator.get_short_url_length_https() == 43

def test_runic_weighting():
    # Runes weigh one unit each under v4, two under v3 and under the default.
    # https://github.com/twitter/twitter-text/issues/430
    runes = "\u16A0\u16A2\u16A6\u16A8\u16B1\u16B2"

    v4 = twitter_text.TwitterTextParser.parse(
        runes, twitter_text.TwitterTextConfiguration.config_v4(), True)
    assert v4.weighted_length == 6
    assert v4.is_valid

    v3 = twitter_text.TwitterTextParser.parse(
        runes, twitter_text.TwitterTextConfiguration.config_v3(), True)
    assert v3.weighted_length == 12

    default = twitter_text.TwitterTextParser.parse(
        runes, twitter_text.TwitterTextConfiguration(), True)
    assert default.weighted_length == 12


def test_yaml():
    validator = twitter_text.Validator()

    with open (r"rust/conformance/tests/validate.yml") as file:
        testmap = yaml.load(file, Loader=yaml.FullLoader)

        assert len(testmap["tests"]["tweets"]) > 0
        for test in testmap["tests"]["tweets"]:
            assert test["expected"] == validator.is_valid_tweet(test["text"])

        assert len(testmap["tests"]["usernames"]) > 0
        for test in testmap["tests"]["usernames"]:
            assert test["expected"] == validator.is_valid_username(test["text"])

        assert len(testmap["tests"]["lists"]) > 0
        for test in testmap["tests"]["lists"]:
            assert test["expected"] == validator.is_valid_list(test["text"])

        assert len(testmap["tests"]["hashtags"]) > 0
        for test in testmap["tests"]["hashtags"]:
            assert test["expected"] == validator.is_valid_hashtag(test["text"])

        assert len(testmap["tests"]["urls"]) > 0
        for test in testmap["tests"]["urls"]:
            assert test["expected"] == validator.is_valid_url(test["text"])

        assert len(testmap["tests"]["urls_without_protocol"]) > 0
        for test in testmap["tests"]["urls_without_protocol"]:
            assert test["expected"] == validator.is_valid_url_without_protocol(test["text"])

def validate_weighting(tests, config):
    assert len(tests) > 0
    for test in tests:
        result = twitter_text.TwitterTextParser.parse(test["text"], config, True)
        assert test["expected"]["weightedLength"] == result.weighted_length
        assert test["expected"]["valid"] == result.is_valid
        assert test["expected"]["permillage"] == result.permillage
        assert test["expected"]["displayRangeStart"] == result.display_text_range.start
        assert test["expected"]["displayRangeEnd"] == result.display_text_range.end
        assert test["expected"]["validRangeStart"] == result.valid_text_range.start
        assert test["expected"]["validRangeEnd"] == result.valid_text_range.end

def test_weighted():
    validator = twitter_text.Validator()

    with open (r"rust/conformance/tests/validate.yml") as file:
        testmap = yaml.load(file, Loader=yaml.FullLoader)

        validate_weighting(testmap["tests"]["WeightedTweetsCounterTest"], twitter_text.TwitterTextConfiguration.config_v2())
        validate_weighting(testmap["tests"]["WeightedTweetsWithDiscountedEmojiCounterTest"], twitter_text.TwitterTextConfiguration.config_v3())
        validate_weighting(testmap["tests"]["UnicodeDirectionalMarkerCounterTest"], twitter_text.TwitterTextConfiguration.config_v3())

if __name__ == "__main__":
    raise SystemExit(pytest.main([__file__]))