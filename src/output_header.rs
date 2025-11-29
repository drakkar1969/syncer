use gtk::subclass::prelude::*;
use gtk::glib;

use crate::{
    output_window::OutputObject,
    rsync_process::RsyncMsgType
};

//------------------------------------------------------------------------------
// MODULE: OutputHeader
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/output_header.ui")]
    pub struct OutputHeader {
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for OutputHeader {
        const NAME: &'static str = "OutputHeader";
        type Type = super::OutputHeader;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for OutputHeader {}
    impl WidgetImpl for OutputHeader {}
    impl BoxImpl for OutputHeader {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: OutputHeader
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct OutputHeader(ObjectSubclass<imp::OutputHeader>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl OutputHeader {
    //---------------------------------------
    // Bind function
    //---------------------------------------
    pub fn bind(&self, obj: &OutputObject) {
        self.imp().label.set_label(
            match obj.tag {
                RsyncMsgType::Error => "Errors",
                RsyncMsgType::Stat => "Statistics",
                _ => "Output",
            }
        );
    }
}

impl Default for OutputHeader {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
