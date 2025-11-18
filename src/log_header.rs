use gtk::subclass::prelude::*;
use gtk::glib;

use crate::{
    log_window::LogObject,
    rsync_process::RsyncMsgType
};

//------------------------------------------------------------------------------
// MODULE: LogHeader
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/log_header.ui")]
    pub struct LogHeader {
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for LogHeader {
        const NAME: &'static str = "LogHeader";
        type Type = super::LogHeader;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LogHeader {}
    impl WidgetImpl for LogHeader {}
    impl BoxImpl for LogHeader {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: LogHeader
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct LogHeader(ObjectSubclass<imp::LogHeader>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl LogHeader {
    //---------------------------------------
    // Bind function
    //---------------------------------------
    pub fn bind(&self, obj: &LogObject) {
        self.imp().label.set_label(
            match obj.tag {
                RsyncMsgType::Error => "Errors",
                RsyncMsgType::Stat => "Statistics",
                _ => "Log Messages",
            }

        );
    }
}

impl Default for LogHeader {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
