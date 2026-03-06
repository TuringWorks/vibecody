---
triggers: ["RAG", "retrieval augmented", "embeddings", "vector store", "chunking", "semantic search", "Pinecone", "Qdrant"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# RAG Pipeline Design

When building Retrieval-Augmented Generation systems:

1. Chunk documents by semantic units (paragraphs, sections) — not fixed character counts
2. Chunk size: 256-512 tokens with 50-token overlap for context continuity
3. Use embedding models: `text-embedding-3-small` (OpenAI), `nomic-embed-text` (Ollama), `BAAI/bge-*`
4. Vector stores: Qdrant (self-hosted), Pinecone (managed), pgvector (PostgreSQL extension)
5. Retrieval: cosine similarity search — return top-K (3-5) most relevant chunks
6. Reranking: use a cross-encoder after initial retrieval to improve precision
7. Prompt template: "Context: {retrieved_chunks}\n\nQuestion: {user_query}\n\nAnswer based on the context above."
8. Include source attribution: return which documents/chunks informed the answer
9. Hybrid search: combine vector similarity with keyword (BM25) search — better recall
10. Metadata filtering: filter by date, source, category before vector search
11. Evaluation: use RAGAS (Relevancy, Answer correctness, Faithfulness) metrics
12. Index management: incremental updates, version embeddings with model name, rebuild on model change
