use std::cell::{Cell, RefCell};
use std::time::Duration;

use adw::{prelude::*, subclass::prelude::*};
use gtk::{glib, gio, gdk};
use glib::{clone, BoxedAnyObject};

use crate::{
    output_item::OutputItem,
    output_header::OutputHeader,
    rsync::{RsyncMsg, RsyncOutput}
};

//------------------------------------------------------------------------------
// STRUCT: OutputObject
//------------------------------------------------------------------------------
#[derive(Default, Debug, Clone)]
pub struct OutputObject {
    tag: RsyncMsg,
    msg: String,
    msg_lower: String
}

impl OutputObject {
    pub fn new(tag: RsyncMsg, msg: &str) -> Self {
        Self {
            tag,
            msg: msg.to_owned(),
            msg_lower: msg.to_lowercase()
        }
    }

    pub fn tag(&self) -> RsyncMsg {
        self.tag
    }

    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn msg_lower(&self) -> &str {
        &self.msg_lower
    }

    pub fn icon(&self) -> Option<&str> {
        match self.tag {
            RsyncMsg::Error => Some("rsync-error-symbolic"),
            RsyncMsg::Stat => Some("stats-symbolic"),
            RsyncMsg::Info => {
                if self.msg_lower.starts_with("deleting") {
                    Some("user-trash-symbolic")
                } else if self.msg_lower.starts_with("skipping") {
                    Some("edit-undo-symbolic")
                } else {
                    Some("info-outline-symbolic")
                }
            }
            RsyncMsg::File => Some("stats-file-symbolic"),
            RsyncMsg::Directory => Some("stats-dir-symbolic"),
            RsyncMsg::Link => Some("stats-link-symbolic"),
            RsyncMsg::Device | RsyncMsg::Special => Some("stats-special-symbolic"),
            RsyncMsg::None => None
        }
    }
}

//------------------------------------------------------------------------------
// ENUM: OutputFilter
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "OutputFilter")]
pub enum OutputFilter {
    #[default]
    All,
    Info,
    Files,
    Dirs,
    Links,
    Specials
}

