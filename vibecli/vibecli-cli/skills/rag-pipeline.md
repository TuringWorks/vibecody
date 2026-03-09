---
triggers: ["RAG", "retrieval augmented generation", "RAG pipeline", "knowledge base", "semantic search", "document QA", "context retrieval"]
tools_allowed: ["read_file", "write_file", "bash"]
category: ai
---

# RAG Pipeline

When building a retrieval augmented generation pipeline:

1. **Chunking strategies** — Choose based on content type: fixed-size chunks (512-1024 tokens) for homogeneous text, semantic chunking (split on topic boundaries using embedding similarity), or recursive character splitting (split by paragraphs, then sentences, then words). Always include 10-20% overlap between chunks to preserve context across boundaries.

2. **Embedding model selection** — Match the embedding model to your domain and latency budget. General-purpose models (OpenAI text-embedding-3-large, Cohere embed-v3) work well for broad corpora. Fine-tuned or domain-specific models (e.g., PubMedBERT for biomedical) improve recall on specialized content. Benchmark with your actual queries before committing.

3. **Hybrid search (dense + sparse)** — Combine dense vector similarity with sparse keyword search (BM25) for best retrieval quality. Use reciprocal rank fusion (RRF) or learned score combination to merge results. Sparse search catches exact terms and rare entities that dense embeddings may miss.

4. **Reranking with cross-encoders** — After initial retrieval (top-50 to top-100), apply a cross-encoder reranker (e.g., Cohere Rerank, BGE-reranker, ColBERT) to re-score and select the final top-k chunks. Cross-encoders attend jointly to query and passage, dramatically improving precision.

5. **Query expansion** — Generate multiple query variants (HyDE, multi-query, step-back prompting) to increase recall. HyDE generates a hypothetical answer and retrieves against it. Multi-query rewrites the original question from different angles. Combine results from all variants before reranking.

6. **Metadata filtering** — Attach structured metadata to chunks (source, date, author, section, document type) and apply pre-retrieval filters to narrow the search space. This reduces noise and improves relevance, especially in multi-tenant or multi-collection setups.

7. **Parent-child retrieval** — Index small chunks for precise matching but retrieve the parent chunk or full section for richer context. Store a parent_id on each child chunk. After retrieval, expand to the parent document window before passing to the LLM.

8. **Contextual compression** — After retrieval, compress or extract only the relevant sentences from each chunk using an LLM or extractive model. This reduces token usage and focuses the generation on the most pertinent information.

9. **Evaluation metrics** — Measure retrieval quality with recall@k, MRR (mean reciprocal rank), and NDCG (normalized discounted cumulative gain). Measure end-to-end answer quality with faithfulness (grounded in retrieved context), relevance, and correctness. Use frameworks like RAGAS or DeepEval for automated evaluation.

10. **Production deployment patterns** — Cache frequent queries and their retrieved contexts. Use async retrieval to overlap embedding computation with generation. Monitor retrieval latency, cache hit rates, and answer quality over time. Implement fallback strategies when retrieval returns low-confidence results. Version your index alongside your embedding model to avoid drift.
