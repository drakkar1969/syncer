use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use adw::prelude::*;
use glib::clone;

use crate::Application;
use crate::sidebar_row::SidebarRow;
use crate::profile_object::ProfileObject;
use crate::rsync_page::RsyncPage;
use crate::options_page::OptionsPage;

//------------------------------------------------------------------------------
// MODULE: AppWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RsyncUI/ui/window.ui")]
    pub struct AppWindow {
        #[template_child]
        pub(super) profile_add_button: TemplateChild<gtk::Button>,

        #[template_child]
        pub(super) sidebar_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) sidebar_selection: TemplateChild<gtk::SingleSelection>,
        #[template_child]
        pub(super) sidebar_model: TemplateChild<gio::ListStore>,
        #[template_child]
        pub(super) sidebar_factory: TemplateChild<gtk::SignalListItemFactory>,

        #[template_child]
        pub(super) content_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) content_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) rsync_page: TemplateChild<RsyncPage>,
        #[template_child]
        pub(super) options_page: TemplateChild<OptionsPage>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for AppWindow {
        const NAME: &'static str = "AppWindow";
        type Type = super::AppWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            ProfileObject::ensure_type();

            klass.bind_template();

            //---------------------------------------
            // Add class actions
            //---------------------------------------
            // New profile action
            klass.install_action("sidebar.new-profile", None, |window, _, _| {
                window.profile_name_dialog("Create New Profile", "Create", clone!(
                    #[weak] window,
                    move |name| {
                        let imp = window.imp();

                        imp.sidebar_model.append(&ProfileObject::new(name));

                        imp.sidebar_view.scroll_to(
                            imp.sidebar_model.n_items() - 1,
                            gtk::ListScrollFlags::FOCUS | gtk::ListScrollFlags::SELECT,
                            None
                        );

                        imp.sidebar_view.grab_focus();
                    }
                ));
            });

            // Rename profile action
            klass.install_action("sidebar.rename-profile", Some(glib::VariantTy::STRING), |window, _, parameter| {
                let imp = window.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                if let Some(pos) = imp.sidebar_model.iter::<ProfileObject>().flatten()
                    .position(|obj| obj.name() == name)
                {
                    window.profile_name_dialog("Rename Profile", "Rename", clone!(
                        #[weak] imp,
                        move |name| {
                            let obj = imp.sidebar_model.item(pos as u32)
                                .and_downcast::<ProfileObject>()
                                .expect("Could not downcast to 'ProfileObject'");

                            imp.sidebar_model.remove(pos as u32);

                            obj.set_name(name);

                            imp.sidebar_model.insert(pos as u32, &obj);
                        }
                    ));
                }
            });

            // Delete profile action
            klass.install_action("sidebar.delete-profile", Some(glib::VariantTy::STRING), |window, _, parameter| {
                let imp = window.imp();

                let name = parameter
                    .and_then(|param| param.get::<String>())
                    .expect("Could not get string from variant");

                let dialog = adw::AlertDialog::builder()
                    .heading("Delete Profile?")
                    .body("This wil permamenently delete the profile.")
                    .default_response("delete")
                    .build();

                dialog.add_responses(&[("cancel", "_Cancel"), ("delete", "_Delete")]);
                dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

                dialog.connect_response(Some("delete"), clone!(
                    #[weak] imp,
                    move |_, _| {
                        if let Some(pos) = imp.sidebar_model.iter::<ProfileObject>().flatten()
                            .position(|obj| obj.name() == name)
                        {
                            imp.sidebar_model.remove(pos as u32);
                        }
                    }
                ));

                dialog.present(Some(window));
            });

            // Content push options action
            klass.install_action("content.push-options", None, |window, _, _| {
                window.imp().content_navigation_view.push_by_tag("settings");
            });

            //---------------------------------------
            // Add class key bindings
            //---------------------------------------
            // New profile key binding
            klass.add_binding_action(gdk::Key::N, gdk::ModifierType::CONTROL_MASK, "sidebar.new-profile");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AppWindow {
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

    impl WidgetImpl for AppWindow {}
    impl WindowImpl for AppWindow {}
    impl ApplicationWindowImpl for AppWindow {}
    impl AdwApplicationWindowImpl for AppWindow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: AppWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct AppWindow(ObjectSubclass<imp::AppWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AppWindow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(app: &Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }

    //---------------------------------------
    // Profile name dialog helper function
    //---------------------------------------
    fn profile_name_dialog<F>(&self, heading: &str, label: &str, f: F)
    where F: Fn(&str) + 'static {
        let builder = gtk::Builder::from_resource("/com/github/RsyncUI/ui/builder/profile_name_dialog.ui");

        let dialog: adw::AlertDialog = builder.object("dialog")
            .expect("Could not get object from resource");

        dialog.set_heading(Some(heading));
        dialog.set_response_label("add", label);

        let entry: adw::EntryRow = builder.object("entry")
            .expect("Could not get object from resource");

        entry.connect_changed(clone!(
            #[weak] dialog,
            move |entry| {
                dialog.set_response_enabled("add", !entry.text().is_empty());
            }
        ));

        dialog.connect_response(Some("add"), move |_, _| {
            f(&entry.text());
        });

        dialog.present(Some(self));
    }

    //---------------------------------------
    // Setup signals
    //---------------------------------------
    fn setup_signals(&self) {
        let imp = self.imp();

        // Sidebar factory setup signal
        imp.sidebar_factory.connect_setup(|_, item| {
            item
                .downcast_ref::<gtk::ListItem>()
                .expect("Could not downcast to 'GtkListItem'")
                .set_child(Some(&SidebarRow::default()));
        });

        // Sidebar factory bind signal
        imp.sidebar_factory.connect_bind(|_, item| {
            let obj = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.item())
                .and_downcast::<ProfileObject>()
                .expect("Could not downcast to 'ProfileObject'");

            let row = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.child())
                .and_downcast::<SidebarRow>()
                .expect("Could not downcast to 'SidebarRow'");

            row.bind(&obj);
        });

        // Sidebar factory unbind signal
        imp.sidebar_factory.connect_unbind(|_, item| {
            let row = item
                .downcast_ref::<gtk::ListItem>()
                .and_then(|item| item.child())
                .and_downcast::<SidebarRow>()
                .expect("Could not downcast to 'SidebarRow'");

            row.unbind();
        });

        // Sidebar model items changed signal
        imp.sidebar_model.connect_items_changed(clone!(
            #[weak] imp,
            move |model, _, removed, added| {
                if removed != 0 || added != 0 {
                    imp.content_stack.set_visible_child_name(
                        if model.n_items() == 0 {
                            "status"
                        } else {
                            "profile"
                        }
                    );
                }
            }
        ));

        // Profile pane rsync running property notify signal
        // imp.rsync_page.connect_rsync_running_notify(clone!(
        //     #[weak] imp,
        //     move |pane| {
        //         let enabled = !pane.rsync_running();

        //         imp.profile_add_button.set_sensitive(enabled);
        //         imp.sidebar_view.set_sensitive(enabled);
        //     }
        // ));
    }

    //---------------------------------------
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        imp.sidebar_model.append(&ProfileObject::new("TEST"));

        // Bind sidebar selected item to profile pane
        imp.sidebar_selection.bind_property("selected-item", &imp.rsync_page.get(), "profile")
            .sync_create()
            .build();
    }
}
