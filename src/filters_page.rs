use std::cell::{RefCell, OnceCell};
use std::marker::PhantomData;

use adw::subclass::prelude::*;
use adw::prelude::*;
use gtk::{gio, glib};
use glib::{clone, closure_local, translate::IntoGlib};

use strum::{EnumIter, EnumProperty, FromRepr, IntoEnumIterator};

use crate::{
    profile_object::ProfileObject,
    filter_row::FilterRow,
    filter_dialog::FilterDialog
};

//------------------------------------------------------------------------------
// ENUM: FilterRule
//------------------------------------------------------------------------------
#[derive(Default, Debug, Eq, PartialEq, Clone, Copy, glib::Enum, EnumIter, EnumProperty, FromRepr)]
#[repr(u32)]
#[enum_type(name = "FilterRule")]
pub enum FilterRule {
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

impl FilterRule {
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
        #[property(get = Self::filters, set = Self::set_filters)]
        filters: PhantomData<Vec<String>>,

        pub(super) bindings: RefCell<Option<Vec<glib::Binding>>>,

        pub(super) filter_model: OnceCell<gio::ListStore>,
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
            FilterRule::ensure_type();

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
            obj.setup_widgets();
        }
    }

    impl WidgetImpl for FiltersPage {}
    impl NavigationPageImpl for FiltersPage {}

    impl FiltersPage {
        //---------------------------------------
        // Property getters/setters
        //---------------------------------------
        fn filters(&self) -> Vec<String> {
            self.filter_model.get().unwrap()
                .iter::<FilterRow>()
                .flatten()
                .map(|row| row.filter())
                .collect()
        }

        fn set_filters(&self, filters: Vec<String>) {
            let rows: Vec<FilterRow> = filters.iter()
                .map(|filter| {
                    let (rule_str, pattern) = filter
                        .trim_start_matches("-f")
                        .trim_matches(['"', '\''])
                        .split_once(' ')
                        .expect("Failed to split filter");

                    let rule = FilterRule::iter()
                        .find(|rule| rule.rule() == Some(rule_str))
                        .expect("Failed to find filter rule");

                    self.obj().new_filter_row(rule, pattern)
                })
                .collect();

            let filter_model = self.filter_model.get().unwrap();

            filter_model.splice(0, filter_model.n_items(), &rows);
        }
    }
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

        // Add button activated signal
        imp.add_button.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |_| {
                page.filter_dialog("Add", None, clone!(
                    #[weak] page,
                    move |rule, pattern| {
                        let row = page.new_filter_row(rule, pattern);

                        page.imp().filter_model.get().unwrap().append(&row);
                    }
                ));
            }
        ));

        // Delete button activated signal
        imp.delete_button.connect_activated(clone!(
            #[weak(rename_to = page)] self,
            move |_| {
                let dialog = adw::AlertDialog::builder()
                    .heading("Remove All Filter Rules?")
                    .body("Permanently remove all rules from profile.")
                    .default_response("remove")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("remove", "_Remove")]);
                dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("remove"), clone!(
                    #[weak] page,
                    move |_, _| {
                        page.imp().filter_model.get().unwrap().remove_all();
                    })
                );

                dialog.present(Some(&page));
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind filters preferences group to model
        let model = gio::ListStore::new::<FilterRow>();

        imp.filters_group.bind_model(Some(&model), |obj| {
            obj
                .downcast_ref::<FilterRow>()
                .expect("Could not downcast to 'FilterRow'")
                .clone()
                .into()
        });

        // Connect model items changed signal
        model.connect_items_changed(clone!(
            #[weak(rename_to = page)] self,
            move |_, _, _, _| {
                page.notify_filters();
            }
        ));

        // Store model
        imp.filter_model.set(model).unwrap();
    }

    //---------------------------------------
    // New filter row function
    //---------------------------------------
    fn new_filter_row(&self, rule: FilterRule, pattern: &str) -> FilterRow {
        let imp = self.imp();

        let row = FilterRow::new(rule, pattern);

        row.connect_closure("modified", false, closure_local!(
            #[weak(rename_to = page)] self,
            move |row: FilterRow| {
                page.filter_dialog("Modify", Some((row.rule(), &row.pattern())), clone!(
                    #[weak] page,
                    move |rule, pattern| {
                        let model = page.imp().filter_model.get().unwrap();

                        let pos = row.index() as u32;

                        let obj = model
                            .item(pos)
                            .and_downcast::<FilterRow>()
                            .expect("Could not downcast to 'FilterRow'");

                        obj.set_rule(rule);
                        obj.set_pattern(pattern);

                        model.items_changed(pos, 1, 1);
                    }
                ));
            }
        ));

        row.connect_closure("deleted", false, closure_local!(
            #[weak] imp,
            move |row: FilterRow| {
                imp.filter_model.get().unwrap().remove(row.index() as u32);
            }
        ));

        row.connect_closure("drop", false, closure_local!(
            #[weak] imp,
            move |row: FilterRow, drag_row: FilterRow| {
                let model = imp.filter_model.get().unwrap();

                let old_pos = drag_row.index() as u32;
                let new_pos = row.index() as u32;

                let obj = model.item(old_pos)
                    .and_downcast::<FilterRow>()
                    .expect("Could not downcast to 'FilterRow'");

                model.remove(old_pos);

                model.insert(new_pos, &obj);
            }
        ));

        row
    }

    //---------------------------------------
    // Filter dialog function
    //---------------------------------------
    fn filter_dialog<F>(&self, action: &str, filter: Option<(FilterRule, &str)>, f: F)
    where F: Fn(FilterRule, &str) + 'static {
        let dialog = FilterDialog::new(action, filter);

        dialog.connect_response(Some("add"), move |dialog, _| {
            f(dialog.rule(), &dialog.pattern());
        });

        dialog.present(Some(self));
    }
}
