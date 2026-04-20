Feature: OpenMemoryStore is backed by the TurboQuant-compressed embedding index
  Once `LocalEmbeddingEngine` emits fixed-dim vectors, the in-memory waypoint
  index can move from raw f32 storage (HnswIndex) to a TurboQuant-quantized
  index that holds ~3 bits/dim. Cosine search is decoded on the fly. The swap
  must preserve the existing add → query → delete behaviour while exposing the
  reduced footprint for callers that care (e.g. /memory stats).

  Scenario: Compression ratio reflects TurboQuant storage
    Given a fresh OpenMemoryStore
    When 200 distinct memories are added
    Then the embedding compression ratio is at least 8.0

  Scenario: Added memories are retrievable by query
    Given a fresh OpenMemoryStore
    When the memory "the quick brown fox jumps over the lazy dog" is added
    And the store is queried for "quick brown fox" with limit 5
    Then the top result content is "the quick brown fox jumps over the lazy dog"

  Scenario: Deleting a memory removes it from the compressed index
    Given a fresh OpenMemoryStore
    When the memory "alpha beta gamma delta epsilon" is added and its id captured as "victim"
    And the memory at id "victim" is deleted
    And the store is queried for "alpha beta gamma" with limit 5
    Then no result has id equal to "victim"
