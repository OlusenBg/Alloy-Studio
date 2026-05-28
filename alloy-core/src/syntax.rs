//! Tree-sitter syntax parsing and highlighting registry.

use dashmap::DashMap;
use tree_sitter::{InputEdit, Language, Parser, Tree};

// ---------------------------------------------------------------------------
// HighlightRange
// ---------------------------------------------------------------------------

/// A byte-range span annotated with a highlight category name.
#[derive(Debug, Clone)]
pub struct HighlightRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: u32,
    pub end_line: u32,
    /// Highlight category, e.g. `"keyword"`, `"string"`, `"comment"`.
    pub highlight_name: &'static str,
}

// ---------------------------------------------------------------------------
// SyntaxLayer
// ---------------------------------------------------------------------------

/// A per-document tree-sitter parsing context.
pub struct SyntaxLayer {
    language_id: String,
    parser: Parser,
    tree: Option<Tree>,
}

impl SyntaxLayer {
    /// Create a new layer for the given language identifier.
    ///
    /// Returns an error when no [`Language`] is registered for `language_id`.
    pub fn new(language_id: &str) -> anyhow::Result<Self> {
        let lang = SyntaxRegistry::language_for_id(language_id)
            .ok_or_else(|| anyhow::anyhow!("no tree-sitter language for '{language_id}'"))?;

        let mut parser = Parser::new();
        parser
            .set_language(&lang)
            .map_err(|e| anyhow::anyhow!("set_language error: {e}"))?;

        Ok(Self {
            language_id: language_id.to_owned(),
            parser,
            tree: None,
        })
    }

    /// Parse the full document text (discards any prior tree).
    pub fn parse_full(&mut self, text: &str) -> anyhow::Result<()> {
        self.tree = Some(self.parser.parse(text.as_bytes(), None).ok_or_else(|| {
            anyhow::anyhow!("tree-sitter parse_full failed for '{}'", self.language_id)
        })?);
        Ok(())
    }

    /// Apply `edit` to the existing tree and re-parse incrementally.
    ///
    /// If no prior tree exists, falls back to a full parse.
    pub fn parse_incremental(&mut self, text: &str, edit: &InputEdit) -> anyhow::Result<()> {
        if let Some(ref mut old_tree) = self.tree {
            old_tree.edit(edit);
        }

        let old_tree = self.tree.as_ref();
        self.tree = Some(
            self.parser
                .parse(text.as_bytes(), old_tree)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "tree-sitter parse_incremental failed for '{}'",
                        self.language_id
                    )
                })?,
        );
        Ok(())
    }

    /// Return the S-expression representation of the current parse tree root,
    /// useful for debugging.
    pub fn root_node_sexp(&self) -> Option<String> {
        self.tree.as_ref().map(|t| t.root_node().to_sexp())
    }
}

// ---------------------------------------------------------------------------
// SyntaxRegistry
// ---------------------------------------------------------------------------

/// A thread-safe registry that maps document URIs to their [`SyntaxLayer`]s.
pub struct SyntaxRegistry {
    layers: DashMap<String, parking_lot::Mutex<SyntaxLayer>>,
}

impl SyntaxRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            layers: DashMap::new(),
        }
    }

    /// Ensure a [`SyntaxLayer`] exists for `uri`.
    ///
    /// If a layer already exists it is left unchanged (language changes are
    /// not yet handled — close and re-open the document instead).
    pub fn get_or_create(&self, uri: &str, language_id: &str) -> anyhow::Result<()> {
        if !self.layers.contains_key(uri) {
            let layer = SyntaxLayer::new(language_id)?;
            self.layers
                .insert(uri.to_owned(), parking_lot::Mutex::new(layer));
        }
        Ok(())
    }

    /// Re-parse the document identified by `uri` using the full `full_text`.
    ///
    /// Silently does nothing if no layer is registered for `uri`.
    pub fn update(&self, uri: &str, full_text: &str) -> anyhow::Result<()> {
        if let Some(entry) = self.layers.get(uri) {
            entry.lock().parse_full(full_text)?;
        }
        Ok(())
    }

    /// Remove the syntax layer for `uri` (e.g. when the document is closed).
    pub fn remove(&self, uri: &str) {
        self.layers.remove(uri);
    }

    /// Map a language identifier to its tree-sitter [`Language`].
    ///
    /// Returns `None` for unsupported languages instead of panicking.
    pub fn language_for_id(id: &str) -> Option<Language> {
        match id {
            "java" => Some(tree_sitter_java::language()),
            _ => None,
        }
    }
}

impl Default for SyntaxRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_for_java() {
        assert!(SyntaxRegistry::language_for_id("java").is_some());
    }

    #[test]
    fn test_language_for_unknown() {
        assert!(SyntaxRegistry::language_for_id("cobol").is_none());
    }

    #[test]
    fn test_new_layer_unsupported_lang() {
        let err = SyntaxLayer::new("brainfuck");
        assert!(err.is_err());
    }

    #[test]
    fn test_parse_full_java() {
        let mut layer = SyntaxLayer::new("java").unwrap();
        layer.parse_full("class Foo {}").unwrap();
        let sexp = layer.root_node_sexp().unwrap();
        assert!(sexp.contains("program") || sexp.contains("class_declaration"));
    }

    #[test]
    fn test_registry_get_or_create_and_update() {
        let reg = SyntaxRegistry::new();
        reg.get_or_create("file:///Foo.java", "java").unwrap();
        reg.update("file:///Foo.java", "class Foo {}").unwrap();
        reg.remove("file:///Foo.java");
        // After removal, update should be a no-op (not an error).
        reg.update("file:///Foo.java", "class Foo {}").unwrap();
    }
}
