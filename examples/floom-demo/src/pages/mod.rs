//! The gallery's page model.
//!
//! A gallery is a list of pages, one per component, grouped into six
//! categories. Each page carries a title, a line of introduction, the
//! component's API tables, and a set of live samples.
//!
//! The samples live in the category modules -- [`actions`], [`forms`],
//! [`data`], [`feedback`], [`layout`] and [`motion`] -- because each of those
//! owns the components of one category.

pub mod actions;
pub mod data;
pub mod feedback;
pub mod forms;
pub mod layout;
pub mod motion;

use floom::floom_core::Child;

use crate::api::ApiDoc;

/// The six groups the gallery's components fall into.
///
/// The order of [`Category::ALL`] is the order the rail shows them in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Category {
    /// Buttons, toggles and the other things a reader clicks.
    Actions,
    /// The things a reader types into or picks from.
    Forms,
    /// The things a reader reads.
    DataDisplay,
    /// The things that tell a reader something happened.
    Feedback,
    /// The things that arrange the page around the content.
    Layout,
    /// The things that move, progress, and provide context to their children.
    Motion,
}

impl Category {
    /// Every category, in the order the rail shows them.
    pub const ALL: [Category; 6] = [
        Category::Actions,
        Category::Forms,
        Category::DataDisplay,
        Category::Feedback,
        Category::Layout,
        Category::Motion,
    ];

    /// The name a reader sees.
    pub fn label(self) -> &'static str {
        match self {
            Category::Actions => "Actions",
            Category::Forms => "Forms",
            Category::DataDisplay => "Data display",
            Category::Feedback => "Feedback",
            Category::Layout => "Navigation and layout",
            Category::Motion => "Motion, progress and providers",
        }
    }

    /// A short, stable name for the category.
    pub fn slug(self) -> &'static str {
        match self {
            Category::Actions => "actions",
            Category::Forms => "forms",
            Category::DataDisplay => "data-display",
            Category::Feedback => "feedback",
            Category::Layout => "layout",
            Category::Motion => "motion",
        }
    }
}

/// One live example of a component.
///
/// The builder is a plain function so a page can be built from a table; the
/// state a sample needs is created inside it and lives as long as the child
/// closures it hands to the host.
pub struct Sample {
    /// What the sample is called.
    pub title: &'static str,
    /// A caveat worth reading next to the sample.
    pub note: Option<&'static str>,
    /// The widget tree the sample shows.
    pub build: fn() -> Child,
}

/// One component's page.
pub struct ComponentPage {
    /// The key the satellite names this component after -- `text-field`,
    /// `datatable`. This is what a page module looks its component up by.
    pub component: &'static str,
    /// The name a reader sees.
    pub name: &'static str,
    /// Which category the component belongs to.
    pub category: Category,
    /// One line about what the component is for.
    pub intro: &'static str,
    /// The component's props, events and methods.
    pub api: ApiDoc,
    /// The live examples.
    pub samples: Vec<Sample>,
}

/// A page for a component that has no samples yet.
///
/// The category modules build their pages through this until each component's
/// samples are written.
pub fn page(
    component: &'static str,
    name: &'static str,
    category: Category,
) -> ComponentPage {
    ComponentPage {
        component,
        name,
        category,
        intro: "",
        api: ApiDoc::empty(),
        samples: Vec::new(),
    }
}
