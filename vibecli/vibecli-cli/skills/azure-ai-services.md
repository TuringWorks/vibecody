---
triggers: ["Azure OpenAI", "azure ai", "azure cognitive", "azure ai search", "azure speech", "azure vision", "document intelligence", "prompt flow"]
tools_allowed: ["read_file", "write_file", "bash"]
requires_bins: ["az"]
category: cloud-azure
---

# Azure AI Services

When working with Azure AI Services:

1. Use Azure OpenAI Service with the OpenAI SDK by pointing to your Azure endpoint: `AzureOpenAI(azure_endpoint="https://<resource>.openai.azure.com", api_version="2024-06-01", azure_deployment="gpt-4o")` with `DefaultAzureCredential` via `get_bearer_token_provider` — deploy models with `az cognitiveservices account deployment create` and manage quota per deployment.
2. Implement rate limiting and retry handling for Azure OpenAI: respect `Retry-After` headers on 429 responses, use token-per-minute (TPM) and requests-per-minute (RPM) quotas per deployment; create multiple deployments of the same model across regions and load-balance with Azure API Management or application-level routing.
3. Use Document Intelligence (formerly Form Recognizer) for structured data extraction: `DocumentIntelligenceClient(endpoint, credential).begin_analyze_document("prebuilt-invoice", document)` returns structured fields with confidence scores; train custom models with `begin_build_document_model()` for domain-specific forms using labeled training data.
4. Implement Azure AI Speech for real-time transcription: `SpeechRecognizer(SpeechConfig.from_subscription(key, region))` with `recognized` event for final results and `recognizing` for interim; use `SpeechSynthesizer` for text-to-speech with SSML for prosody control and custom neural voices for brand consistency.
5. Use Azure AI Vision for image analysis: `ImageAnalysisClient(endpoint, credential).analyze(image_url, visual_features=[VisualFeatures.CAPTION, VisualFeatures.TAGS, VisualFeatures.OBJECTS])`; use custom models trained in Vision Studio for domain-specific object detection and image classification tasks.
6. Build RAG applications with Azure AI Search as vector store: create an index with `SearchableField` and `SearchField(type="Collection(Edm.Single)", vector_search_dimensions=1536)`; use `SearchClient.search(query, vector_queries=[VectorizedQuery(vector=embedding, fields="contentVector")])` for hybrid keyword + vector retrieval.
7. Configure AI Search indexers for automated ingestion: connect to Blob Storage, SQL Database, or Cosmos DB data sources; use built-in skillsets (`OcrSkill`, `EntityRecognitionSkill`, `SplitSkill`) for document cracking and enrichment, and `AzureOpenAIEmbeddingSkill` for automatic vectorization during indexing.
8. Use Prompt Flow for LLM application orchestration: define flows as YAML DAGs with `llm`, `python`, and `prompt` nodes; test locally with `pf flow test`, evaluate with custom metrics (`groundedness`, `relevance`, `fluency`), and deploy as managed online endpoints with `az ml online-deployment create`.
9. Implement content safety with Azure AI Content Safety: `ContentSafetyClient(endpoint, credential).analyze_text(AnalyzeTextOptions(text=input))` returns severity levels (0-6) for hate, violence, self-harm, and sexual categories; set threshold filters in Azure OpenAI content filtering configurations per deployment.
10. Use responsible AI practices: enable Azure OpenAI content filters at deployment level, implement input/output logging for abuse monitoring, add system messages with behavioral guardrails, and use the Responsible AI dashboard in Azure ML for model fairness, explainability, and error analysis on custom models.
11. Optimize costs across AI services: use provisioned throughput (PTU) for Azure OpenAI at scale (committed capacity at lower per-token cost), batch API for offline processing, Standard tier only for production (Free tier for prototyping); cache embeddings to avoid recomputation and use smallest effective models (GPT-4o-mini before GPT-4o).
12. Secure AI services with private endpoints and VNet restrictions: `az cognitiveservices account network-rule add` to restrict access; use managed identity for service-to-service authentication, store API keys in Key Vault, enable diagnostic logging for all API calls, and audit usage with `az monitor metrics list --resource <resourceId>`.
