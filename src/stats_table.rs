use gtk::subclass::prelude::*;
use gtk::glib;

use crate::rsync_process::RsyncStats;

//------------------------------------------------------------------------------
// MODULE: StatsTable
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/Syncer/ui/stats_table.ui")]
    pub struct StatsTable {
        #[template_child]
        pub(super) source_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) deleted_files_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub(super) source_size_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transferred_size_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) sent_bytes_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) received_bytes_label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for StatsTable {
        const NAME: &'static str = "StatsTable";
        type Type = super::StatsTable;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StatsTable {}
    impl WidgetImpl for StatsTable {}
    impl BoxImpl for StatsTable {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: StatsTable
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct StatsTable(ObjectSubclass<imp::StatsTable>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl StatsTable {
    //---------------------------------------
    // Fill function
    //---------------------------------------
    pub fn fill(&self, stats: &RsyncStats) {
        let imp = self.imp();

        imp.source_files_label.set_label(&stats.source_files);
        imp.created_files_label.set_label(&stats.created_files);
        imp.transferred_files_label.set_label(&stats.transferred_files);
        imp.deleted_files_label.set_label(&stats.deleted_files);

        imp.source_size_label.set_label(&stats.source_size);
        imp.transferred_size_label.set_label(&stats.transferred_size);
        imp.sent_bytes_label.set_label(&stats.sent_bytes);
        imp.received_bytes_label.set_label(&stats.received_bytes);
    }
}
