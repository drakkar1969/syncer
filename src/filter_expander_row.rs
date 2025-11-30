use std::sync::LazyLock;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::glib;
use glib::{clone, closure_local};

use regex::Regex;

use crate::{
    filter_row::FilterRow,
    utils::case
};

//------------------------------------------------------------------------------
// MODULE: FilterExpanderRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/filter_expander_row.ui")]
    pub struct FilterExpanderRow {
        #[template_child]
        pub(super) add_button: TemplateChild<gtk::Button>,
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

        // Add button clicked signal
        imp.add_button.connect_clicked(clone!(
            #[weak(rename_to = expander)] self,
            move |_| {
                expander.filter_dialog(clone!(
                    #[weak] expander,
                    move |type_, filter| {
                        expander.add_filter_row(type_, filter);

                        expander.set_enable_expansion(true);
                        expander.set_expanded(true);

                        expander.update_subtitle();
                    }
                ));
            }
        ));
    }

    //---------------------------------------
    // Add filter function
    //---------------------------------------
    fn add_filter_row(&self, type_: &str, filter: &str) {
        let row = FilterRow::new(type_, filter);

        row.connect_closure("changed", false, closure_local!(
            #[weak(rename_to = expander)] self,
            move |_: FilterRow| {
                expander.update_subtitle();

                if expander.filter_rows().is_empty() {
                    expander.set_expanded(false);
                    expander.set_enable_expansion(false);
                }
            }
        ));

        self.add_row(&row);
    }

    //---------------------------------------
    // Filter dialog function
    //---------------------------------------
    fn filter_dialog<F>(&self, f: F)
    where F: Fn(&str, &str) + 'static {
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

            f(&type_, &filter_entry.text());
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

    //---------------------------------------
    // Update subtitle function
    //---------------------------------------
    fn update_subtitle(&self) {
        let filters: Vec<String> = self.filter_rows().iter()
            .map(|row| {
                format!("--{}=\"{}\"",
                    row.title().to_ascii_lowercase(),
                    row.subtitle().unwrap_or_default()
                )
            })
            .collect();

        self.set_subtitle(&filters.join(" "));
    }

    //---------------------------------------
    // Create filter rows function
    //---------------------------------------
    pub fn create_filter_rows(&self, filters: &str) {
        for row in self.filter_rows() {
            self.remove(&row);
        }

        static EXPR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"--(?P<type>\w+)="(?P<filter>[^"]+)""#)
                .expect("Failed to compile Regex")
        });

        for filter in filters.split(' ') {
            if let Some((type_, filter)) = EXPR.captures(filter)
                .and_then(|caps| caps.name("type").zip(caps.name("filter"))) {
                    self.add_filter_row(
                        &case::capitalize_first(type_.as_str()),
                        filter.as_str()
                    );
                }
        }

        self.set_enable_expansion(!filters.is_empty());
        self.set_expanded(false);
    }
}
