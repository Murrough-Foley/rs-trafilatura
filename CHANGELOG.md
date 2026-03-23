# Changelog

## 0.1.0

Initial public release.

- Page type classification (7 types: article, forum, product, collection, listing, documentation, service)
- ML classifier (Random Forest, 200 trees, 163 features)
- Per-type extraction profiles with specialized boilerplate removal
- Extraction quality confidence scoring (0.0-1.0)
- Markdown output support (GitHub Flavored Markdown)
- Bottom-up paragraph scorer (Readability-inspired)
- Rich metadata extraction (JSON-LD, Open Graph, Dublin Core, HTML meta)
- Image extraction with hero detection, alt text, and captions
- Character encoding detection for byte input
- CLI binary (`extract_stdin`) for pipeline integration
