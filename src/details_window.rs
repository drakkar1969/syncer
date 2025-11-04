use gtk::{gio, glib, gdk};
use adw::subclass::prelude::*;
use gtk::prelude::*;
use glib::clone;

//------------------------------------------------------------------------------
// MODULE: DetailsWindow
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RsyncUI/ui/details_window.ui")]
    pub struct DetailsWindow {
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) model: TemplateChild<gio::ListStore>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for DetailsWindow {
        const NAME: &'static str = "DetailsWindow";
        type Type = super::DetailsWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            //---------------------------------------
            // Close window key binding
            //---------------------------------------
            klass.add_binding_action(gdk::Key::Escape, gdk::ModifierType::NO_MODIFIER_MASK, "window.close");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DetailsWindow {}
    impl WidgetImpl for DetailsWindow {}
    impl WindowImpl for DetailsWindow {}
    impl AdwWindowImpl for DetailsWindow {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: DetailsWindow
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct DetailsWindow(ObjectSubclass<imp::DetailsWindow>)
    @extends adw::Window, gtk::Window, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl DetailsWindow {
    //---------------------------------------
    // New function
    //---------------------------------------
    pub fn new(parent: &impl IsA<gtk::Window>) -> Self {
        glib::Object::builder()
            .property("transient-for", parent)
            .build()
    }

    //---------------------------------------
    // Style to color function
    //---------------------------------------
    fn style_to_color(style: &str) -> String {
        let f32_to_u8 = |color: f32| -> u8 {
            (color * f32::from(u8::MAX)) as u8
        };

        let label = gtk::Label::builder()
            .css_classes([style])
            .build();

        let color = label.color();

        format!("#{:02X}{:02X}{:02X}{:02X}",
            f32_to_u8(color.red()),
            f32_to_u8(color.green()),
            f32_to_u8(color.blue()),
            f32_to_u8(color.alpha()))
    }

    //---------------------------------------
    // Display function
    //---------------------------------------
    pub fn display(&self, details: &[String]) {
        self.present();

        let details = details.to_vec();

        let accent_color = Self::style_to_color("accent");
        let warning_color = Self::style_to_color("warning");
        let success_color = Self::style_to_color("success");

        glib::spawn_future_local(clone!(
            #[weak(rename_to = window)] self,
            async move {
                let imp = window.imp();

                let details: Vec<String> = gio::spawn_blocking(clone!(
                    move || {
                        let format_span = |s: &str, color: &str| -> String {
                            format!("<span foreground=\"{}\">{}</span>",
                                color,
                                glib::markup_escape_text(s))
                        };

                        let mut stats = false;

                        details.iter()
                            .map(|s| {
                                if s.contains("->") {
                                    format_span(s, &accent_color)
                                } else if s.contains("skipping") || s.contains("deleting") {
                                    format_span(s, &warning_color)
                                } else if s.contains(":: STATISTICS ::") || stats {
                                    stats = true;

                                    format_span(s, &success_color)
                                } else {
                                    glib::markup_escape_text(s).to_string()
                                }
                            })
                            .collect()
                    }
                ))
                .await
                .expect("Failed to complete task");

                let objects: Vec<gtk::StringObject> = details.iter()
                    .map(|s| gtk::StringObject::new(s))
                    .collect();

                imp.model.splice(0, 0, &objects);

                imp.stack.set_visible_child_name("details");
            }
        ));

    }
}
