use serde::{Deserialize, Serialize};

/// UI settings for fields
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiSettings {
    /// Placeholder text for input fields
    pub placeholder: Option<String>,
    
    /// Help text to display in UI
    pub help_text: Option<String>,
    
    /// Whether to hide this field in list views
    pub hide_in_lists: Option<bool>,
    
    /// Layout width (1-12 for 12-column grid)
    pub width: Option<u8>,
    
    /// Order in form (lower numbers appear first)
    pub order: Option<i32>,
    
    /// Group name for visually grouping fields
    pub group: Option<String>,
    
    /// CSS class names to apply to field
    pub css_class: Option<String>,
    
    /// For WYSIWYG: toolbar configuration
    pub wysiwyg_toolbar: Option<String>,
    
    /// For input fields: input type (e.g., "password", "email")
    pub input_type: Option<String>,
} 