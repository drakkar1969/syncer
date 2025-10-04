use std::cell::OnceCell;

use gtk::{glib, gio};
use adw::subclass::prelude::*;
use gtk::prelude::*;

use crate::profile_object::ProfileObject;

//------------------------------------------------------------------------------
// MODULE: SidebarRow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RsyncUI/ui/sidebar_row.ui")]
    pub struct SidebarRow {
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,

        pub(super) binding: OnceCell<glib::Binding>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for SidebarRow {
        const NAME: &'static str = "SidebarRow";
        type Type = super::SidebarRow;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SidebarRow {}

    impl WidgetImpl for SidebarRow {}
    impl BinImpl for SidebarRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: SidebarRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct SidebarRow(ObjectSubclass<imp::SidebarRow>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl SidebarRow {
    //---------------------------------------
    // Public bind function
    //---------------------------------------
    pub fn bind(&self, obj: &ProfileObject) {
        let imp = self.imp();

        let binding = obj.bind_property("name", &imp.label.get(), "label")
            .sync_create()
            .build();

        imp.binding.set(binding).unwrap();

        let name = obj.name();

        let menu_model = gio::Menu::new();

        let section_model = gio::Menu::new();

        section_model.append_item(
            &gio::MenuItem::new(Some("Rename"), Some(&format!("sidebar.rename-profile::{name}")))
        );

        section_model.append_item(
            &gio::MenuItem::new(Some("Delete"), Some(&format!("sidebar.delete-profile::{name}")))
        );

        menu_model.append_section(None, &section_model);

        let section_model = gio::Menu::new();

        section_model.append_item(
            &gio::MenuItem::new(Some("Duplicate"), Some(&format!("sidebar.duplicate-profile::{name}")))
        );

        menu_model.append_section(None, &section_model);

        imp.menu_button.set_menu_model(Some(&menu_model));
    }

    //---------------------------------------
    // Public unbind function
    //---------------------------------------
    pub fn unbind(&self) {
        if let Some(binding) = self.imp().binding.get() {
            binding.unbind();
        }
    }
}

impl Default for SidebarRow {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
