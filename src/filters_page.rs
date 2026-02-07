use std::cell::{Cell, RefCell};

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::glib;
use glib::{clone, closure_local, translate::IntoGlib};

use strum::{EnumIter, EnumProperty, FromRepr, IntoEnumIterator};

use crate::{
    profile_object::ProfileObject,
    filter_row::FilterRow
};

//------------------------------------------------------------------------------
// ENUM: RsyncFilterRule
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum, EnumIter, EnumProperty, FromRepr)]
#[repr(u32)]
#[enum_type(name = "RsyncFilterRule")]
pub enum RsyncFilterRule {
    #[strum(props(Rule="-"))]
    Exclude,
    #[strum(props(Rule="+"))]
    Include,
    #[default]
    #[strum(props(Rule="H"))]
    Hide,
    #[strum(props(Rule="S"))]
    Show,
    #[strum(props(Rule="P"))]
    Protect,
    #[strum(props(Rule="R"))]
    Risk
}

impl RsyncFilterRule {
    pub fn value(self) -> u32 {
        self.into_glib() as u32
    }

    pub fn rule<'a>(self) -> Option<&'a str> {
        self.get_str("Rule")
    }
}

//------------------------------------------------------------------------------
// MODULE: FiltersPage
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FiltersPage)]
    #[template(resource = "/com/github/Syncer/ui/filters_page.ui")]
    pub struct FiltersPage {
        #[template_child]
        pub(super) filters_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) add_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub(super) delete_button: TemplateChild<adw::ButtonRow>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,
        #[property(get, set)]
        filters: RefCell<Vec<String>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>,

        pub(super) internal_change: Cell<bool>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for FiltersPage {
        const NAME: &'static str = "FiltersPage";
        type Type = super::FiltersPage;
        type ParentType = adw::NavigationPage;

        fn class_init(klass: &mut Self::Class) {
            RsyncFilterRule::ensure_type();

            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FiltersPage {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_signals();
        }
    }

    impl WidgetImpl for FiltersPage {}
    impl NavigationPageImpl for FiltersPage {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: FiltersPage
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct FiltersPage(ObjectSubclass<imp::FiltersPage>)
        @extends adw::NavigationPage, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl FiltersPage {
    //---------------------------------------
    // Listbox function
    //---------------------------------------
    fn listbox(&self) -> gtk::ListBox {
        self.imp().filters_group.first_child()
            .and_downcast::<gtk::Box>()
            .expect("Could not downcast to 'GtkBox'")
            .last_child()
            .and_downcast::<gtk::Box>()
            .expect("Could not downcast to 'GtkBox'")
            .first_child()
            .and_downcast::<gtk::ListBox>()
            .expect("Could not downcast to 'GtkListBox'")
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Profile property notify signal
        self.connect_profile_notify(|page| {
            let imp = page.imp();

            // Unbind stored bindings
            if let Some(bindings) = imp.bindings.take() {
                for binding in bindings {
                    binding.unbind();
                }
            }

            if let Some(profile) = page.profile() {
                // Bind profile property to widgets
                let bindings: Vec<glib::Binding> = vec![
                    // Bind profile property to filter property
                    profile.bind_property("filters", page, "filters")
                        .bidirectional()
                        .sync_create()
                        .build(),

                    // Bind profile property to page title
                    profile.bind_property("name", page, "title")
                        .sync_create()
                        .build()
                ];

                // Store bindings
                imp.bindings.replace(Some(bindings));
            }
        });

        // Filters property notify signal
        self.connect_filters_notify(|page| {
            let imp = page.imp();

            let filters = page.filters();

            if !imp.internal_change.get() {
                let listbox = page.listbox();

                // Remove all filter rows
                listbox.remove_all();

                // Create new filter rows
                for filter in &filters {
                    let (rule_str, pattern) = filter
                        .trim_start_matches("-f")
                        .trim_matches(['"', '\''])
                        .split_once(' ')
                        .expect("Failed to split filter");

                    let rule = RsyncFilterRule::iter()
                        .find(|rule| rule.rule() == Some(rule_str))
                        .expect("Failed to find filter rule");

                    let row = page.new_filter_row(rule, pattern);

                    listbox.append(&row);
                }
            }

            imp.delete_button.set_sensitive(!filters.is_empty());
        });

        // Add button activated signal
        imp.add_button.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |_| {
                page.filter_dialog("Add", None, clone!(
                    #[weak] page,
                    move |rule, pattern| {
                        let imp = page.imp();

                        let row = page.new_filter_row(rule, pattern);
                        page.listbox().append(&row);

                        imp.internal_change.set(true);

                        let mut filters = page.filters();
                        filters.push(row.filter());
                        page.set_filters(filters);

                        imp.internal_change.set(false);
                    }
                ));
            }
        ));

        // Delete button clicked signal
        imp.delete_button.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |_| {
                let imp = page.imp();

                imp.internal_change.set(true);

                page.set_filters(vec![]);

                page.listbox().remove_all();

                imp.internal_change.set(false);
            }
        ));
    }

    //---------------------------------------
    // New filter row function
    //---------------------------------------
    fn new_filter_row(&self, rule: RsyncFilterRule, pattern: &str) -> FilterRow {
        let row = FilterRow::new(rule, pattern);

        row.connect_closure("modified", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |row: FilterRow| {
                page.filter_dialog("Modify", Some((row.rule(), &row.pattern())), clone!(
                    #[weak] page,
                    move |rule, pattern| {
                        let imp = page.imp();

                        imp.internal_change.set(true);

                        row.set_rule(rule);
                        row.set_pattern(pattern);

                        let pos = row.index() as usize;

                        let mut filters = page.filters();
                        filters.remove(pos);
                        filters.insert(pos, row.filter());
                        page.set_filters(filters);

                        imp.internal_change.set(false);
                    }
                ));
            }
        ));

        row.connect_closure("deleted", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |row: FilterRow| {
                let imp = page.imp();

                imp.internal_change.set(true);

                let pos = row.index();

                let mut filters = page.filters();
                filters.remove(pos as usize);
                page.set_filters(filters);

                page.listbox().remove(&row);

                imp.internal_change.set(false);
            }
        ));

        row.connect_closure("drop", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |row: FilterRow, drag_row: FilterRow| {
                let imp = page.imp();

                imp.internal_change.set(true);

                let old_pos = drag_row.index();
                let new_pos = row.index();

                let mut filters = page.filters();
                let filter = filters.remove(old_pos as usize);
                filters.insert(new_pos as usize, filter);
                page.set_filters(filters);

                let listbox = page.listbox();
                listbox.remove(&drag_row);
                listbox.insert(&drag_row, new_pos);

                imp.internal_change.set(false);
            }
        ));

        row
    }

    //---------------------------------------
    // Filter dialog function
    //---------------------------------------
    fn filter_dialog<F>(&self, action: &str, filter: Option<(RsyncFilterRule, &str)>, f: F)
    where F: Fn(RsyncFilterRule, &str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/Syncer/ui/builder/filter_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(&format!("{action} Filter")));
        dialog.set_response_label("add", action);

        let rule_combo: adw::ComboRow = builder.object("rule_combo")
            .expect("Could not get object from resource");

        let pattern_entry: adw::EntryRow = builder.object("pattern_entry")
            .expect("Could not get object from resource");

        pattern_entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        if let Some((rule, pattern)) = filter {
            rule_combo.set_selected(rule.value());
            pattern_entry.set_text(pattern);
        } else {
            rule_combo.set_selected(RsyncFilterRule::default().value());
        }

        dialog.connect_response(Some("add"), move |_, _| {
            let rule = rule_combo.selected_item()
                .and_downcast::<adw::EnumListItem>()
                .and_then(|item| RsyncFilterRule::from_repr(item.value() as u32))
                .unwrap_or_default();

            f(rule, &pattern_entry.text());
        });

        dialog.present(Some(self));
    }
}
