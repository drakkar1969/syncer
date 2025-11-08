use gtk::prelude::WidgetExt;
use adw::subclass::prelude::*;
use gtk::glib;

//------------------------------------------------------------------------------
// CONST Variables
//------------------------------------------------------------------------------
pub const STATS_TAG: &str = "::STATS::";

//------------------------------------------------------------------------------
// MODULE: LogItem
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/log_item.ui")]
    pub struct LogItem {
        #[template_child]
        pub(super) box_: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for LogItem {
        const NAME: &'static str = "LogItem";
        type Type = super::LogItem;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LogItem {}
    impl WidgetImpl for LogItem {}
    impl BinImpl for LogItem {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: LogItem
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct LogItem(ObjectSubclass<imp::LogItem>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl LogItem {
    //---------------------------------------
    // Bind function
    //---------------------------------------
    pub fn bind(&self, text: &str) {
        let imp = self.imp();

        if text.starts_with(STATS_TAG) {
            imp.box_.set_css_classes(&["success"]);

            imp.label.set_label(&text.replace(STATS_TAG, ""));

            imp.image.set_visible(false);
        } else {
            imp.label.set_label(text);

            imp.image.set_visible(true);
            imp.image.set_icon_name(None);

            if text.starts_with("cannot") {
                imp.box_.set_css_classes(&["error"]);

                imp.image.set_icon_name(Some("rsync-error-symbolic"));
            } else if text.starts_with("skipping") {
                imp.box_.set_css_classes(&["warning"]);

                imp.image.set_icon_name(Some("stats-skipped-symbolic"));
            } else if text.starts_with("deleting") {
                imp.box_.set_css_classes(&["warning"]);

                imp.image.set_icon_name(Some("stats-deleted-symbolic"));
            } else if text.contains("->") {
                imp.box_.set_css_classes(&["accent"]);

                imp.image.set_icon_name(Some("stats-link-symbolic"));
            } else {
                imp.box_.set_css_classes(&[]);

                if text.ends_with('/') {
                    imp.image.set_icon_name(Some("stats-dir-symbolic"));
                } else if !text.is_empty() {
                    imp.image.set_icon_name(Some("stats-file-symbolic"));
                }
            }
        }
    }
}

impl Default for LogItem {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
