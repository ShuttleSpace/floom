use floem::reactive::RwSignal;
use floem_core_theme::theme::{MaterialTheme, Theme};

pub fn theme_signal() -> RwSignal<MaterialTheme> {
    RwSignal::new(MaterialTheme::new())
}

pub fn component_token(
    theme: &MaterialTheme,
    id: floem_core_theme::component_tokens::ComponentId,
) -> floem_core_theme::component_tokens::ComponentTokens {
    theme.component(id)
}
