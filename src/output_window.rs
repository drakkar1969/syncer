use std::cell::Cell;
use std::time::Duration;

use adw::subclass::prelude::*;
use gtk::prelude::*;
use gtk::{gio, glib, gdk};
use glib::{clone, BoxedAnyObject};

use crate::{
    output_item::OutputItem,
    output_header::OutputHeader,
    rsync_process::{RsyncMsgType, RsyncMessages}
};

//------------------------------------------------------------------------------
// STRUCT: OutputObject
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct OutputObject {
    pub tag: RsyncMsgType,
    pub msg: String
}

impl OutputObject {
    pub fn new(tag: RsyncMsgType, msg: &str) -> Self {
        Self {
            tag,
            msg: msg.to_owned()
        }
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
    Info,
    Files,
    Dirs,
    Links,
    Specials
}

//------------------------------------------------------------------------------
// MODULE: OutputWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::OutputWindow)]
    #[template(resource = "/com/github/Syncer/ui/output_window.ui")]
    pub struct OutputWindow {
        #[template_child]
        pub(super) header_sub_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) scroll_window: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub(super) view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::NoSelection>,
        #[template_child]
        pub(super) error_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) stat_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) message_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) filter: TemplateChild<gtk::CustomFilter>,
        #[template_child]
        pub(super) filter_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) item_factory: TemplateChild<gtk::SignalListItemFactory>,
        #[template_child]
        pub(super) header_factory: TemplateChild<gtk::SignalListItemFactory>,

        #[property(get, set, builder(FilterType::default()))]
        filter_type: Cell<FilterType>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for OutputWindow {
        const NAME: &'static str = "OutputWindow";
        type Type = super::OutputWindow;
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
    impl ObjectImpl for OutputWindow {
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

    impl WidgetImpl for OutputWindow {}
    impl WindowImpl for OutputWindow {}
    impl AdwWindowImpl for OutputWindow {}

    impl OutputWindow {
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
// IMPLEMENTATION: OutputWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct OutputWindow(ObjectSubclass<imp::OutputWindow>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl OutputWindow {
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
                FilterType::Info => "stats-info-symbolic",
                FilterType::Files => "stats-file-symbolic",
                FilterType::Dirs => "stats-dir-symbolic",
                FilterType::Links => "stats-link-symbolic",
                FilterType::Specials => "stats-special-symbolic",
            };

            imp.filter_button.set_icon_name(icon);
        });

        // Item factory setup signal
        imp.item_factory.connect_setup(|_, obj| {
            let item = obj
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkLIstItem'");

            item.set_child(Some(&OutputItem::default()));
        });

        // Item factory bind signal
        imp.item_factory.connect_bind(|_, obj| {
            let item = obj
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkListItem'");

            let child = item.child()
                .and_downcast::<OutputItem>()
                .expect("Could not downcast to 'OutputItem'");

            let output_object = item.item()
                .and_downcast::<BoxedAnyObject>()
                .expect("Could not downcast to 'BoxedAnyObject'");

            child.bind(&output_object.borrow());
        });

        // Header factory setup signal
        imp.header_factory.connect_setup(|_, obj| {
            let header = obj
                .downcast_ref::<gtk::ListHeader>()
                .expect("Could not downcast to 'GtkLIstHeader'");

            header.set_child(Some(&OutputHeader::default()));
        });

        // Header factory bind signal
        imp.header_factory.connect_bind(|_, obj| {
            let header = obj
                .downcast_ref::<gtk::ListHeader>()
                .expect("Could not downcast to 'GtkListHeader'");

            let child = header.child()
                .and_downcast::<OutputHeader>()
                .expect("Could not downcast to 'OutputHeader'");

            let output_object = header.item()
                .and_downcast::<BoxedAnyObject>()
                .expect("Could not downcast to 'BoxedAnyObject'");

            child.bind(&output_object.borrow());
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
                let output_object = obj
                    .downcast_ref::<BoxedAnyObject>()
                    .expect("Could not downcast to 'BoxedAnyObject'")
                    .borrow::<OutputObject>();

                let tag = output_object.tag;
                let msg = &output_object.msg;

                let search = imp.search_entry.text();

                // Return if message text doesn’t contain the search string (ignore case)
                if !msg.to_ascii_lowercase().contains(&search.to_ascii_lowercase()) {
                    return false;
                }

                match window.filter_type() {
                    FilterType::All => true,
                    FilterType::Info => tag == RsyncMsgType::Info,
                    FilterType::Files => tag == RsyncMsgType::f,
                    FilterType::Dirs => tag == RsyncMsgType::d,
                    FilterType::Links => tag == RsyncMsgType::L,
                    FilterType::Specials => tag == RsyncMsgType::D || tag == RsyncMsgType::S,
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
        let errors: Vec<BoxedAnyObject> = messages.errors.iter()
            .map(|msg| BoxedAnyObject::new(OutputObject::new(RsyncMsgType::Error, msg)))
            .collect();

        imp.error_model.splice(0, 0, &errors);

        // Add stats to model
        let stats: Vec<BoxedAnyObject> = messages.stats.iter()
            .map(|msg| BoxedAnyObject::new(OutputObject::new(RsyncMsgType::Stat, msg)))
            .collect();

        imp.stat_model.splice(0, 0, &stats);

        // Spawn task to process messages
        let (sender, receiver) = async_channel::bounded(10);

        let messages = messages.messages.clone();

        gio::spawn_blocking(
            move || {
                for chunk in messages.chunks(500) {
                    sender
                        .send_blocking(chunk.to_vec())
                        .expect("Could not send through channel");
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
                        .map(|(flag, msg)| BoxedAnyObject::new(OutputObject::new(*flag, msg)))
                        .collect();

                    imp.message_model.splice(imp.message_model.n_items(), 0, &messages);
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

        imp.error_model.remove_all();
        imp.stat_model.remove_all();
        imp.message_model.remove_all();

        imp.search_entry.set_text("");

        self.set_filter_type(FilterType::default());
    }

    //---------------------------------------
    // Display function
    //---------------------------------------
    pub fn display(&self, window: &gtk::Window) {
        let imp = self.imp();

        self.set_transient_for(Some(window));

        self.present();

        // Scroll to start
        glib::idle_add_local_once(clone!(
            #[weak] imp,
            move || {
                let v_adjust = imp.scroll_window.vadjustment();
                v_adjust.set_value(v_adjust.lower());
            }
        ));
    }
}

impl Default for OutputWindow {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
