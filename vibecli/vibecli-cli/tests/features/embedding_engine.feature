Feature: LocalEmbeddingEngine — fixed-dimension feature-hashed TF-IDF
  The original engine produced TF-IDF vectors whose length grew with the
  vocabulary, which silently broke cosine similarity (the helper returns 0
  on length mismatch) any time two vectors were embedded at different
  vocabulary states. The fix is feature hashing: tokens map to a fixed
  number of buckets so every embedding has the same dimension.

  Scenario: with_dim sets the embedding dimension
    Given a local embedding engine with dimension 256
    When document "hello world" is added
    And "hello world" is embedded as "v"
    Then "v" has length 256

  Scenario: Embedding dimension stays constant as vocabulary grows
    Given a local embedding engine with dimension 512
    When document "the quick brown fox" is added and embedded as "v1"
    And 100 unrelated documents are added
    And document "the quick brown fox" is embedded as "v2"
    Then "v1" has length 512
    And "v2" has length 512

  Scenario: Repeat-embedding the same content survives vocabulary growth
    Given a local embedding engine with dimension 512
    When document "the quick brown fox jumps over the lazy dog" is added and embedded as "v1"
    And 100 unrelated documents are added
    And document "the quick brown fox jumps over the lazy dog" is embedded as "v2"
    Then cosine similarity between "v1" and "v2" is at least 0.99

  Scenario: Related content scores higher than unrelated content
    Given a local embedding engine with dimension 512
    When document "alpha beta gamma delta epsilon" is added and embedded as "anchor"
    And document "alpha beta gamma delta zeta" is added and embedded as "near"
    And document "xylophone harmonica saxophone trumpet drum" is added and embedded as "far"
    Then cosine "anchor" "near" is greater than cosine "anchor" "far"
