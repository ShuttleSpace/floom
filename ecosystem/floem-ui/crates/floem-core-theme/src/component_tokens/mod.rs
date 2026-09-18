pub mod material;

mod alert;
mod app_bar;
mod autocomplete;
mod avatar;
mod banner;
mod badge;
mod bottom_navigation;
mod button;
mod card;
mod checkbox;
mod chip;
mod combobox;
mod container;
mod date_picker;
mod dialog;
mod divider;
mod empty_state;
mod form;
mod validation_messages;
mod stepper;
mod breadcrumbs;
mod footer;
mod grid;
mod icon;
mod image;
mod list;
mod menu;
mod navigation_drawer;
mod pagination;
mod radio_group;
mod rating;
mod select;
mod sheet;
mod skeleton_loader;
mod slider;
mod snackbar;
mod spacer;
mod switch;
mod system_bar;
mod tabs;
mod text;
mod text_field;
mod textarea;
mod time_picker;
mod timeline;
mod toolbar;
mod progress_linear;
mod progress_circular;
mod tooltip;
mod datatable;
mod treeview;
mod calendar;
mod carousel;
mod virtual_scroller;
mod window;
mod slide_group;
mod item_group;
mod color_picker;
mod button_group;
mod floating_action_button;
mod range_slider;
mod file_input;
mod otp_input;
mod list_item;
mod table;
mod data_iterator;
mod sparkline;
mod kbd;
mod code;
mod radio;
mod overlay;
mod parallax;
mod infinite_scroller;
mod hover;
mod fade_transition;
mod scale_transition;
mod expand_transition;
mod slide_transition;
mod icon_button;
mod expansion_panels;
mod speed_dial;
mod chip_group;
mod list_group;
mod counter;
mod responsive;
mod checkbox_btn;
mod list_subheader;
mod number_input;
mod pull_to_refresh;
mod confirm_edit;

pub use alert::AlertTokens;
pub use app_bar::AppBarTokens;
pub use autocomplete::AutocompleteTokens;
pub use avatar::AvatarTokens;
pub use banner::BannerTokens;
pub use badge::BadgeTokens;
pub use bottom_navigation::BottomNavigationTokens;
pub use button::ButtonTokens;
pub use card::CardTokens;
pub use checkbox::CheckboxTokens;
pub use chip::ChipTokens;
pub use combobox::ComboboxTokens;
pub use container::ContainerTokens;
pub use date_picker::DatePickerTokens;
pub use dialog::DialogTokens;
pub use divider::DividerTokens;
pub use empty_state::EmptyStateTokens;
pub use form::FormTokens;
pub use validation_messages::ValidationMessagesTokens;
pub use stepper::StepperTokens;
pub use breadcrumbs::BreadcrumbsTokens;
pub use footer::FooterTokens;
pub use grid::GridTokens;
pub use icon::IconTokens;
pub use image::ImageTokens;
pub use list::ListTokens;
pub use menu::MenuTokens;
pub use navigation_drawer::NavigationDrawerTokens;
pub use pagination::PaginationTokens;
pub use radio_group::RadioGroupTokens;
pub use rating::RatingTokens;
pub use select::SelectTokens;
pub use sheet::SheetTokens;
pub use skeleton_loader::SkeletonTokens;
pub use slider::SliderTokens;
pub use snackbar::SnackbarTokens;
pub use spacer::SpacerTokens;
pub use switch::SwitchTokens;
pub use system_bar::SystemBarTokens;
pub use tabs::TabsTokens;
pub use text::TextTokens;
pub use text_field::TextFieldTokens;
pub use textarea::TextareaTokens;
pub use time_picker::TimePickerTokens;
pub use timeline::TimelineTokens;
pub use toolbar::ToolbarTokens;
pub use progress_linear::ProgressLinearTokens;
pub use progress_circular::ProgressCircularTokens;
pub use tooltip::TooltipTokens;
pub use datatable::DataTableTokens;
pub use treeview::TreeviewTokens;
pub use calendar::CalendarTokens;
pub use carousel::CarouselTokens;
pub use virtual_scroller::VirtualScrollerTokens;
pub use window::WindowTokens;
pub use slide_group::SlideGroupTokens;
pub use item_group::ItemGroupTokens;
pub use color_picker::ColorPickerTokens;
pub use button_group::ButtonGroupTokens;
pub use floating_action_button::FloatingActionButtonTokens;
pub use range_slider::RangeSliderTokens;
pub use file_input::FileInputTokens;
pub use otp_input::OtpInputTokens;
pub use list_item::ListItemTokens;
pub use table::TableTokens;
pub use data_iterator::DataIteratorTokens;
pub use sparkline::SparklineTokens;
pub use kbd::KbdTokens;
pub use code::CodeTokens;
pub use radio::RadioTokens;
pub use overlay::OverlayTokens;
pub use parallax::ParallaxTokens;
pub use infinite_scroller::InfiniteScrollerTokens;
pub use hover::HoverTokens;
pub use fade_transition::FadeTransitionTokens;
pub use scale_transition::ScaleTransitionTokens;
pub use expand_transition::ExpandTransitionTokens;
pub use slide_transition::SlideTransitionTokens;
pub use icon_button::IconButtonTokens;
pub use expansion_panels::ExpansionPanelsTokens;
pub use speed_dial::SpeedDialTokens;
pub use chip_group::ChipGroupTokens;
pub use list_group::ListGroupTokens;
pub use counter::CounterTokens;
pub use responsive::ResponsiveTokens;
pub use checkbox_btn::CheckboxBtnTokens;
pub use list_subheader::ListSubheaderTokens;
pub use number_input::NumberInputTokens;
pub use pull_to_refresh::PullToRefreshTokens;
pub use confirm_edit::ConfirmEditTokens;

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
