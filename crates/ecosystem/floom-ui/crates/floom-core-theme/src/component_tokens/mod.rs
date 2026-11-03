pub mod material;

mod alert;
mod app_bar;
mod autocomplete;
mod avatar;
mod badge;
mod banner;
mod bottom_navigation;
mod breadcrumbs;
mod button;
mod button_group;
mod calendar;
mod card;
mod carousel;
mod checkbox;
mod checkbox_btn;
mod chip;
mod chip_group;
mod code;
mod color_picker;
mod combobox;
mod confirm_edit;
mod container;
mod counter;
mod data_iterator;
mod datatable;
mod date_picker;
mod dialog;
mod divider;
mod empty_state;
mod expand_transition;
mod expansion_panels;
mod fade_transition;
mod file_input;
mod floating_action_button;
mod footer;
mod form;
mod grid;
mod hover;
mod icon;
mod icon_button;
mod image;
mod infinite_scroller;
mod item_group;
mod kbd;
mod list;
mod list_group;
mod list_item;
mod list_subheader;
mod menu;
mod navigation_drawer;
mod number_input;
mod otp_input;
mod overlay;
mod pagination;
mod parallax;
mod progress_circular;
mod progress_linear;
mod pull_to_refresh;
mod radio;
mod radio_group;
mod range_slider;
mod rating;
mod responsive;
mod scale_transition;
mod select;
mod sheet;
mod skeleton_loader;
mod slide_group;
mod slide_transition;
mod slider;
mod snackbar;
mod spacer;
mod sparkline;
mod speed_dial;
mod stepper;
mod switch;
mod system_bar;
mod table;
mod tabs;
mod text;
mod text_field;
mod textarea;
mod time_picker;
mod timeline;
mod toolbar;
mod tooltip;
mod treeview;
mod validation_messages;
mod virtual_scroller;
mod window;

