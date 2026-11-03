//! Linux: `StatusNotifierItem` over D-Bus, with a `com.canonical.dbusmenu` menu.
//!
//! There is no cross-desktop tray API on Linux, only a convention: an
//! application exports an object implementing `org.kde.StatusNotifierItem`,
//! a panel (the "watcher", `org.kde.StatusNotifierWatcher`) is told the bus
//! name, and the panel reads the item's properties and menu from there. That is
//! what this arm does — one D-Bus connection, two objects, and a call to the
//! watcher to say they exist.
//!
//! No panel means no tray: with no watcher on the bus the registration is
//! refused rather than silently succeeding into an icon nobody draws.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use zbus::blocking::{Connection, Proxy};
use zbus::interface;
use zbus::zvariant::{OwnedObjectPath, Value};

use crate::{MenuItem, Queue, TrayEvent, drain, push};

const ITEM_PATH: &str = "/StatusNotifierItem";
const MENU_PATH: &str = "/MenuBar";
const WATCHER_BUS: &str = "org.kde.StatusNotifierWatcher";
const WATCHER_PATH: &str = "/StatusNotifierWatcher";
const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";

/// One node of a `com.canonical.dbusmenu` layout tree: `(ia{sv}av)` — the
/// node's id, its properties, and its children as variants of this same shape.
type Node = (i32, HashMap<String, Value<'static>>, Vec<Value<'static>>);

struct Item {
    queue: Queue,
    label: String,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl Item {
    #[zbus(property)]
    fn id(&self) -> String {
        "floom-tray".to_string()
    }

    #[zbus(property)]
    fn category(&self) -> String {
        "ApplicationStatus".to_string()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "Active".to_string()
    }

    #[zbus(property)]
    fn title(&self) -> String {
        self.label.clone()
    }

    /// SNI wants a themed icon *name*, not an image: the panel resolves it
    /// against the icon theme. `application-x-executable` exists in every
    /// theme that ships the standard icon set, which is the only name that can
    /// be promised from here.
    #[zbus(property)]
    fn icon_name(&self) -> String {
        "application-x-executable".to_string()
    }

    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(MENU_PATH).expect("MENU_PATH is a valid object path")
    }

    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        true
    }

    fn activate(&self, _x: i32, _y: i32) {
        push(&self.queue, TrayEvent::Activate);
    }

    fn secondary_activate(&self, _x: i32, _y: i32) {}

    fn context_menu(&self, _x: i32, _y: i32) {}

    fn scroll(&self, _delta: i32, _orientation: String) {}
}

/// A menu row, flattened: `None` is a separator.
struct Row {
    id: i32,
    label: Option<(String, bool)>,
}

struct Menu {
    queue: Queue,
    rows: Vec<Row>,
}

impl Menu {
    fn layout(&self) -> Node {
        let children = self
            .rows
            .iter()
            .map(|row| {
                let mut props: HashMap<String, Value<'static>> = HashMap::new();
                match &row.label {
                    Some((label, enabled)) => {
                        props.insert("label".to_string(), Value::from(label.clone()));
                        props.insert("enabled".to_string(), Value::from(*enabled));
                    }
                    None => {
                        props.insert("type".to_string(), Value::from("separator"));
                    }
                }
                Value::new((row.id, props, Vec::new()) as Node)
            })
            .collect();
        (0, HashMap::new(), children)
    }
}

#[interface(name = "com.canonical.dbusmenu")]
impl Menu {
    #[zbus(property)]
    fn version(&self) -> u32 {
        3
    }

    #[zbus(property)]
    fn text_direction(&self) -> String {
        "ltr".to_string()
    }

    #[zbus(property)]
    fn status(&self) -> String {
        "normal".to_string()
    }

    fn get_layout(
        &self,
        _parent_id: i32,
        _recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, Node) {
        // Revision 1: the layout never changes, because the menu cannot be
        // edited after `show`. A panel that caches it is right to.
        (1, self.layout())
    }

    fn get_group_properties(
        &self,
        _ids: Vec<i32>,
        _property_names: Vec<String>,
    ) -> Vec<(i32, HashMap<String, Value<'static>>)> {
        Vec::new()
    }

    fn event(&self, id: i32, event_id: String, _data: Value<'_>, _timestamp: u32) {
        if event_id == "clicked" && self.rows.iter().any(|row| row.id == id) {
            push(&self.queue, TrayEvent::Menu(id as u32));
        }
    }

    fn event_group(&self, _events: Vec<(i32, String, Value<'_>, u32)>) -> Vec<i32> {
        Vec::new()
    }

    fn about_to_show(&self, _id: i32) -> bool {
        false
    }

    fn about_to_show_group(&self, _ids: Vec<i32>) -> (Vec<i32>, Vec<i32>) {
        (Vec::new(), Vec::new())
    }
}

pub struct Tray {
    connection: Connection,
    queue: Queue,
}

impl Tray {
    pub fn show(label: &str, items: &[MenuItem]) -> Result<Self, String> {
        let queue: Queue = Arc::new(Mutex::new(Vec::new()));

        let connection = Connection::session().map_err(|error| {
            format!(
                "tray: no session bus ({error}) — a tray on Linux is a D-Bus \
                 StatusNotifierItem, and without a bus there is nobody to serve it"
            )
        })?;

        connection
            .object_server()
            .at(
                ITEM_PATH,
                Item {
                    queue: queue.clone(),
                    label: label.to_string(),
                },
            )
            .map_err(|error| format!("tray: could not export {ITEM_PATH} ({error})"))?;

        let mut rows = Vec::with_capacity(items.len());
        for item in items {
            match item {
                MenuItem::Separator => rows.push(Row { id: 0, label: None }),
                MenuItem::Item { id, label, enabled } => rows.push(Row {
                    id: *id as i32,
                    label: Some((label.clone(), *enabled)),
                }),
            }
        }

        connection
            .object_server()
            .at(
                MENU_PATH,
                Menu {
                    queue: queue.clone(),
                    rows,
                },
            )
            .map_err(|error| format!("tray: could not export {MENU_PATH} ({error})"))?;

        let unique = connection
            .unique_name()
            .ok_or("tray: the session bus did not give this connection a name")?
            .to_string();

        let watcher = Proxy::new(&connection, WATCHER_BUS, WATCHER_PATH, WATCHER_INTERFACE)
            .map_err(|error| format!("tray: {WATCHER_BUS} is not on this bus ({error})"))?;

        watcher
            .call::<_, _, ()>("RegisterStatusNotifierItem", &unique.as_str())
            .map_err(|error| {
                format!(
                    "tray: {WATCHER_BUS} refused this item ({error}) — the panel \
                     owns the tray, and without one running nothing draws the icon"
                )
            })?;

        Ok(Self { connection, queue })
    }

    pub fn events(&mut self) -> Vec<TrayEvent> {
        drain(&self.queue)
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // Removing the objects is what tells the panel the icon is gone; the
        // connection closing would do it too, but doing both means the icon
        // disappears before the socket does.
        let server = self.connection.object_server();
        let _ = server.remove::<Item, _>(ITEM_PATH);
        let _ = server.remove::<Menu, _>(MENU_PATH);
    }
}
