use gtk::subclass::prelude::*;
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
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LogItem {}
    impl WidgetImpl for LogItem {}
    impl BoxImpl for LogItem {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: LogItem
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct LogItem(ObjectSubclass<imp::LogItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl LogItem {
    //---------------------------------------
    // Bind function
    //---------------------------------------
    pub fn bind(&self, obj: &LogObject) {
        let imp = self.imp();

        let msg = &obj.msg;

        imp.label.set_label(msg);

        self.set_css_classes(
            if obj.tag == RsyncMsgType::Error {
                &["error"]
            } else {
                &[]
            }
        );

        imp.image.set_icon_name(match obj.tag {
            RsyncMsgType::Error => Some("rsync-error-symbolic"),
            RsyncMsgType::Stat => Some("stats-symbolic"),
            RsyncMsgType::Info => {
                if msg.starts_with("deleting") {
                    Some("stats-deleted-symbolic")
                } else if msg.contains("non-regular") {
                    Some("stats-skipped-symbolic")
                } else {
                    Some("stats-info-symbolic")
                }
            }
            RsyncMsgType::f => Some("stats-file-symbolic"),
            RsyncMsgType::d => Some("stats-dir-symbolic"),
            RsyncMsgType::L => Some("stats-link-symbolic"),
            RsyncMsgType::D | RsyncMsgType::S => Some("stats-special-symbolic"),
            RsyncMsgType::None => None
        });
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
