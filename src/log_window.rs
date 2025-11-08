use std::iter;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use gtk::prelude::*;
use glib::clone;

use crate::log_item::{STATS_TAG, LogItem}; 

//------------------------------------------------------------------------------
// MODULE: LogWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/log_window.ui")]
    pub struct LogWindow {
        #[template_child]
        pub(super) header_sub_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) skipped_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) deleted_button: TemplateChild<gtk::ToggleButton>,
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

            //---------------------------------------
            // Search key binding
            //---------------------------------------
            klass.add_binding(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, |window| {
                let imp = window.imp();

                if !imp.search_entry.has_focus() {
                    imp.search_entry.grab_focus();
                }

                glib::Propagation::Stop
            });

            //---------------------------------------
            // Close window key binding
            //---------------------------------------
            klass.add_binding_action(gdk::Key::Escape, gdk::ModifierType::NO_MODIFIER_MASK, "window.close");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

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
    // New function
    //---------------------------------------
    pub fn new(parent: &impl IsA<gtk::Window>) -> Self {
        glib::Object::builder()
            .property("transient-for", parent)
            .build()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

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

            let text = item.item()
                .and_downcast::<gtk::StringObject>()
                .expect("Could not downcast to 'GtkStringObject'")
                .string();

            child.bind(&text);
        });

        // Search entry search started signal
        imp.search_entry.connect_search_started(|entry| {
            if !entry.has_focus() {
                entry.grab_focus();
            }
        });

        // Search entry search changed signal
        imp.search_entry.connect_search_changed(clone!(
            #[weak] imp,
            move |_| {
                imp.spinner.set_visible(true);

                imp.filter.changed(gtk::FilterChange::Different);
            }
        ));

        // Skipped button toggled signal
        imp.skipped_button.connect_toggled(clone!(
            #[weak] imp,
            move |_| {
                imp.spinner.set_visible(true);

                imp.filter.changed(gtk::FilterChange::Different);
            }
        ));

        // Deleted button toggled signal
        imp.deleted_button.connect_toggled(clone!(
            #[weak] imp,
            move |_| {
                imp.spinner.set_visible(true);

                imp.filter.changed(gtk::FilterChange::Different);
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
            #[weak] imp,
            move |model| {
                if model.pending() == 0 {
                    imp.spinner.set_visible(false);
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
            #[weak] imp,
            #[upgrade_or] false,
            move |obj| {
                let text = obj
                    .downcast_ref::<gtk::StringObject>()
                    .expect("Could not downcast to 'GtkStringObject'")
                    .string()
                    .to_lowercase();

                if text.contains(&imp.search_entry.text().to_lowercase()) {
                    if text.starts_with("skipping") {
                        imp.skipped_button.is_active()
                    } else if text.starts_with("deleting") {
                        imp.deleted_button.is_active()
                    } else {
                        true
                    }
                } else {
                    false
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
    // Display function
    //---------------------------------------
    pub fn display(&self, messages: &[String], stats_msgs: &[String]) {
        let imp = self.imp();

        self.present();

        let messages = messages.to_vec();
        let stats_msgs = stats_msgs.to_vec();

        glib::spawn_future_local(clone!(
            #[weak] imp,
            async move {
                // Spawn task to process stats messages
                let stats_msgs: Vec<String> = gio::spawn_blocking(clone!(
                    move || {
                        stats_msgs.into_iter()
                            .map(|s| format!("{STATS_TAG}{s}"))
                            .collect()
                    }
                ))
                .await
                .expect("Failed to complete task");

                // Populate view
                let objects: Vec<gtk::StringObject> = messages.iter()
                    .chain(
                        iter::once(&String::new())
                            .filter(|_| !messages.is_empty() && !stats_msgs.is_empty())
                    )
                    .chain(stats_msgs.iter())
                    .map(|s| gtk::StringObject::new(s))
                    .collect();

                imp.model.splice(0, 0, &objects);

                // Set initial focus on view
                imp.view.grab_focus();
            }
        ));
    }
}
