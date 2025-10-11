use std::cell::OnceCell;

use gtk::{glib, gio, gdk};
use adw::subclass::prelude::*;
use gtk::prelude::*;
use glib::clone;

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

        pub(super) bindings: OnceCell<Vec<glib::Binding>>,
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
        let imp = self.imp();

        let popup_gesture = gtk::GestureClick::builder()
            .button(gdk::BUTTON_SECONDARY)
            .build();

        popup_gesture.connect_pressed(clone!(
            #[weak] imp,
            move |_, _, x, y| {
                let popover = imp.popover.get();
                
                let rect = gdk::Rectangle::new(x as i32, y as i32, 0, 0);

                popover.set_pointing_to(Some(&rect));
                popover.popup();
            }
        ));

        self.add_controller(popup_gesture);
    }

    //---------------------------------------
    // Public bind function
    //---------------------------------------
    pub fn bind(&self, obj: &ProfileObject) {
        let imp = self.imp();

        let bindings = vec![
            // Bind object to label
            obj.bind_property("name", &imp.label.get(), "label")
                .sync_create()
                .build(),

            // Bind object to popover menu model
            obj.bind_property("name", &imp.popover.get(), "menu-model")
                .transform_to(|_, name: String| {
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

                    let section = gio::Menu::new();

                    section.append_item(&gio::MenuItem::new(Some("Reset to Default"),
                        Some(&format!("sidebar.reset-profile::{name}"))));

                    menu.append_section(None, &section);

                    Some(menu)
                })
                .sync_create()
                .build()
        ];

        imp.bindings.set(bindings).unwrap();
    }

    //---------------------------------------
    // Public unbind function
    //---------------------------------------
    pub fn unbind(&self) {
        if let Some(bindings) = self.imp().bindings.get() {
            for binding in bindings {
                binding.unbind();
            }
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