pub use alert::AlertTokens;
pub use app_bar::AppBarTokens;
pub use autocomplete::AutocompleteTokens;
pub use avatar::AvatarTokens;
pub use badge::BadgeTokens;
pub use banner::BannerTokens;
pub use bottom_navigation::BottomNavigationTokens;
pub use breadcrumbs::BreadcrumbsTokens;
pub use button::ButtonTokens;
pub use button_group::ButtonGroupTokens;
pub use calendar::CalendarTokens;
pub use card::CardTokens;
pub use carousel::CarouselTokens;
pub use checkbox::CheckboxTokens;
pub use checkbox_btn::CheckboxBtnTokens;
pub use chip::ChipTokens;
pub use chip_group::ChipGroupTokens;
pub use code::CodeTokens;
pub use color_picker::ColorPickerTokens;
pub use combobox::ComboboxTokens;
pub use confirm_edit::ConfirmEditTokens;
pub use container::ContainerTokens;
pub use counter::CounterTokens;
pub use data_iterator::DataIteratorTokens;
pub use datatable::DataTableTokens;
pub use date_picker::DatePickerTokens;
pub use dialog::DialogTokens;
pub use divider::DividerTokens;
pub use empty_state::EmptyStateTokens;
pub use expand_transition::ExpandTransitionTokens;
pub use expansion_panels::ExpansionPanelsTokens;
pub use fade_transition::FadeTransitionTokens;
pub use file_input::FileInputTokens;
pub use floating_action_button::FloatingActionButtonTokens;
pub use footer::FooterTokens;
pub use form::FormTokens;
pub use grid::GridTokens;
pub use hover::HoverTokens;
pub use icon::IconTokens;
pub use icon_button::IconButtonTokens;
pub use image::ImageTokens;
pub use infinite_scroller::InfiniteScrollerTokens;
pub use item_group::ItemGroupTokens;
pub use kbd::KbdTokens;
pub use list::ListTokens;
pub use list_group::ListGroupTokens;
pub use list_item::ListItemTokens;
pub use list_subheader::ListSubheaderTokens;
pub use menu::MenuTokens;
pub use navigation_drawer::NavigationDrawerTokens;
pub use number_input::NumberInputTokens;
pub use otp_input::OtpInputTokens;
pub use overlay::OverlayTokens;
pub use pagination::PaginationTokens;
pub use parallax::ParallaxTokens;
pub use progress_circular::ProgressCircularTokens;
pub use progress_linear::ProgressLinearTokens;
pub use pull_to_refresh::PullToRefreshTokens;
pub use radio::RadioTokens;
pub use radio_group::RadioGroupTokens;
pub use range_slider::RangeSliderTokens;
pub use rating::RatingTokens;
pub use responsive::ResponsiveTokens;
pub use scale_transition::ScaleTransitionTokens;
pub use select::SelectTokens;
pub use sheet::SheetTokens;
pub use skeleton_loader::SkeletonTokens;
pub use slide_group::SlideGroupTokens;
pub use slide_transition::SlideTransitionTokens;
pub use slider::SliderTokens;
pub use snackbar::SnackbarTokens;
pub use spacer::SpacerTokens;
pub use sparkline::SparklineTokens;
pub use speed_dial::SpeedDialTokens;
pub use stepper::StepperTokens;
pub use switch::SwitchTokens;
pub use system_bar::SystemBarTokens;
pub use table::TableTokens;
pub use tabs::TabsTokens;
pub use text::TextTokens;
pub use text_field::TextFieldTokens;
pub use textarea::TextareaTokens;
pub use time_picker::TimePickerTokens;
pub use timeline::TimelineTokens;
pub use toolbar::ToolbarTokens;
pub use tooltip::TooltipTokens;
pub use treeview::TreeviewTokens;
pub use validation_messages::ValidationMessagesTokens;
pub use virtual_scroller::VirtualScrollerTokens;
pub use window::WindowTokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Button,
    Text,
    TextField,
    Checkbox,
    Switch,
    SystemBar,
    Dialog,
    Menu,
    Card,
    Icon,
    Divider,
    Footer,
    Grid,
    Slider,
    RadioGroup,
    Tooltip,
    Tabs,
    List,
    Select,
    Textarea,
    Snackbar,
    Spacer,
    Chip,
    Container,
    Badge,
    AppBar,
    NavigationDrawer,
    BottomNavigation,
    ProgressLinear,
    ProgressCircular,
    Avatar,
    Alert,
    Rating,
    Pagination,
    Timeline,
    Image,
    Banner,
    SkeletonLoader,
    Toolbar,
    Sheet,
    Combobox,
    Autocomplete,
    DatePicker,
    TimePicker,
    EmptyState,
    DataTable,
    Treeview,
    Calendar,
    Carousel,
    VirtualScroller,
    Form,
    ValidationMessages,
    Stepper,
    Breadcrumbs,
    Window,
    SlideGroup,
    ItemGroup,
    ColorPicker,
    ButtonGroup,
    FloatingActionButton,
    RangeSlider,
    FileInput,
    OtpInput,
    ListItem,
    Table,
    DataIterator,
    Sparkline,
    Kbd,
    Code,
    Radio,
    Overlay,
    Parallax,
    InfiniteScroller,
    Hover,
    FadeTransition,
    ScaleTransition,
    ExpandTransition,
    SlideTransition,
    IconButton,
    ExpansionPanels,
    SpeedDial,
    ChipGroup,
    ListGroup,
    Counter,
    Responsive,
    CheckboxBtn,
    ListSubheader,
    NumberInput,
    PullToRefresh,
    ConfirmEdit,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentTokens {
    Button(ButtonTokens),
    Text(TextTokens),
    TextField(TextFieldTokens),
    Checkbox(CheckboxTokens),
    Switch(SwitchTokens),
    SystemBar(SystemBarTokens),
    Dialog(DialogTokens),
    Menu(MenuTokens),
    Card(CardTokens),
    Icon(IconTokens),
    Divider(DividerTokens),
    Footer(FooterTokens),
    Grid(GridTokens),
    Slider(SliderTokens),
    RadioGroup(RadioGroupTokens),
    Tooltip(TooltipTokens),
    Tabs(TabsTokens),
    List(ListTokens),
    Select(SelectTokens),
    Textarea(TextareaTokens),
    Snackbar(SnackbarTokens),
    Spacer(SpacerTokens),
    Chip(ChipTokens),
    Container(ContainerTokens),
    Badge(BadgeTokens),
    AppBar(AppBarTokens),
    NavigationDrawer(NavigationDrawerTokens),
    BottomNavigation(BottomNavigationTokens),
    ProgressLinear(ProgressLinearTokens),
    ProgressCircular(ProgressCircularTokens),
    Avatar(AvatarTokens),
    Alert(AlertTokens),
    Rating(RatingTokens),
    Pagination(PaginationTokens),
    Timeline(TimelineTokens),
    Image(ImageTokens),
    Banner(BannerTokens),
    SkeletonLoader(SkeletonTokens),
    Toolbar(ToolbarTokens),
    Sheet(SheetTokens),
    Combobox(ComboboxTokens),
    Autocomplete(AutocompleteTokens),
    DatePicker(DatePickerTokens),
    TimePicker(TimePickerTokens),
    EmptyState(EmptyStateTokens),
    DataTable(DataTableTokens),
    Treeview(TreeviewTokens),
    Calendar(CalendarTokens),
    Carousel(CarouselTokens),
    VirtualScroller(VirtualScrollerTokens),
    Form(FormTokens),
    ValidationMessages(ValidationMessagesTokens),
    Stepper(StepperTokens),
    Breadcrumbs(BreadcrumbsTokens),
    Window(WindowTokens),
    SlideGroup(SlideGroupTokens),
    ItemGroup(ItemGroupTokens),
    ColorPicker(ColorPickerTokens),
    ButtonGroup(ButtonGroupTokens),
    FloatingActionButton(FloatingActionButtonTokens),
    RangeSlider(RangeSliderTokens),
    FileInput(FileInputTokens),
    OtpInput(OtpInputTokens),
    ListItem(ListItemTokens),
    Table(TableTokens),
    DataIterator(DataIteratorTokens),
    Sparkline(SparklineTokens),
    Kbd(KbdTokens),
    Code(CodeTokens),
    Radio(RadioTokens),
    Overlay(OverlayTokens),
    Parallax(ParallaxTokens),
    InfiniteScroller(InfiniteScrollerTokens),
    Hover(HoverTokens),
    FadeTransition(FadeTransitionTokens),
    ScaleTransition(ScaleTransitionTokens),
    ExpandTransition(ExpandTransitionTokens),
    SlideTransition(SlideTransitionTokens),
    IconButton(IconButtonTokens),
    ExpansionPanels(ExpansionPanelsTokens),
    SpeedDial(SpeedDialTokens),
    ChipGroup(ChipGroupTokens),
    ListGroup(ListGroupTokens),
    Counter(CounterTokens),
    Responsive(ResponsiveTokens),
    CheckboxBtn(CheckboxBtnTokens),
    ListSubheader(ListSubheaderTokens),
    NumberInput(NumberInputTokens),
    PullToRefresh(PullToRefreshTokens),
    ConfirmEdit(ConfirmEditTokens),
}
