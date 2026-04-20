Feature: OpenMemoryStore migrates legacy variable-dim embeddings on load
  Memories saved by the old TF-IDF engine carried `embedding` vectors whose
  length matched the vocabulary at write time. After the feature-hashing fix,
  the embedding engine emits fixed-dim vectors. Without migration, on-disk
  vectors of the wrong length silently produce zero cosine similarity (the
  helper short-circuits on length mismatch), making every query miss.
  `OpenMemoryStore::load` must regenerate any embedding whose length doesn't
  match the current engine dimension.

  Scenario: Legacy memories with shorter-than-current embeddings are re-embedded
    Given a legacy memories.json with 3 entries whose embeddings are length 7
    When the store is loaded from that directory
    Then every loaded memory has an embedding of the engine's dimension
    And no loaded memory carries the legacy length-7 embedding

  Scenario: Migrated memories are queryable and outrank unrelated content
    Given a legacy memories.json with a target "the quick brown fox" and 4 unrelated decoys all carrying length-3 embeddings
    When the store is loaded from that directory
    And the store is queried for "quick brown fox" with limit 5
    Then the top result content is "the quick brown fox"
    And the top result similarity is greater than 0

  Scenario: Memories that already match the current dimension are preserved byte-for-byte
    Given a legacy memories.json with 1 entry whose embedding is already at the engine dimension
    When the store is loaded from that directory
    Then the loaded embedding equals the on-disk embedding
