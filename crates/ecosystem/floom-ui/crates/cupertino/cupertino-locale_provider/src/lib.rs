use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::props::locale_provider::LocaleProviderProps;

/// Material [`LocaleProvider`]: owns a BCP-47 locale tag [`RwSignal`] and
/// shares it with its subtree through [`CupertinoLocaleProvider::locale_signal`].
///
/// NOTE: subtree-scoped locale overrides are not enforced by the type system
/// (Rust/Floom static dispatch); descendants read `locale_signal()` and react.
pub struct CupertinoLocaleProvider {
    props: LocaleProviderProps,
    locale: RwSignal<String>,
    child: Option<AnyView>,
}

impl CupertinoLocaleProvider {
    pub fn new() -> Self {
        let props = LocaleProviderProps::default();
        Self {
            locale: RwSignal::new(props.locale.clone()),
            props,
            child: None,
        }
    }

    /// Initial locale tag (e.g. `"en"`, `"zh-CN"`).
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        let l = locale.into();
        self.props.locale = l.clone();
        self.locale.set(l);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the locale tag, shared with descendants.
    pub fn locale_signal(&self) -> RwSignal<String> {
        self.locale
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &LocaleProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for CupertinoLocaleProvider {
    fn default() -> Self {
        Self::new()
    }
}
