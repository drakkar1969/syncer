use std::cell::{Cell, RefCell};

use adw::{prelude::*, subclass::prelude::*};
use gtk::glib;
use glib::clone;

use crate::filters_page::FilterRule;

//------------------------------------------------------------------------------
// MODULE: FilterDialog
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::FilterDialog)]
    #[template(resource = "/com/github/Syncer/ui/filter_dialog.ui")]
    pub struct FilterDialog {
        #[template_child]
        pub(super) rule_combo: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) pattern_entry: TemplateChild<adw::EntryRow>,

        #[property(get, set, builder(FilterRule::default()))]
        rule: Cell<FilterRule>,
        #[property(get, set)]
        pattern: RefCell<String>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for FilterDialog {
        const NAME: &'static str = "FilterDialog";
        type Type = super::FilterDialog;
        type ParentType = adw::AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for FilterDialog {
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

    impl WidgetImpl for FilterDialog {}
    impl AdwDialogImpl for FilterDialog {}
    impl AdwAlertDialogImpl for FilterDialog {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: FilterDialog
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct FilterDialog(ObjectSubclass<imp::FilterDialog>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl FilterDialog {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(action: &str, filter: Option<(FilterRule, &str)>) -> Self {
        let (rule, pattern) = filter.unwrap_or_default();

        let dialog: Self = glib::Object::builder()
            .property("heading", format!("{action} Filter Rule"))
            .property("rule", rule)
            .property("pattern", pattern)
            .build();

        dialog.set_response_label("add", action);

        dialog
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        self.imp().pattern_entry.connect_changed(clone!(
            #[weak(rename_to = dialog)] self,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        // Bind properties to widget
        self.bind_property("rule", &imp.rule_combo.get(), "selected")
            .sync_create()
            .bidirectional()
            .build();

        self.bind_property("pattern", &imp.pattern_entry.get(), "text")
            .sync_create()
            .bidirectional()
            .build();
    }
}
