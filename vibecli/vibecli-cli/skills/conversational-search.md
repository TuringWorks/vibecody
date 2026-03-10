# Conversational Codebase Search

Interactive conversational search engine for asking natural language questions about your codebase.

## Triggers
- "conversational search", "ask codebase", "code Q&A", "search chat"
- "devin search", "codebase question", "find and explain"

## Usage
```
/search ask "How does authentication work?"      # Ask a question
/search follow-up "What about OAuth?"            # Follow-up question
/search refine --type rs --path src/auth          # Narrow results
/search history                                   # Show conversation history
/search suggest                                   # Get follow-up suggestions
/search clear                                     # Reset conversation context
```

## Features
- 5 query types: Natural, Regex, Semantic, FollowUp, Refinement
- Conversational context maintained across queries
- Follow-up questions build on previous results
- Result refinement with file type and path filters
- Answer synthesis with confidence scoring
- Automatic follow-up question suggestions
- Evidence-based answers with code snippet references
- Topic-focused context tracking
