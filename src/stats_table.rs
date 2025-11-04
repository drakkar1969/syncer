use gtk::glib::{self, object::ObjectExt};
use adw::subclass::prelude::*;

use crate::rsync_process::Stats;

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

        let dest_created = stats.created.total
            .replace([',', '.'], "")
            .parse::<u64>()
            .unwrap_or_default();

        let dest_created_files = stats.created.files
            .replace([',', '.'], "")
            .parse::<u64>()
            .unwrap_or_default();

        let dest_transferred = stats.transferred
            .replace([',', '.'], "")
            .parse::<u64>()
            .unwrap_or_default();

        let dest_total = if dest_created > dest_transferred {
            &stats.created.total
        } else {
            &stats.transferred
        };

        let dest_files = if dest_created_files > dest_transferred {
            &stats.created.files
        } else {
            &stats.transferred
        };

        imp.source_total_label.set_label(&stats.source.total);
        imp.source_files_label.set_label(&stats.source.files);
        imp.source_dirs_label.set_label(&stats.source.dirs);
        imp.source_links_label.set_label(&stats.source.links);
        imp.source_specials_label.set_label(&stats.source.specials);

        imp.destination_total_label.set_label(dest_total);
        imp.destination_files_label.set_label(dest_files);
        imp.destination_dirs_label.set_label(&stats.created.dirs);
        imp.destination_links_label.set_label(&stats.created.links);
        imp.destination_specials_label.set_label(&stats.created.specials);
        imp.destination_deleted_label.set_label(&stats.deleted.total);
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
