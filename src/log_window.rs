use std::iter;

use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use gtk::prelude::*;
use glib::clone;

//------------------------------------------------------------------------------
// CONST Variables
//------------------------------------------------------------------------------
const STATS_TAG: &str = "::STATS::";

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
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) skipped_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) deleted_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::NoSelection>,
        #[template_child]
        pub(super) model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) filter: TemplateChild<gtk::CustomFilter>,
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

            let image = gtk::Image::new();

            let label = gtk::Label::builder()
                .xalign(0.0)
                .build();

            let box_ = gtk::Box::builder()
                .spacing(8)
                .build();

            box_.append(&image);
            box_.append(&label);

            item.set_child(Some(&box_));
        });

        // Factory bind signal
        imp.factory.connect_bind(|_, obj| {
            let item = obj
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkListItem'");

            let box_ = item.child()
                .and_downcast::<gtk::Box>()
                .expect("Could not downcast to 'GtkBox'");

            let image = box_.first_child()
                .and_downcast::<gtk::Image>()
                .expect("Could not downcast to 'GtkImage'");

            let label = box_.last_child()
                .and_downcast::<gtk::Label>()
                .expect("Could not downcast to 'GtkLabel'");

            let text = item.item()
                .and_downcast::<gtk::StringObject>()
                .expect("Could not downcast to 'GtkStringObject'")
                .string();

            let is_stats = text.starts_with(STATS_TAG);

            if is_stats {
                label.set_label(&text.replace(STATS_TAG, ""));
            } else {
                label.set_label(&text);
            }

            image.set_visible(true);
            image.set_icon_name(None);

            if text.starts_with("cannot") {
                box_.set_css_classes(&["error"]);

                image.set_icon_name(Some("rsync-error-symbolic"));
            } else if text.starts_with("skipping") {
                box_.set_css_classes(&["warning"]);

                image.set_icon_name(Some("stats-skipped-symbolic"));
            } else if text.starts_with("deleting") {
                box_.set_css_classes(&["warning"]);

                image.set_icon_name(Some("stats-deleted-symbolic"));
            } else if text.contains("->") {
                box_.set_css_classes(&["accent"]);

                image.set_icon_name(Some("stats-link-symbolic"));
            } else if is_stats {
                box_.set_css_classes(&["success"]);

                image.set_visible(false);
            } else {
                box_.set_css_classes(&[]);

                if text.ends_with('/') {
                    image.set_icon_name(Some("stats-dir-symbolic"));
                } else if !text.is_empty() {
                    image.set_icon_name(Some("stats-file-symbolic"));
                }
            }
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
                imp.filter.changed(gtk::FilterChange::Different);
            }
        ));

        // Skipped button toggled signal
        imp.skipped_button.connect_toggled(clone!(
            #[weak] imp,
            move |button| {
                if button.is_active() {
                    imp.filter.changed(gtk::FilterChange::LessStrict);
                } else {
                    imp.filter.changed(gtk::FilterChange::MoreStrict);
                }
            }
        ));

        // Deleted button toggled signal
        imp.deleted_button.connect_toggled(clone!(
            #[weak] imp,
            move |button| {
                if button.is_active() {
                    imp.filter.changed(gtk::FilterChange::LessStrict);
                } else {
                    imp.filter.changed(gtk::FilterChange::MoreStrict);
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
                // Spawn task to populate view
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
                    .chain(iter::once(&String::new()).filter(|_| !messages.is_empty()))
                    .chain(stats_msgs.iter())
                    .map(|s| gtk::StringObject::new(s))
                    .collect();

                imp.model.splice(0, 0, &objects);

                // Display view
                imp.stack.set_visible_child_name("log");

                // Set initial focus on view
                imp.view.grab_focus();
            }
        ));
    }
}
