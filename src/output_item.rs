use gtk::{prelude::WidgetExt, subclass::prelude::*, glib};

use crate::{
    output_window::OutputObject,
    rsync_page::RsyncMsgType
};

//------------------------------------------------------------------------------
// MODULE: OutputItem
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/output_item.ui")]
    pub struct OutputItem {
        #[template_child]
        pub(super) image: TemplateChild<gtk::Image>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for OutputItem {
        const NAME: &'static str = "OutputItem";
        type Type = super::OutputItem;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for OutputItem {}
    impl WidgetImpl for OutputItem {}
    impl BoxImpl for OutputItem {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: OutputItem
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct OutputItem(ObjectSubclass<imp::OutputItem>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl OutputItem {
    //---------------------------------------
    // Bind function
    //---------------------------------------
    pub fn bind(&self, obj: &OutputObject) {
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
                let msg_lower = msg.to_ascii_lowercase();

                if msg_lower.starts_with("deleting") {
                    Some("user-trash-symbolic")
                } else if msg_lower.starts_with("skipping") {
                    Some("edit-undo-symbolic")
                } else {
                    Some("info-outline-symbolic")
                }
            }
            RsyncMsgType::File => Some("stats-file-symbolic"),
            RsyncMsgType::Directory => Some("stats-dir-symbolic"),
            RsyncMsgType::Link => Some("stats-link-symbolic"),
            RsyncMsgType::Device | RsyncMsgType::Special => Some("stats-special-symbolic"),
            RsyncMsgType::None => None
        });
    }
}

impl Default for OutputItem {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
