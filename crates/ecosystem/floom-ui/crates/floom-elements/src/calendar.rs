pub use floom_core_theme::component_tokens::CalendarTokens;
pub use floom_core_theme::props::calendar::{CalendarDate, CalendarProps};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_calendar::CupertinoCalendar as Calendar;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_calendar::CupertinoCalendarStyle as CalendarStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_calendar::FluentCalendar as Calendar;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_calendar::FluentCalendarStyle as CalendarStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_calendar::MdCalendar as Calendar;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_calendar::MdCalendarStyle as CalendarStyle;

/// Runtime-dispatch `Calendar` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::calendar::{CalendarDate, CalendarProps};

    use floom_widget_md_calendar::MdCalendar;
    use floom_widget_cupertino_calendar::CupertinoCalendar;
    use floom_widget_fluent_calendar::FluentCalendar;

    pub struct Calendar {
        value: CalendarDate,
        show_week_numbers: bool,
        on_change: Option<Box<dyn Fn(CalendarDate)>>,
    }

    impl Calendar {
        pub fn new() -> Self {
            let props = CalendarProps::default();
            Self {
                value: props.value,
                show_week_numbers: props.show_week_numbers,
                on_change: None,
            }
        }

        pub fn value(mut self, value: CalendarDate) -> Self {
            self.value = value;
            self
        }

        pub fn show_week_numbers(mut self, show_week_numbers: bool) -> Self {
            self.show_week_numbers = show_week_numbers;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(CalendarDate) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self
                .on_change
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdCalendar::new()
                    .value(self.value)
                    .show_week_numbers(self.show_week_numbers)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoCalendar::new()
                    .value(self.value)
                    .show_week_numbers(self.show_week_numbers)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentCalendar::new()
                    .value(self.value)
                    .show_week_numbers(self.show_week_numbers)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Calendar {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Calendar;
#[cfg(feature = "multi")]
pub use floom_widget_md_calendar::MdCalendarStyle as CalendarStyle;
