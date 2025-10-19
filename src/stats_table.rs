use gtk::glib::{self, object::ObjectExt};
use adw::subclass::prelude::*;

use crate::rsync::Stats;

//------------------------------------------------------------------------------
// MODULE: StatsTable
//------------------------------------------------------------------------------
mod imp {
    use super::*;

    //---------------------------------------
    // Private structure
    //---------------------------------------
    #[derive(Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/github/RsyncUI/ui/stats_table.ui")]
    pub struct StatsTable {
        #[template_child]
        pub(super) transfer_total_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) transfer_files_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) transfer_files_label: TemplateChild<gtk::Label>,

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
        pub(super) created_total_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_files_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) created_dirs_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) created_links_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) created_specials_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) created_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_dirs_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_links_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) created_specials_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub(super) deleted_total_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) deleted_files_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) deleted_dirs_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) deleted_links_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) deleted_specials_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) deleted_files_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) deleted_dirs_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) deleted_links_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) deleted_specials_label: TemplateChild<gtk::Label>,
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

    impl ObjectImpl for StatsTable {
        //---------------------------------------
        // Constructor
        //---------------------------------------
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.setup_widgets();
        }
    }

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
    // Setup widgets
    //---------------------------------------
    fn setup_widgets(&self) {
        let imp = self.imp();

        let widgets = [
            (&imp.transfer_files_box, &imp.transfer_files_label),

            (&imp.source_files_box, &imp.source_files_label),
            (&imp.source_dirs_box, &imp.source_dirs_label),
            (&imp.source_links_box, &imp.source_links_label),
            (&imp.source_specials_box, &imp.source_specials_label),

            (&imp.created_files_box, &imp.created_files_label),
            (&imp.created_dirs_box, &imp.created_dirs_label),
            (&imp.created_links_box, &imp.created_links_label),
            (&imp.created_specials_box, &imp.created_specials_label),

            (&imp.deleted_files_box, &imp.deleted_files_label),
            (&imp.deleted_dirs_box, &imp.deleted_dirs_label),
            (&imp.deleted_links_box, &imp.deleted_links_label),
            (&imp.deleted_specials_box, &imp.deleted_specials_label),
        ];

        for (box_, label) in widgets {
            label.bind_property("label", &box_.get(), "visible")
                .transform_to(|_, label: &str| Some(!label.is_empty() && label != "0"))
                .sync_create()
                .build();
        }
    }

    //---------------------------------------
    // Fill function
    //---------------------------------------
    pub fn fill(&self, stats: &Stats) {
        let imp = self.imp();

        imp.transfer_total_label.set_label(&stats.transferred);
        imp.transfer_files_label.set_label(&stats.transferred);

        imp.source_total_label.set_label(&stats.source.total);
        imp.source_files_label.set_label(&stats.source.files);
        imp.source_dirs_label.set_label(&stats.source.dirs);
        imp.source_links_label.set_label(&stats.source.links);
        imp.source_specials_label.set_label(&stats.source.specials);

        imp.created_total_label.set_label(&stats.created.total);
        imp.created_files_label.set_label(&stats.created.files);
        imp.created_dirs_label.set_label(&stats.created.dirs);
        imp.created_links_label.set_label(&stats.created.links);
        imp.created_specials_label.set_label(&stats.created.specials);

        imp.deleted_total_label.set_label(&stats.deleted.total);
        imp.deleted_files_label.set_label(&stats.deleted.files);
        imp.deleted_dirs_label.set_label(&stats.deleted.dirs);
        imp.deleted_links_label.set_label(&stats.deleted.links);
        imp.deleted_specials_label.set_label(&stats.deleted.specials);
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
