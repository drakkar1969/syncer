use std::cell::RefCell;

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
    #[derive(Default, gtk::CompositeTemplate, glib::Properties)]
    #[properties(wrapper_type = super::SidebarRow)]
    #[template(resource = "/com/github/Syncer/ui/sidebar_row.ui")]
    pub struct SidebarRow {
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) popover: TemplateChild<gtk::PopoverMenu>,

        #[property(get, set, nullable)]
        profile: RefCell<Option<ProfileObject>>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for SidebarRow {
        const NAME: &'static str = "SidebarRow";
        type Type = super::SidebarRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for SidebarRow {}

    impl WidgetImpl for SidebarRow {}
    impl ListBoxRowImpl for SidebarRow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: SidebarRow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct SidebarRow(ObjectSubclass<imp::SidebarRow>)
        @extends gtk::ListBoxRow, gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl SidebarRow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(profile: &ProfileObject) -> Self {
        let row: Self = glib::Object::builder()
            .property("profile", profile)
            .build();

        let imp = row.imp();

        // Bind profile to label
        profile.bind_property("name", &imp.label.get(), "label")
            .sync_create()
            .build();

        // Bind profile to popover menu model
        profile.bind_property("name", &imp.popover.get(), "menu-model")
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
            .build();

        // Add popup gesture
        let popup_gesture = gtk::GestureClick::builder()
            .button(gdk::BUTTON_SECONDARY)
            .build();

        popup_gesture.connect_pressed(clone!(
            #[weak] imp,
            move |_, _, x, y| {
                let rect = gdk::Rectangle::new(x as i32, y as i32, 0, 0);

                imp.popover.set_pointing_to(Some(&rect));
                imp.popover.popup();
            }
        ));

        row.add_controller(popup_gesture);

        row
    }
}
