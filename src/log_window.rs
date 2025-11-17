use std::cell::Cell;
use std::time::Duration;

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib, gdk};
use glib::{clone, BoxedAnyObject};

use crate::{
    log_item::LogItem,
    rsync_process::{RsyncMsgType, RsyncMessages}
};

//------------------------------------------------------------------------------
// STRUCT: LogObject
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct LogObject {
    tag: RsyncMsgType,
    msg: String
}

impl LogObject {
    pub fn new(tag: RsyncMsgType, msg: &str) -> Self {
        Self {
            tag,
            msg: msg.to_owned()
        }
    }

    pub fn tag(&self) -> RsyncMsgType {
        self.tag
    }

    pub fn msg(&self) -> &str {
        &self.msg
    }
}

//------------------------------------------------------------------------------
// ENUM: FilterType
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "FilterType")]
pub enum FilterType {
    #[default]
    All,
    Errors,
    Info,
    Deleted,
    Skipped,
}

//------------------------------------------------------------------------------
// MODULE: LogWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::LogWindow)]
    #[template(resource = "/com/github/Syncer/ui/log_window.ui")]
    pub struct LogWindow {
        #[template_child]
        pub(super) header_sub_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::NoSelection>,
        #[template_child]
        pub(super) model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) filter: TemplateChild<gtk::CustomFilter>,
        #[template_child]
        pub(super) filter_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) factory: TemplateChild<gtk::SignalListItemFactory>,

        #[property(get, set, builder(FilterType::default()))]
        filter_type: Cell<FilterType>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for LogWindow {
        const NAME: &'static str = "LogWindow";
        type Type = super::LogWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            Self::install_actions(klass);
            Self::bind_shortcuts(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for LogWindow {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for LogWindow {}
    impl WindowImpl for LogWindow {}
    impl AdwWindowImpl for LogWindow {}

    impl LogWindow {
        //---------------------------------------
        // Install actions
        //---------------------------------------
        fn install_actions(klass: &mut <Self as ObjectSubclass>::Class) {
            // Filter type property action
            klass.install_property_action("filter.type", "filter-type");
        }

        //---------------------------------------
        // Bind shortcuts
        //---------------------------------------
        fn bind_shortcuts(klass: &mut <Self as ObjectSubclass>::Class) {
            // Search key binding
            klass.add_binding(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, |window| {
                let imp = window.imp();

                if !imp.search_entry.has_focus() {
                    imp.search_entry.grab_focus();
                }

                glib::Propagation::Stop
            });

            // Close window key binding
            klass.add_binding_action(gdk::Key::Escape, gdk::ModifierType::NO_MODIFIER_MASK, "window.close");
        }
    }
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: LogWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct LogWindow(ObjectSubclass<imp::LogWindow>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl LogWindow {
    //---------------------------------------
    // Show spinner function
    //---------------------------------------
    fn show_spinner(&self, show: bool) {
        let imp = self.imp();

        if show {
            glib::timeout_add_local_once(Duration::from_millis(100), clone!(
                #[weak] imp,
                move || {
                    if imp.filter_model.pending() != 0 {
                        imp.spinner.set_visible(true);
                    }
                }
            ));
        } else {
            imp.spinner.set_visible(false);
        }
    }
    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Filter type property notify signal 
        self.connect_filter_type_notify(|window| {
            let imp = window.imp();

            window.show_spinner(true);

            imp.filter.changed(gtk::FilterChange::Different);

            let icon = match window.filter_type() {
                FilterType::All => "stats-symbolic",
                FilterType::Errors => "rsync-error-symbolic",
                FilterType::Info => "stats-info-symbolic",
                FilterType::Deleted => "stats-deleted-symbolic",
                FilterType::Skipped => "stats-skipped-symbolic",
            };

            imp.filter_button.set_icon_name(icon);
        });

        // Factory setup signal
        imp.factory.connect_setup(|_, obj| {
            let item = obj
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkLIstItem'");

            item.set_child(Some(&LogItem::default()));
        });

        // Factory bind signal
        imp.factory.connect_bind(|_, obj| {
            let item = obj
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkListItem'");

            let child = item.child()
                .and_downcast::<LogItem>()
                .expect("Could not downcast to 'LogItem'");

            let log_object = item.item()
                .and_downcast::<BoxedAnyObject>()
                .expect("Could not downcast to 'BoxedAnyObject'");

            child.bind(&log_object.borrow());
        });

        // Search entry search started signal
        imp.search_entry.connect_search_started(|entry| {
            if !entry.has_focus() {
                entry.grab_focus();
            }
        });

        // Search entry search changed signal
        imp.search_entry.connect_search_changed(clone!(
            #[weak(rename_to = window)] self,
            move |_| {
                window.show_spinner(true);

                window.imp().filter.changed(gtk::FilterChange::Different);
            }
        ));

        // Selection items changed signal
        imp.selection.connect_items_changed(clone!(
            #[weak] imp,
            move |selection, _, _, _| {
                let n_items = selection.n_items();

                imp.header_sub_label.set_label(&format!("{n_items} item{}", if n_items == 1 { "" } else { "s" }));
            }
        ));

        // Filter model pending property notify signal
        imp.filter_model.connect_pending_notify(clone!(
            #[weak(rename_to = window)] self,
            move |model| {
                if model.pending() == 0 {
                    window.show_spinner(false);
                }
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Set search entry key capture widget
        imp.search_entry.set_key_capture_widget(Some(&imp.view.get()));

        // Set filter function
        imp.filter.set_filter_func(clone!(
            #[weak(rename_to = window)] self,
            #[weak] imp,
            #[upgrade_or] false,
            move |obj| {
                let log_object = obj
                    .downcast_ref::<BoxedAnyObject>()
                    .expect("Could not downcast to 'BoxedAnyObject'")
                    .borrow::<LogObject>();

                let msg = log_object.msg();
                let tag = log_object.tag();

                let search = imp.search_entry.text();

                // Return if message text doesn’t contain the search string (ignore case)
                if !msg.to_ascii_lowercase().contains(&search.to_ascii_lowercase()) {
                    return false;
                }

                // Helper closure for case-insensitive prefix check
                let starts_with_ic = |prefix: &str| -> bool {
                    msg.get(..prefix.len())
                        .is_some_and(|s| s.eq_ignore_ascii_case(prefix))
                };

                match window.filter_type() {
                    FilterType::All => true,
                    FilterType::Errors => tag == RsyncMsgType::Error,
                    FilterType::Info => {
                        tag == RsyncMsgType::Info
                            && !starts_with_ic("deleting")
                            && !starts_with_ic("skipping")
                    }
                    FilterType::Deleted => {
                        tag == RsyncMsgType::Info && starts_with_ic("deleting")
                    }
                    FilterType::Skipped => {
                        tag == RsyncMsgType::Info && starts_with_ic("skipping")
                    }
                }
            }
        ));

        // Add keyboard shortcut to cancel search
        let shortcut = gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string("Escape"),
            Some(gtk::CallbackAction::new(clone!(
                #[weak] imp,
                #[upgrade_or] glib::Propagation::Proceed,
                move |_, _| {
                    imp.search_entry.set_text("");
                    imp.view.grab_focus();

                    glib::Propagation::Stop
                }
            )))
        );

        let controller = gtk::ShortcutController::new();
        controller.add_shortcut(shortcut);

        imp.search_entry.add_controller(controller);
    }

    //---------------------------------------
    // Load messages function
    //---------------------------------------
    pub fn load_messages(&self, messages: &RsyncMessages) {
        let imp = self.imp();

        // Add errors to model
        let errors: Vec<BoxedAnyObject> = messages.errors().iter()
            .map(|msg| BoxedAnyObject::new(LogObject::new(RsyncMsgType::Error, msg)))
            .collect();

        imp.model.splice(0, 0, &errors);

        if !messages.errors().is_empty() && !messages.stats().is_empty() {
            imp.model.append(&BoxedAnyObject::new(LogObject::default()));
        }

        // Add stats to model
        let stats: Vec<BoxedAnyObject> = messages.stats().iter()
            .map(|msg| BoxedAnyObject::new(LogObject::new(RsyncMsgType::Stat, msg)))
            .collect();

        imp.model.splice(imp.model.n_items(), 0, &stats);

        if (!messages.errors().is_empty() || !messages.stats().is_empty())
            && !messages.messages().is_empty() {
            imp.model.append(&BoxedAnyObject::new(LogObject::default()));
        }

        // Spawn task to process messages
        let (sender, receiver) = async_channel::bounded(10);

        let messages = messages.messages().to_vec();

        gio::spawn_blocking(
            move || {
                for chunk in messages.chunks(500) {
                    sender
                        .send_blocking(chunk.to_vec())
                        .expect("The channel needs to be open.");
                }
            }
        );

        // Attach receiver for task
        glib::spawn_future_local(clone!(
            #[weak] imp,
            async move {
                while let Ok(chunk) = receiver.recv().await {
                    // Add messages to model
                    let messages: Vec<BoxedAnyObject> = chunk.iter()
                        .map(|(flag, msg)| BoxedAnyObject::new(LogObject::new(*flag, msg)))
                        .collect();

                    imp.model.splice(imp.model.n_items(), 0, &messages);
                }

                // Set initial focus on view
                imp.view.grab_focus();
            }
        ));
    }

    //---------------------------------------
    // Clear messages function
    //---------------------------------------
    pub fn clear_messages(&self) {
        let imp = self.imp();

        imp.model.remove_all();

        imp.search_entry.set_text("");

        self.set_filter_type(FilterType::default());
    }

    //---------------------------------------
    // Display function
    //---------------------------------------
    pub fn display(&self, window: &gtk::Window) {
        self.set_transient_for(Some(window));

        self.present();
    }
}

impl Default for LogWindow {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
