use gtk::prelude::WidgetExt;
use adw::subclass::prelude::*;
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
        pub(super) source_total_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) source_files_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) source_dirs_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) source_links_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) source_specials_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) source_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) source_dirs_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) source_links_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) source_specials_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub(super) destination_none_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_total_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_files_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_dirs_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_links_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_specials_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_deleted_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) destination_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_dirs_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_links_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_specials_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) destination_deleted_label: TemplateChild<gtk::Label>,
    }

    //---------------------------------------
    // Subclass
    //---------------------------------------
    #[glib::object_subclass]
    impl ObjectSubclass for StatsTable {
        const NAME: &'static str = "StatsTable";
        type Type = super::StatsTable;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for StatsTable {}
    impl WidgetImpl for StatsTable {}
    impl BinImpl for StatsTable {}
}

//------------------------------------------------------------------------------
// IMPLEMENTATION: StatsTable
//------------------------------------------------------------------------------
glib::wrapper! {
    pub struct StatsTable(ObjectSubclass<imp::StatsTable>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl StatsTable {
    //---------------------------------------
    // Fill function
    //---------------------------------------
    pub fn fill(&self, stats: &RsyncStats) {
        let imp = self.imp();

        imp.source_total_label.set_label(&stats.source_total);
        imp.source_files_label.set_label(&stats.source_files);
        imp.source_dirs_label.set_label(&stats.source_dirs);
        imp.source_links_label.set_label(&stats.source_links);
        imp.source_specials_label.set_label(&stats.source_specials);

        imp.destination_total_label.set_label(&stats.destination_total);
        imp.destination_files_label.set_label(&stats.destination_files);
        imp.destination_dirs_label.set_label(&stats.destination_dirs);
        imp.destination_links_label.set_label(&stats.destination_links);
        imp.destination_specials_label.set_label(&stats.destination_specials);
        imp.destination_deleted_label.set_label(&stats.destination_deleted);

        imp.destination_none_label.set_visible(stats.destination_total == "0");

        let widgets = [
            (&imp.source_files_box, &imp.source_files_label),
            (&imp.source_dirs_box, &imp.source_dirs_label),
            (&imp.source_links_box, &imp.source_links_label),
            (&imp.source_specials_box, &imp.source_specials_label),

            (&imp.destination_files_box, &imp.destination_files_label),
            (&imp.destination_dirs_box, &imp.destination_dirs_label),
            (&imp.destination_links_box, &imp.destination_links_label),
            (&imp.destination_specials_box, &imp.destination_specials_label),
            (&imp.destination_deleted_box, &imp.destination_deleted_label),
        ];

        for (box_, label) in widgets {
            box_.set_visible(label.label() != "0");
        }
    }
}

impl Default for StatsTable {
    //---------------------------------------
    // Default constructor
    //---------------------------------------
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
