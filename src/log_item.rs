use adw::subclass::prelude::*;
use gtk::prelude::WidgetExt;
use gtk::glib;

use crate::{
    log_window::LogObject,
    rsync_process::RsyncMsgType
};

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
    pub fn bind(&self, obj: &LogObject) {
        let imp = self.imp();

        let msg = &obj.msg;

        imp.box_.set_css_classes(&[""]);
        imp.image.set_icon_name(None);
        imp.label.set_label(msg);

        match obj.tag {
            RsyncMsgType::Error => {
                imp.box_.set_css_classes(&["error"]);
                imp.image.set_icon_name(Some("rsync-error-symbolic"));
            }
            RsyncMsgType::Stat => {
                imp.image.set_icon_name(Some("stats-symbolic"));
            }
            RsyncMsgType::Info => {
                imp.box_.set_css_classes(&["warning"]);

                if msg.starts_with("deleting") {
                    imp.image.set_icon_name(Some("stats-deleted-symbolic"));
                } else if msg.contains("non-regular") {
                    imp.image.set_icon_name(Some("stats-skipped-symbolic"));
                } else {
                    imp.image.set_icon_name(Some("stats-info-symbolic"));
                }
            }
            RsyncMsgType::f => imp.image.set_icon_name(Some("stats-file-symbolic")),
            RsyncMsgType::d => imp.image.set_icon_name(Some("stats-dir-symbolic")),
            RsyncMsgType::L => imp.image.set_icon_name(Some("stats-link-symbolic")),
            RsyncMsgType::D | RsyncMsgType::S => imp.image.set_icon_name(Some("stats-special-symbolic")),
            RsyncMsgType::None => {}
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
