use std::cell::OnceCell;

use gtk::{glib, gio, gdk};
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
        pub(super) popover: TemplateChild<gtk::PopoverMenu>,

        pub(super) binding: OnceCell<glib::Binding>,
        pub(super) popup_gesture: OnceCell<gtk::GestureClick>,
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

    impl ObjectImpl for SidebarRow {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

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
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let popup_gesture = gtk::GestureClick::builder()
            .button(gdk::BUTTON_SECONDARY)
            .build();

        self.add_controller(popup_gesture.clone());

        self.imp().popup_gesture.set(popup_gesture).unwrap();
    }

    //---------------------------------------
    // Public bind function
    //---------------------------------------
    pub fn bind(&self, obj: &ProfileObject) {
        let imp = self.imp();

        // Bind object to label
        let binding = obj.bind_property("name", &imp.label.get(), "label")
            .sync_create()
            .build();

        imp.binding.set(binding).unwrap();

        // Create menu model
        let name = obj.name();

        let menu = gio::Menu::new();

        let section = gio::Menu::new();

        section.append_item(&gio::MenuItem::new(Some("Rename"),
            Some(&format!("sidebar.rename-profile::{name}"))));

        section.append_item(&gio::MenuItem::new(Some("Delete"),
            Some(&format!("sidebar.delete-profile::{name}"))));

        menu.append_section(None, &section);

        let section = gio::Menu::new();

        section.append_item(&gio::MenuItem::new(Some("Duplicate"),
            Some(&format!("sidebar.duplicate-profile::{name}"))));

        menu.append_section(None, &section);

        // Set popover menu model
        let popover = imp.popover.get();
        popover.set_menu_model(Some(&menu));

        // Connect popup gesture pressed signal
        let popup_gesture = imp.popup_gesture.get().unwrap();

        popup_gesture.connect_pressed(
            move |_, _, x, y| {
                let rect = gdk::Rectangle::new(x as i32, y as i32, 0, 0);

                popover.set_pointing_to(Some(&rect));
                popover.popup();
            }
        );
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
