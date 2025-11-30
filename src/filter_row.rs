use std::cell::RefCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, glib};
use glib::clone;
use glib::subclass::Signal;

use crate::utils::case;

//------------------------------------------------------------------------------
// MODULE: FilterRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FilterRow)]
    #[template(resource = "/com/github/Syncer/ui/filter_row.ui")]
    pub struct FilterRow {
        #[template_child]
        pub(super) delete_button: TemplateChild<gtk::Button>,

        #[property(get, set)]
        filter: RefCell<String>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for FilterRow {
        const NAME: &'static str = "FilterRow";
        type Type = super::FilterRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterRow {
        //---------------------------------------
        // Signals
        //---------------------------------------
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("deleted")
                        .build(),
                    Signal::builder("drag")
                        .param_types([i32::static_type(), i32::static_type()])
                        .build(),
                ]
            })
        }

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

    impl WidgetImpl for FilterRow {}
    impl ListBoxRowImpl for FilterRow {}
    impl PreferencesRowImpl for FilterRow {}
    impl ActionRowImpl for FilterRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: FilterRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct FilterRow(ObjectSubclass<imp::FilterRow>)
        @extends adw::ActionRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl FilterRow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(filter: &str) -> Self {
        glib::Object::builder()
            .property("filter", filter)
            .build()
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Delete button clicked signal
        imp.delete_button.connect_clicked(clone!(
            #[weak(rename_to = row)] self,
            move |_| {
                row.emit_by_name::<()>("deleted", &[]);
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        // Bind filter property to widget
        self.bind_property("filter", self, "title")
            .transform_to(|_, filter: String| {
                filter.split_once("=")
                    .map(|(type_, _)| case::capitalize_first(type_.trim_start_matches("--")))
            })
            .sync_create()
            .build();

        self.bind_property("filter", self, "subtitle")
            .transform_to(|_, filter: String| {
                filter.split_once("=")
                    .map(|(_, filter)| filter.trim_matches('"').to_owned())
            })
            .sync_create()
            .build();

        // Create drag source
        let drag_source = gtk::DragSource::builder()
            .actions(gdk::DragAction::MOVE)
            .build();

        // Connect drag source prepare signal
        drag_source.connect_prepare(|source, _, _| {
            source.widget()
                .map(|widget| gdk::ContentProvider::for_value(&widget.to_value()))
        });

        // Connect drag source drag begin signal
        drag_source.connect_drag_begin(|source, drag| {
            // Create dummy drag widget
            let listbox = source.widget()
                .and_downcast::<Self>()
                .map(|row| {
                    let drag_row = Self::new(&row.filter());

                    let listbox = gtk::ListBox::new();
                    listbox.set_size_request(row.width(), row.height());

                    listbox.append(&drag_row);
                    listbox.drag_highlight_row(&drag_row);

                    listbox
                });

            let drag_icon = gtk::DragIcon::for_drag(drag);
            drag_icon.set_child(listbox.as_ref());
        });

        // Add drag source to row
        self.add_controller(drag_source);

        // Create drop target
        let drop_target = gtk::DropTarget::new(gtk::Widget::static_type(), gdk::DragAction::MOVE);

        // Connect drop target drop begin signal
        drop_target.connect_drop(clone!(
            #[weak(rename_to = row)] self,
            #[upgrade_or] false,
            move|_, value, _, _| {
                let Ok(drag_row) = value.get::<Self>() else {
                    return false;
                };

                if drag_row == row {
                    return false;
                }

                row.emit_by_name::<()>("drag", &[&drag_row.index(), &row.index()]);

                true
            }
        ));

        // Add drop target to row
        self.add_controller(drop_target);
    }
}