//------------------------------------------------------------------------------
// MODULE: OutputPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::OutputPage)]
    #[template(resource = "/com/github/Syncer/ui/output_page.ui")]
    pub struct OutputPage {
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_bar: TemplateChild<gtk::SearchBar>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,

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
        pub(super) filter_model: TemplateChild<gtk::FilterListModel>,
        #[template_child]
        pub(super) search_filter: TemplateChild<gtk::CustomFilter>,
        #[template_child]
        pub(super) type_filter: TemplateChild<gtk::CustomFilter>,
        #[template_child]
        pub(super) item_factory: TemplateChild<gtk::SignalListItemFactory>,
        #[template_child]
        pub(super) header_factory: TemplateChild<gtk::SignalListItemFactory>,

        #[template_child]
        pub(super) footer_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,

        #[property(get, set, builder(OutputFilter::default()))]
        filter_type: Cell<OutputFilter>,

        pub(super) search_term: RefCell<String>,

        pub(super) shown: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for OutputPage {
        const NAME: &'static str = "OutputPage";
        type Type = super::OutputPage;
        type ParentType = adw::NavigationPage;

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
    impl ObjectImpl for OutputPage {
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

    impl WidgetImpl for OutputPage {}
    impl NavigationPageImpl for OutputPage {
        //---------------------------------------
        // Shown function
        //---------------------------------------
        fn shown(&self) {
            if !self.shown.get() {
                // Set initial focus on view
                self.view.grab_focus();

                self.shown.set(true);
            }
        }
    }

    impl OutputPage {
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
            klass.add_binding(gdk::Key::F, gdk::ModifierType::CONTROL_MASK, |page| {
                page.imp().search_bar.set_search_mode(true);

                glib::Propagation::Stop
            });
        }
    }
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: OutputPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct OutputPage(ObjectSubclass<imp::OutputPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl OutputPage {
    //---------------------------------------
    // Update bottom bar function
    //---------------------------------------
    fn update_bottom_bar(&self, show: bool) {
        let imp = self.imp();

        if show {
            glib::timeout_add_local_once(Duration::from_millis(100), clone!(
                #[weak] imp,
                move || {
                    if imp.filter_model.pending() != 0 {
                        imp.footer_label.set_label("Searching…");
                        imp.spinner.set_visible(true);
                    }
                }
            ));
        } else {
            let n_lines = imp.selection.n_items();

            imp.spinner.set_visible(false);

            imp.footer_label.set_label(
                &format!("{n_lines} line{}", if n_lines == 1 { "" } else { "s" })
            );
        }
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Filter type property notify signal 
        self.connect_filter_type_notify(|page| {
            let imp = page.imp();

            // Update footer
            page.update_bottom_bar(true);

            // Set filter button icon
            let icon = match page.filter_type() {
                OutputFilter::All => "stats-symbolic",
                OutputFilter::Info => "info-outline-symbolic",
                OutputFilter::Files => "stats-file-symbolic",
                OutputFilter::Dirs => "stats-dir-symbolic",
                OutputFilter::Links => "stats-link-symbolic",
                OutputFilter::Specials => "stats-special-symbolic",
            };

            imp.filter_button.set_icon_name(icon);

            // Update type filter
            imp.type_filter.changed(gtk::FilterChange::Different);
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

        // Search entry search changed signal
        imp.search_entry.connect_search_changed(clone!(
            #[weak(rename_to = page)] self,
            move |entry| {
                let imp = page.imp();

                // Store lowercase search term
                imp.search_term.replace(entry.text().to_ascii_lowercase());

                // Show spinner in footer
                page.update_bottom_bar(true);

                // Update search filter
                imp.search_filter.changed(gtk::FilterChange::Different);
            }
        ));

        // Filter model pending property notify signal
        imp.filter_model.connect_pending_notify(clone!(
            #[weak(rename_to = page)] self,
            move |model| {
                if model.pending() == 0 {
                    page.update_bottom_bar(false);
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
        imp.search_bar.set_key_capture_widget(Some(&imp.view.get()));

        // Bind search button state to search bar visibility
        imp.search_button.bind_property("active", &imp.search_bar.get(), "search-mode-enabled")
            .bidirectional()
            .sync_create()
            .build();

        // Set search filter function
        imp.search_filter.set_filter_func(clone!(
            #[weak(rename_to = page)] self,
            #[upgrade_or] false,
            move |obj| {
                let output_object = obj
                    .downcast_ref::<BoxedAnyObject>()
                    .expect("Could not downcast to 'BoxedAnyObject'")
                    .borrow::<OutputObject>();

                let search_term = page.imp().search_term.borrow();

                search_term.is_empty() || output_object.msg_lower.contains(&*search_term)
            }
        ));

        // Set type filter function
        imp.type_filter.set_filter_func(clone!(
            #[weak(rename_to = page)] self,
            #[upgrade_or] false,
            move |obj| {
                let output_object = obj
                    .downcast_ref::<BoxedAnyObject>()
                    .expect("Could not downcast to 'BoxedAnyObject'")
                    .borrow::<OutputObject>();

                let tag = output_object.tag;

                match page.filter_type() {
                    OutputFilter::All => true,
                    OutputFilter::Info => tag == RsyncMsg::Info,
                    OutputFilter::Files => tag == RsyncMsg::File,
                    OutputFilter::Dirs => tag == RsyncMsg::Directory,
                    OutputFilter::Links => tag == RsyncMsg::Link,
                    OutputFilter::Specials => {
                        tag == RsyncMsg::Device || tag == RsyncMsg::Special
                    }
                }
            }
        ));
    }

    //---------------------------------------
    // Load function
    //---------------------------------------
    pub fn load(&self, output: RsyncOutput) {
        let imp = self.imp();

        // Add errors to model
        let errors: Vec<BoxedAnyObject> = output.errors().iter()
            .map(|msg| BoxedAnyObject::new(OutputObject::new(RsyncMsg::Error, msg)))
            .collect();

        imp.error_model.splice(0, imp.error_model.n_items(), &errors);

        // Add stats to model
        let stats: Vec<BoxedAnyObject> = output.stats().iter()
            .map(|msg| BoxedAnyObject::new(OutputObject::new(RsyncMsg::Stat, msg)))
            .collect();

        imp.stat_model.splice(0, imp.stat_model.n_items(), &stats);

        // Spawn task to process messages
        let (sender, receiver) = async_channel::bounded(10);

        imp.message_model.remove_all();

        gio::spawn_blocking(
            move || {
                for chunk in output.messages().chunks(500) {
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
                    let messages: Vec<BoxedAnyObject> = chunk.into_iter()
                        .map(|(flag, msg)| BoxedAnyObject::new(
                            OutputObject::new(flag, &msg)
                        ))
                        .collect();

                    imp.message_model.splice(imp.message_model.n_items(), 0, &messages);
                }
            }
        ));
    }
}

impl Default for OutputPage {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
