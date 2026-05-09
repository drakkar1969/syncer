use gtk::{prelude::WidgetExt, subclass::prelude::*, glib};

use crate::{
    output_page::OutputObject,
    rsync::RsyncMsg
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

        imp.image.set_icon_name(obj.icon());
        imp.label.set_label(obj.msg());

        self.set_css_classes(
            if obj.tag() == RsyncMsg::Error {
                &["error"]
            } else {
                &[]
            }
        );
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
