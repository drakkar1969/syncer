use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::glib;
use glib::{clone, closure_local};

use crate::filter_row::FilterRow;

//------------------------------------------------------------------------------
// MODULE: FilterExpanderRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FilterExpanderRow)]
    #[template(resource = "/com/github/Syncer/ui/filter_expander_row.ui")]
    pub struct FilterExpanderRow {
        #[template_child]
        pub(super) add_button: TemplateChild<gtk::Button>,

        #[property(get, set)]
        filters: RefCell<Vec<String>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for FilterExpanderRow {
        const NAME: &'static str = "FilterExpanderRow";
        type Type = super::FilterExpanderRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterExpanderRow {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
        }
    }

    impl WidgetImpl for FilterExpanderRow {}
    impl ListBoxRowImpl for FilterExpanderRow {}
    impl PreferencesRowImpl for FilterExpanderRow {}
    impl ExpanderRowImpl for FilterExpanderRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: FilterExpanderRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct FilterExpanderRow(ObjectSubclass<imp::FilterExpanderRow>)
        @extends adw::ExpanderRow, adw::PreferencesRow, gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl FilterExpanderRow {
    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Filters property notify signal
        self.connect_filters_notify(|expander| {
            // Remove all filter rows
            for row in expander.filter_rows() {
                expander.remove(&row);
            }

            // Create new filter rows
            let filters = expander.filters();

            for filter in filters.iter() {
                let row = FilterRow::new(filter);

                row.connect_closure("deleted", false, closure_local!(
                    #[weak] expander,
                    move |row: FilterRow| {
                        if let Some(pos) = expander.filters().iter()
                            .position(|filter| filter == &row.filter()) {
                                let mut filters = expander.filters();

                                filters.remove(pos);

                                expander.set_filters(filters);
                            }
                    }
                ));

                row.connect_closure("drag", false, closure_local!(
                    #[weak] expander,
                    move |_: FilterRow, old_pos: i32, new_pos: i32| {
                        let mut filters = expander.filters();

                        let filter = filters.remove(old_pos as usize);
                        filters.insert(new_pos as usize, filter);

                        expander.set_filters(filters);
                    }
                ));

                expander.add_row(&row);
            }

            expander.set_expanded(!filters.is_empty());
            expander.set_enable_expansion(!filters.is_empty());

            // Update subtitle
            expander.set_subtitle(&filters.join(" "));
        });

        // Add button clicked signal
        imp.add_button.connect_clicked(clone!(
            #[weak(rename_to = expander)] self,
            move |_| {
                expander.filter_dialog(clone!(
                    #[weak] expander,
                    move |filter| {
                        let mut filters = expander.filters();
                        filters.push(filter.to_owned());
                        expander.set_filters(filters);
                    }
                ));
            }
        ));
    }

    //---------------------------------------
    // Filter dialog function
    //---------------------------------------
    fn filter_dialog<F>(&self, f: F)
    where F: Fn(&str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/filter_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        let type_combo: adw::ComboRow = builder.object("type_combo")
            .expect("Could not get object from resource");

        let filter_entry: adw::EntryRow = builder.object("filter_entry")
            .expect("Could not get object from resource");

        filter_entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        dialog.connect_response(Some("add"), move |_, _| {
            let type_ = type_combo.selected_item()
                .and_downcast::<gtk::StringObject>()
                .expect("Could not downcast to 'GtkStringObject'")
                .string();

            f(&format!("--{}=\"{}\"", type_.to_ascii_lowercase(), filter_entry.text()));
        });

        dialog.present(Some(self));
    }

    //---------------------------------------
    // Rows function
    //---------------------------------------
    fn filter_rows(&self) -> Vec<FilterRow> {
        let listbox = self.first_child()
            .and_downcast::<gtk::Box>()
            .expect("Could not downcast to 'GtkBox'")
            .last_child()
            .and_downcast::<gtk::Revealer>()
            .expect("Could not downcast to 'GtkListBox'")
            .child()
            .and_downcast::<gtk::ListBox>()
            .expect("Could not downcast to 'GtkListBox'");

        let mut i = 0;
        let mut rows = vec![];

        while let Some(row) = listbox.row_at_index(i).and_downcast::<FilterRow>() {
            rows.push(row);
            
            i += 1;
        }

        rows
    }
}
