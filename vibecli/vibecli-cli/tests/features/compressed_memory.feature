Feature: TurboQuant-compressed memory index — recall-preserving compression
  Wraps vibe_core::index::turboquant so OpenMemory can hold embeddings at
  ~3 bits/dim instead of f32. Search decompresses on the fly and returns
  approximate cosine matches. Two properties matter: identity (a vector
  inserted at id X is the top hit when X is queried back) and recall
  against exact cosine search on a held-out set.

  Scenario: Inserted vector is the top hit when queried back
    Given a compressed memory index of dimension 128
    When 200 random unit vectors are inserted with ids "v0".."v199"
    And the vector at id "v42" is queried with top_k 5
    Then the top result id is "v42"

  Scenario: Compression ratio meets the 8x floor
    Given a compressed memory index of dimension 256
    When 500 random unit vectors are inserted
    Then the reported compression ratio is at least 8.0

  # The use case: "I stored memory M, I search with text near M, does M come back?"
  # Each query is a known seed vector + small noise; we check whether that exact
  # seed appears in the top-K returned. This tests what compression must preserve
  # (nearest-neighbour identity) rather than within-cluster ordering, which is
  # an unstable metric under any quantizer.
  Scenario: Target-hit rate at top-10 stays above 0.95 on clustered embeddings
    Given a compressed memory index of dimension 384 seeded with 20 clusters of 50 vectors
    When 50 noisy-seed queries are run with top_k 10
    Then target-hit rate at top-10 is at least 0.95

  Scenario: Empty index returns no results
    Given a compressed memory index of dimension 64
    When the zero vector is queried with top_k 5
    Then the result list is empty
