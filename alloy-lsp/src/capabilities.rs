//! Build and query LSP client/server capabilities.

use lsp_types::{
    ClientCapabilities, CompletionClientCapabilities, CompletionItemCapability,
    DiagnosticClientCapabilities, DocumentSymbolClientCapabilities,
    DynamicRegistrationClientCapabilities, GotoCapability, HoverClientCapabilities,
    PublishDiagnosticsClientCapabilities, ServerCapabilities, TextDocumentClientCapabilities,
    TextDocumentSyncKind, WindowClientCapabilities, WorkspaceClientCapabilities,
    WorkspaceSymbolClientCapabilities,
};

/// Build `ClientCapabilities` that represent Alloy Studio's feature set.
pub fn make_client_capabilities() -> ClientCapabilities {
    ClientCapabilities {
        text_document: Some(TextDocumentClientCapabilities {
            synchronization: Some(lsp_types::TextDocumentSyncClientCapabilities {
                dynamic_registration: Some(false),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                did_save: Some(true),
            }),
            completion: Some(CompletionClientCapabilities {
                dynamic_registration: Some(false),
                completion_item: Some(CompletionItemCapability {
                    snippet_support: Some(false),
                    commit_characters_support: Some(false),
                    documentation_format: Some(vec![lsp_types::MarkupKind::PlainText]),
                    deprecated_support: Some(false),
                    preselect_support: Some(true),
                    ..Default::default()
                }),
                context_support: Some(true),
                ..Default::default()
            }),
            hover: Some(HoverClientCapabilities {
                dynamic_registration: Some(false),
                content_format: Some(vec![lsp_types::MarkupKind::PlainText]),
            }),
            definition: Some(GotoCapability {
                dynamic_registration: Some(false),
                link_support: Some(false),
            }),
            references: Some(DynamicRegistrationClientCapabilities {
                dynamic_registration: Some(false),
            }),
            document_symbol: Some(DocumentSymbolClientCapabilities {
                dynamic_registration: Some(false),
                symbol_kind: None,
                hierarchical_document_symbol_support: Some(false),
                ..Default::default()
            }),
            formatting: Some(DynamicRegistrationClientCapabilities {
                dynamic_registration: Some(false),
            }),
            publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                related_information: Some(false),
                tag_support: None,
                version_support: Some(false),
                code_description_support: Some(false),
                data_support: Some(false),
            }),
            diagnostic: Some(DiagnosticClientCapabilities {
                dynamic_registration: Some(false),
                related_document_support: Some(false),
            }),
            ..Default::default()
        }),
        workspace: Some(WorkspaceClientCapabilities {
            workspace_folders: Some(true),
            configuration: Some(true),
            symbol: Some(WorkspaceSymbolClientCapabilities {
                dynamic_registration: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        }),
        window: Some(WindowClientCapabilities {
            work_done_progress: Some(false),
            show_message: None,
            show_document: None,
        }),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Server capability query helpers
// ---------------------------------------------------------------------------

/// Returns `true` if the server advertises completion support.
pub fn supports_completion(caps: &ServerCapabilities) -> bool {
    caps.completion_provider.is_some()
}

/// Returns `true` if the server advertises hover support.
pub fn supports_hover(caps: &ServerCapabilities) -> bool {
    matches!(caps.hover_provider, Some(lsp_types::HoverProviderCapability::Simple(true))
        | Some(lsp_types::HoverProviderCapability::Options(_)))
}

/// Returns `true` if the server advertises go-to-definition support.
pub fn supports_goto_definition(caps: &ServerCapabilities) -> bool {
    matches!(
        caps.definition_provider,
        Some(lsp_types::OneOf::Left(true)) | Some(lsp_types::OneOf::Right(_))
    )
}

/// Returns `true` if the server advertises document formatting support.
pub fn supports_formatting(caps: &ServerCapabilities) -> bool {
    matches!(
        caps.document_formatting_provider,
        Some(lsp_types::OneOf::Left(true)) | Some(lsp_types::OneOf::Right(_))
    )
}

/// Return the `TextDocumentSyncKind` advertised by the server, defaulting to `None`.
pub fn text_document_sync_kind(caps: &ServerCapabilities) -> TextDocumentSyncKind {
    match &caps.text_document_sync {
        Some(lsp_types::TextDocumentSyncCapability::Kind(k)) => *k,
        Some(lsp_types::TextDocumentSyncCapability::Options(opts)) => {
            opts.change.unwrap_or(TextDocumentSyncKind::NONE)
        }
        None => TextDocumentSyncKind::NONE,
    }
}
